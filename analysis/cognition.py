"""Causal next-decision forecasting from completed PHALANX studies.

Only earlier decision-maker observations enter the model. World-truth frames,
future masks, utility scores, rewards and outcomes are deliberately excluded.
This is a research self/player model, not an admitted live opponent controller.
"""
import argparse
from collections import defaultdict, deque
import hashlib
import json
import math
from pathlib import Path
import random
import sys
import time


def load(root, train_end, val_end, history=2, max_bytes=512*1024*1024):
    root = Path(root)
    files = sorted(root.glob('studies/*/decisions.jsonl'))
    if not files or not 0 < train_end < val_end or not 1 <= history <= 8:
        raise ValueError('require completed studies, increasing seed cutoffs and history 1..8')
    if sum(p.stat().st_size for p in files) > max_bytes:
        raise ValueError('corpus exceeds the explicit local byte budget')
    groups = {name: [] for name in ('train', 'validation', 'test')}
    identities = {name: set() for name in groups}
    seeds = {name: set() for name in groups}
    schema = None
    build = None
    receipts = []
    for path in files:
        status = json.loads(path.with_name('status.json').read_text())
        if status.get('complete') is not True:
            raise ValueError(f'incomplete study: {path}')
        digest = hashlib.sha256()
        with path.open('rb') as stream:
            for chunk in iter(lambda: stream.read(1024*1024), b''):
                digest.update(chunk)
        receipts.append({'path':str(path.relative_to(root)), 'sha256':digest.hexdigest(), 'bytes':path.stat().st_size})
        with path.open() as stream:
            header = json.loads(next(stream))
            current = (header.get('feature_names'), header.get('actions'))
            if (header.get('event') != 'start' or header.get('stream_sequence') != 0
                    or header.get('telemetry_schema_version') != 2 or header.get('initial_tick') != 0
                    or not header.get('simulation_build') or not all(current)):
                raise ValueError('expected complete telemetry-v2 study starting at tick zero')
            if schema is not None and (current != schema or header['simulation_build'] != build):
                raise ValueError('mixed observation schema or simulator build')
            schema, build = current, header['simulation_build']
            features, actions = schema
            if len(features) > 128 or len(actions) > 32:
                raise ValueError('schema exceeds local model budget')
            names = {u['id']:u.get('fighter_id') for u in header['initial_units']}
            if not names or any(not n for n in names.values()):
                raise ValueError('persistent identities are required for leakage checks')
            seed = header['seed']
            split = 'train' if seed < train_end else 'validation' if seed < val_end else 'test'
            seeds[split].add(seed)
            identities[split].update(names.values())
            previous = defaultdict(lambda: deque(maxlen=history))
            ended = False
            sequence = 1
            for line in stream:
                row = json.loads(line)
                if ended or row.get('stream_sequence') != sequence:
                    raise ValueError('non-contiguous stream or data after its terminal event')
                sequence += 1
                if row['event'] == 'end':
                    ended = True
                    continue
                if row['event'] != 'decision':
                    continue
                if row.get('phase') != 'decision_before_movement' or row['unit'] not in names:
                    raise ValueError('invalid observation authority or identity')
                x = row['features']
                if len(x) != len(features) or not all(type(v) in (int,float) and math.isfinite(v) for v in x):
                    raise ValueError('invalid perceived-state features')
                y = actions.index(row['action'])
                mask = row['action_mask']
                if len(mask) != len(actions) or not all(type(v) is bool for v in mask) or not mask[y]:
                    raise ValueError('executed action must be eligible')
                past = previous[row['unit']]
                if past and row['tick'] <= past[-1][0]:
                    raise ValueError('non-increasing actor decision time')
                if past and row['tick'] != past[-1][0]+5:
                    past.clear()
                if len(past) == history:
                    causal = [v for _, values, _ in past for v in values]
                    groups[split].append((causal, y, past[-1][2], seed))
                    if sum(len(g) for g in groups.values()) > 250_000:
                        raise ValueError('corpus exceeds the explicit local sample budget')
                previous[row['unit']].append((row['tick'], x+[float(i == y) for i in range(len(actions))], y))
            if not ended:
                raise ValueError('missing terminal event despite complete status')
    if any(not rows for rows in groups.values()):
        raise ValueError('require nonempty train, validation and held-out seed groups')
    for i, a in enumerate(groups):
        for b in list(groups)[i+1:]:
            if seeds[a] & seeds[b] or identities[a] & identities[b]:
                raise ValueError('seed or persistent identity leakage between splits')
    manifest = json.loads((root/'manifest.json').read_text()) if (root/'manifest.json').exists() else {}
    return groups, schema, {'simulation_build':build, 'files':receipts, 'fixture_data':manifest.get('fixture'),
                            'seeds':{k:sorted(v) for k,v in seeds.items()}, 'history':history}


def metrics(probabilities, y, previous, actions):
    import torch
    from .train import classification
    result = classification(probabilities.argmax(1), y, actions)
    result['negative_log_likelihood'] = float(-probabilities[torch.arange(len(y)),y].clamp_min(1e-9).log().mean())
    result['brier'] = float(((probabilities-torch.nn.functional.one_hot(y,len(actions)))**2).sum(1).mean())
    changed = y != previous
    result['transition_count'] = int(changed.sum())
    result['transition_accuracy'] = float((probabilities.argmax(1)[changed] == y[changed]).float().mean()) if changed.any() else None
    return result


def run(args):
    import torch
    from torch import nn
    torch.set_num_threads(2)
    torch.manual_seed(args.seed)
    random.seed(args.seed)
    torch.use_deterministic_algorithms(True)
    groups, (features, actions), provenance = load(args.data,args.train_end,args.val_end,args.history,max_bytes=getattr(args,"max_corpus_mib",512)*1024*1024)
    output = Path(args.output)
    output.mkdir(parents=True, exist_ok=False)
    data = {k:(torch.tensor([r[0] for r in rows],dtype=torch.float32),
               torch.tensor([r[1] for r in rows]),torch.tensor([r[2] for r in rows])) for k,rows in groups.items()}
    mean = data['train'][0].mean(0)
    std = data['train'][0].std(0,unbiased=False).clamp_min(1e-4)
    inputs = {k:(x-mean)/std for k,(x,_,_) in data.items()}
    model = nn.Sequential(nn.Linear(len(mean),48),nn.ReLU(),nn.Linear(48,len(actions)))
    optimizer = torch.optim.AdamW(model.parameters(),lr=0.001,weight_decay=0.01)
    markov = torch.ones(len(actions),len(actions))
    for _, y, previous, _ in groups['train']:
        markov[previous,y] += 1
    markov /= markov.sum(1,keepdim=True)
    best = math.inf
    history = []
    started = time.perf_counter()
    checkpoint = output/'forecast.pt'
    for epoch in range(args.epochs):
        model.train()
        for indices in torch.randperm(len(inputs['train'])).split(1024):
            loss = nn.functional.cross_entropy(model(inputs['train'][indices]),data['train'][1][indices])
            optimizer.zero_grad()
            loss.backward()
            optimizer.step()
        model.eval()
        with torch.inference_mode():
            validation = float(nn.functional.cross_entropy(model(inputs['validation']),data['validation'][1]))
        history.append({'epoch':epoch+1,'validation_nll':validation})
        if validation < best:
            best = validation
            torch.save({'state_dict':model.state_dict(),'mean':mean,'std':std,'features':features,
                        'actions':actions,'history':args.history,'epoch':epoch+1},checkpoint)
    chosen = torch.load(checkpoint,map_location='cpu',weights_only=True)
    model.load_state_dict(chosen['state_dict'])
    model.eval()
    reports = {}
    with torch.inference_mode():
        for name,(x,y,previous) in data.items():
            p = model(inputs[name]).softmax(1)
            reports[name] = {'samples':len(y),'neural':metrics(p,y,previous,actions),
                             'markov':metrics(markov[previous],y,previous,actions),
                             'persistence_accuracy':float((previous == y).float().mean())}
    report = {'schema_version':1,'task':'next_self_decision_forecast','fixture_data':provenance['fixture_data'],
              'runtime':{'torch':str(torch.__version__),'python':sys.version.split()[0]},
              'training':{'seed':args.seed,'epochs':args.epochs,'history':args.history,'batch_size':1024,
                          'hidden':48,'optimizer':'AdamW','lr':0.001,'weight_decay':0.01},
              'provenance':provenance,'selection':'validation NLL; test used only after selection',
              'selected_epoch':chosen['epoch'],'parameters':sum(p.numel() for p in model.parameters()),
              'device':'cpu','threads':2,'training_and_evaluation_seconds':time.perf_counter()-started,
              'metrics':reports,'history':history,'checkpoint_sha256':hashlib.sha256(checkpoint.read_bytes()).hexdigest(),
              'admission':{'controls_live_game':False,'opponent_visible_predictor':False,
                           'reason':'Uses the decision maker own prior observations; requires a separate opponent-visible dataset and tactical evaluation before opponent deployment.'},
              'limitations':['Dataset authorship is recorded in provenance; this model is not an opponent-visible observation model.',
                             'Ticks are correlated; held-out seeds are the evaluation groups.',
                             'Forecast accuracy does not establish stronger combat or calibrated certainty.',
                             'No online training, Spark cluster or world simulation is implemented here.']}
    (output/'report.json').write_text(json.dumps(report,indent=2)+'\n')
    text = ['# PyTorch decision-sequence forecast','',
            'Research model; no live game control. Whole seeds and fighter identities are held out.',
            '', '| Split | Samples | Neural NLL | Markov NLL | Neural accuracy | Persistence accuracy |',
            '| --- | ---: | ---: | ---: | ---: | ---: |']
    for name,r in reports.items():
        text.append(f"| {name} | {r['samples']} | {r['neural']['negative_log_likelihood']:.4f} | {r['markov']['negative_log_likelihood']:.4f} | {r['neural']['accuracy']:.3f} | {r['persistence_accuracy']:.3f} |")
    text += ['', *report['limitations'], '', report['admission']['reason']]
    (output/'report.md').write_text('\n'.join(text)+'\n')
    print(json.dumps({'parameters':report['parameters'],'test':reports['test'],'output':str(output)}))
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--data',required=True)
    parser.add_argument('--output',required=True)
    parser.add_argument('--train-end',type=int,default=8)
    parser.add_argument('--val-end',type=int,default=10)
    parser.add_argument('--history',type=int,default=2)
    parser.add_argument('--max-corpus-mib',type=int,default=512)
    parser.add_argument('--epochs',type=int,default=20)
    parser.add_argument('--seed',type=int,default=23)
    args = parser.parse_args()
    if not 1 <= args.epochs <= 100:
        parser.error('epochs must be 1..100')
    run(args)


if __name__ == '__main__':
    main()
