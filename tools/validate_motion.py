"""Measure exported foot contacts, sampling and vault offsets, using stdlib only.

These are geometric regressions, not a claim of animation artistry or a player test.
Run from any directory: python3 tools/validate_motion.py.
"""
import json
import math
from pathlib import Path
import struct

ROOT = Path(__file__).resolve().parents[1]


def multiply(a, b):
    x, y, z, w = a
    X, Y, Z, W = b
    return (w*X+x*W+y*Z-z*Y, w*Y-x*Z+y*W+z*X,
            w*Z+x*Y-y*X+z*W, w*W-x*X-y*Y-z*Z)


def rotate(q, v):
    return multiply(multiply(q, (*v, 0)), (-q[0], -q[1], -q[2], q[3]))[:3]


def inspect(path):
    raw = path.read_bytes()
    size, = struct.unpack_from('<I', raw, 12)
    gltf = json.loads(raw[20:20+size])
    binary = raw[28+size:]
    nodes = gltf['nodes']
    parents = {child: i for i, node in enumerate(nodes) for child in node.get('children', [])}
    ids = {node.get('name'): i for i, node in enumerate(nodes)}

    def read(index):
        a = gltf['accessors'][index]
        view = gltf['bufferViews'][a['bufferView']]
        dim = {'SCALAR': 1, 'VEC3': 3, 'VEC4': 4}[a['type']]
        item = struct.Struct('<' + 'f' * dim)
        offset = view.get('byteOffset', 0) + a.get('byteOffset', 0)
        return [item.unpack_from(binary, offset + i*view.get('byteStride', item.size))
                for i in range(a['count'])]

    def world(index, channels, frame, cache):
        if index in cache:
            return cache[index]
        node = nodes[index]
        def channel(kind, default):
            values = channels.get((index, kind), [node.get(kind, default)])
            return values[min(frame, len(values)-1)]
        p = channel('translation', [0, 0, 0])
        q = channel('rotation', [0, 0, 0, 1])
        s = channel('scale', [1, 1, 1])
        if index in parents:
            P, Q, S = world(parents[index], channels, frame, cache)
            delta = rotate(Q, [p[j]*S[j] for j in range(3)])
            p = [P[j]+delta[j] for j in range(3)]
            q = multiply(Q, q)
            s = [S[j]*s[j] for j in range(3)]
        cache[index] = (p, q, s)
        return cache[index]

    result = {}
    for name in ['Walk', 'Run', 'SneakStep', 'SprintLow', 'SprintBlade', 'GuardStrafe',
                 'Attack', 'AttackQuick', 'AttackHeavy', 'CounterCut', 'WallKickStrike']:
        clip = next(c for c in gltf['animations'] if c['name'] == name)
        channels = {(c['target']['node'], c['target']['path']):
                    read(clip['samplers'][c['sampler']]['output']) for c in clip['channels']}
        times = [v[0] for v in read(clip['samplers'][0]['input'])]
        assert max(b-a for a, b in zip(times, times[1:])) < 1/59, (path, name, 'sampling')
        if name == 'WallKickStrike':
            hip = channels[(ids['mixamorigHips'], 'translation')]
            span = max(math.dist(p, hip[0]) for p in hip)
            assert span < 0.1, (path, name, 'visual jump duplicates physical height', span)
            result[name] = {'maximum_local_hip_offset_m': span}
            continue
        gait = name in ['Walk', 'Run', 'SneakStep', 'SprintLow', 'SprintBlade', 'GuardStrafe']
        duty = .48 if name in ['Run', 'SprintLow', 'SprintBlade'] else .62
        errors = []
        for side, phase_offset in [('Left', 0), ('Right', .5)]:
            foot = ids[f'mixamorig{side}Foot']
            rest = world(foot, {}, 0, {})[0]
            for frame, t in enumerate(times):
                point = world(foot, channels, frame, {})[0]
                assert all(math.isfinite(v) for v in point)
                phase = (t/times[-1] + phase_offset) % 1
                if not gait or phase < duty:
                    errors.append(abs(point[1]-rest[1]))
        worst = max(errors)
        assert worst < .03, (path, name, 'planted ankle height error', worst)
        result[name] = {'maximum_planted_ankle_height_error_m': worst, 'samples': len(times)}
    return result


if __name__ == '__main__':
    print(json.dumps({sex: inspect(ROOT/'assets/characters'/f'{sex}.glb')
                      for sex in ['male', 'female']}, indent=2))
