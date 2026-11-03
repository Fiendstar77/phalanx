import json
from pathlib import Path
from tempfile import TemporaryDirectory
from types import SimpleNamespace
import unittest
from contextlib import redirect_stdout
from io import StringIO
from .cognition import load, run


class CognitionTests(unittest.TestCase):
    def fixture(self, root):
        root = Path(root)
        (root/'manifest.json').write_text(json.dumps({'fixture':True}))
        for seed in range(3):
            path = root/'studies'/f'battle-{seed}'
            path.mkdir(parents=True)
            (path/'status.json').write_text(json.dumps({'complete':True}))
            rows = [{'event':'start','telemetry_schema_version':2,'initial_tick':0,
                     'stream_sequence':0,'simulation_build':'test-build','seed':seed,
                     'feature_names':['health','distance'],'actions':['ADVANCE','ATTACK'],
                     'initial_units':[{'id':0,'fighter_id':f'fighter-{seed}'}]}]
            for i in range(6):
                rows.append({'event':'decision','phase':'decision_before_movement','unit':0,
                             'tick':i*5,'features':[seed*100+i,i+1],
                             'action':'ADVANCE' if i%2 == 0 else 'ATTACK','action_mask':[True,True]})
                rows.append({'event':'combat_frame','truth_only':True,'features':[99999,99999]})
            rows.append({'event':'end','tick':30})
            for i,row in enumerate(rows):row['stream_sequence']=i
            (path/'decisions.jsonl').write_text(''.join(json.dumps(r)+'\n' for r in rows))
        return root

    def edit(self, root, seed, change):
        path=root/'studies'/f'battle-{seed}'/'decisions.jsonl'
        rows=[json.loads(l) for l in path.read_text().splitlines()]
        change(rows)
        path.write_text(''.join(json.dumps(r)+'\n' for r in rows))

    def test_only_strictly_earlier_actor_observations_and_actions_are_inputs(self):
        with TemporaryDirectory() as temp:
            root=self.fixture(temp)
            before,_,_=load(root,1,2)
            self.assertEqual(before['train'][0][0],[0,1,1.,0.,1,2,0.,1.])
            # Alter the target's current observation. Its forecast input must stay
            # unchanged, while a later forecast may legitimately see that history.
            self.edit(root,0,lambda rows:rows[5]['features'].__setitem__(0,5000))
            after,_,_=load(root,1,2)
            self.assertEqual(before['train'][0],after['train'][0])
            self.assertNotEqual(before['train'][1],after['train'][1])

    def test_incomplete_noncausal_ineligible_and_mixed_sources_are_rejected(self):
        changes=[lambda r:r[3].__setitem__('tick',0),
                 lambda r:r[3].__setitem__('action_mask',[True,False]),
                 lambda r:r[3].__setitem__('stream_sequence',900),
                 lambda r:r[0].__setitem__('simulation_build','different'),
                 lambda r:r[0]['initial_units'][0].__setitem__('fighter_id','fighter-0'),
                 lambda r:r.pop()]
        for change in changes:
            with self.subTest(change=change),TemporaryDirectory() as temp:
                root=self.fixture(temp)
                self.edit(root,1,change)
                with self.assertRaises(ValueError):load(root,1,2)
        with TemporaryDirectory() as temp:
            root=self.fixture(temp)
            (root/'studies/battle-0/status.json').write_text('{"complete":false}')
            with self.assertRaises(ValueError):load(root,1,2)

    def test_pytorch_checkpoint_uses_training_normalization_and_stays_research_only(self):
        import torch
        with TemporaryDirectory() as temp:
            root=Path(temp)
            data=self.fixture(root)
            groups,_,_=load(data,1,2)
            args=SimpleNamespace(data=str(data),output=str(root/'model'),train_end=1,val_end=2,
                                 history=2,epochs=1,seed=23)
            with redirect_stdout(StringIO()):
                result=run(args)
            saved=torch.load(root/'model/forecast.pt',weights_only=True)
            expected=torch.tensor([r[0] for r in groups['train']],dtype=torch.float32).mean(0)
            self.assertTrue(torch.equal(saved['mean'],expected))
            self.assertFalse(result['admission']['controls_live_game'])
            self.assertFalse(result['admission']['opponent_visible_predictor'])
            self.assertEqual(result['provenance']['seeds'],{'train':[0],'validation':[1],'test':[2]})
            self.assertTrue((root/'model/report.md').is_file())


if __name__ == '__main__':unittest.main()
