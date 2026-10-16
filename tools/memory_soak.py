"""Bounded Linux render-memory soak in an isolated campaign directory.

Examples:
  python tools/memory_soak.py --seconds 600 --labels default
  python tools/memory_soak.py --seconds 180 --labels on --output outputs/labels-on
  python tools/memory_soak.py --seconds 180 --labels off --output outputs/labels-off
Native labels can be disabled without disabling backend validation. This tool
never edits the real campaign, and terminates only the child it starts.
"""
import argparse
import json
import os
from pathlib import Path
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, default=ROOT/'target/debug/phalanx')
    parser.add_argument('--seconds', type=float, default=300)
    parser.add_argument('--warmup', type=float, default=30)
    parser.add_argument('--interval', type=float, default=5)
    parser.add_argument('--max-rss-mib', type=float, default=6144)
    parser.add_argument('--labels', choices=['default', 'on', 'off'], default='default')
    parser.add_argument('--output', type=Path, default=ROOT/'outputs/memory-soak')
    args = parser.parse_args()
    if not Path('/proc/self/status').exists():
        parser.error('This sampler requires Linux /proc.')
    if not (args.seconds > args.warmup >= 0 and args.interval > 0 and args.max_rss_mib > 0):
        parser.error('Require seconds > warmup >= 0, positive interval and RSS limit.')
    args.output = args.output.resolve()
    args.output.mkdir(parents=True, exist_ok=True)
    samples = []
    validation_loaded = False
    reason = 'duration reached'
    env = os.environ.copy()
    env.setdefault('RUST_LOG', 'info')
    if args.labels != 'default':
        env['WGPU_DISCARD_HAL_LABELS'] = '0' if args.labels == 'on' else '1'
    with tempfile.TemporaryDirectory(prefix='phalanx-soak-') as scratch:
        work = Path(scratch)
        (work/'assets').symlink_to(ROOT/'assets', target_is_directory=True)
        (work/'outputs').mkdir()
        with (args.output/'game.log').open('w') as log, (args.output/'rss.jsonl').open('w') as rows:
            process = subprocess.Popen([str(args.binary.resolve()), '--battle', '--arena', 'quarry',
                                        '--memory-soak', '--profile'], cwd=work, env=env,
                                       stdout=log, stderr=subprocess.STDOUT)
            start = time.monotonic()
            try:
                while time.monotonic()-start < args.seconds:
                    if process.poll() is not None:
                        reason = f'child exited early: {process.returncode}'
                        break
                    status = Path(f'/proc/{process.pid}/status')
                    try:
                        values = dict(line.split(':', 1) for line in status.read_text().splitlines() if ':' in line)
                        row = {'seconds': round(time.monotonic()-start, 3),
                               'rss_kib': int(values['VmRSS'].split()[0]),
                               'anonymous_kib': int(values['RssAnon'].split()[0])}
                        validation_loaded |= 'libVkLayer_khronos_validation' in Path(f'/proc/{process.pid}/maps').read_text()
                    except (FileNotFoundError, KeyError, ProcessLookupError):
                        reason = 'child vanished while sampling'
                        break
                    samples.append(row)
                    rows.write(json.dumps(row)+'\n'); rows.flush()
                    if row['rss_kib']/1024 > args.max_rss_mib:
                        reason = 'RSS limit exceeded'
                        break
                    time.sleep(min(args.interval, max(0, args.seconds-(time.monotonic()-start))))
            except KeyboardInterrupt:
                reason = 'interrupted by operator'
            finally:
                if process.poll() is None:
                    process.terminate()
                    try:
                        process.wait(timeout=10)
                    except subprocess.TimeoutExpired:
                        process.kill(); process.wait()
    steady = [s for s in samples if s['seconds'] >= args.warmup]
    growth = (steady[-1]['rss_kib']-steady[0]['rss_kib'])/1024 if len(steady)>1 else None
    span = steady[-1]['seconds']-steady[0]['seconds'] if len(steady)>1 else None
    summary = {'native_labels': args.labels,
               'label_environment_override': env.get('WGPU_DISCARD_HAL_LABELS'),
               'validation_layer_loaded': validation_loaded,
               'requested_seconds': args.seconds, 'warmup_seconds': args.warmup,
               'sampled_seconds': samples[-1]['seconds'] if samples else 0,
               'samples': len(samples), 'stop_reason': reason,
               'peak_rss_mib': max((s['rss_kib']/1024 for s in samples), default=0),
               'steady_growth_mib': growth,
               'steady_growth_mib_per_minute': growth*60/span if span else None,
               'campaign_isolated': True}
    (args.output/'summary.json').write_text(json.dumps(summary, indent=2)+'\n')
    print(json.dumps(summary, indent=2), flush=True)
    if reason != 'duration reached':
        raise SystemExit(1)


if __name__ == '__main__':
    main()
