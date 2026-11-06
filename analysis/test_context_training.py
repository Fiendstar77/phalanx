import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from .prepare_context_training import prepare


class ContextTraining(unittest.TestCase):
    def fixture(self, root, complete=True):
        source=root/'source';study=source/'studies'/'one';study.mkdir(parents=True)
        (source/'manifest.json').write_text(json.dumps({'source_sha256':'fixture-build'}))
        (study/'status.json').write_text(json.dumps({'complete':complete}))
        rows=[{'event':'start','controller':'Utility','seed':4,'feature_names':['memory'],'actions':['HOLD']},
              {'event':'decision','comparison':{'controller':'Utility'},'features':[0.4],'action':'HOLD','action_mask':[True]},
              {'event':'decision','comparison':{'controller':'Opportunity'},'features':[0.8],'action':'HOLD','action_mask':[True]},
              {'event':'combat_frame','truth_only':True,'actors':[{'hidden_enemy':123}]}]
        path=study/'decisions.jsonl';path.write_text(''.join(json.dumps(r)+'\n' for r in rows))
        return source,path

    def test_view_keeps_utility_observations_excludes_composed_labels_and_preserves_provenance(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);source,path=self.fixture(root)
            result=prepare(source,root/'view')
            rows=[json.loads(r) for r in (root/'view'/'000.jsonl').read_text().splitlines()]
            self.assertEqual(result['kept_utility_decisions'],1)
            self.assertEqual(result['excluded_composed_decisions'],1)
            self.assertEqual(result['sources'][0]['sha256'],hashlib.sha256(path.read_bytes()).hexdigest())
            self.assertEqual([r['event'] for r in rows],['start','decision'])
            self.assertEqual(rows[1]['features'],[0.4])

    def test_incomplete_source_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            root=Path(directory);source,_=self.fixture(root,False)
            with self.assertRaisesRegex(ValueError,'completed'):
                prepare(source,root/'view')
