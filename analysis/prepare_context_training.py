"""Create a provenance-linked utility-only view of completed actor-observation studies."""
import argparse
import hashlib
import json
from pathlib import Path


def prepare(source, output):
    source, output = Path(source), Path(output)
    if output.exists():
        raise ValueError('use a fresh training-view directory')
    output.mkdir(parents=True)
    manifest = json.loads((source/'manifest.json').read_text())
    receipts=[]; kept=excluded=0
    for index, path in enumerate(sorted(source.glob('studies/*/decisions.jsonl'))):
        if json.loads(path.with_name('status.json').read_text()).get('complete') is not True:
            raise ValueError('training requires a completed study')
        digest=hashlib.sha256(path.read_bytes()).hexdigest()
        with path.open() as stream, (output/f'{index:03}.jsonl').open('w') as dest:
            header=json.loads(next(stream))
            if header['event']!='start' or header['controller']!='Utility':
                raise ValueError('expected a utility-led study')
            dest.write(json.dumps(header)+'\n')
            for line in stream:
                row=json.loads(line)
                if row['event']!='decision':
                    continue
                if (row.get('comparison') or {}).get('controller','Utility')!='Utility':
                    excluded+=1
                    continue
                dest.write(line); kept+=1
        receipts.append({'source':str(path.relative_to(source)), 'sha256':digest})
    result={'source_sha256':manifest['source_sha256'],'source_corpus':str(source),
            'kept_utility_decisions':kept,'excluded_composed_decisions':excluded,'sources':receipts,
            'scope':'Actor observations and guarded utility imitation; composed planner decisions are retained only in the original studies and causal forecasting corpus.'}
    (output/'manifest.json').write_text(json.dumps(result,indent=2)+'\n')
    return result

if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('source');parser.add_argument('output');args=parser.parse_args()
    result=prepare(args.source,args.output)
    print(json.dumps({key:result[key] for key in ['kept_utility_decisions','excluded_composed_decisions']}))
