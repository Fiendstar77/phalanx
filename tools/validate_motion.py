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
    wall_names=['WallKickStrike']+[c['name'] for c in gltf['animations'] if c['name'].startswith('WallVault_')]
    assert len(wall_names)==15,(path,'expected fifteen wall performances',wall_names)
    wall_poses={}
    for name in ['Walk', 'Run', 'SneakStep', 'SprintLow', 'SprintBlade', 'GuardStrafe',
                 'Attack', 'AttackQuick', 'AttackHeavy', 'CounterCut',
                 'ScoutScan','LowSurvey','GuardListen','ShoulderCheck','SignalAdvance','SignalHold','ReadyWeapon','WatchShift',
                 'GuardLeftArm','GuardRightArm','LimpLeft','LimpRight','TorsoHunch','WoundFlinch','ExhaustedBrace','ReliefBreath']+wall_names:
        clip = next(c for c in gltf['animations'] if c['name'] == name)
        channels = {(c['target']['node'], c['target']['path']):
                    read(clip['samplers'][c['sampler']]['output']) for c in clip['channels']}
        channel_times={(c['target']['node'],c['target']['path']):
                       [v[0] for v in read(clip['samplers'][c['sampler']]['input'])] for c in clip['channels']}
        times=max(channel_times.values(),key=len)
        # glTF may compress constant/linear channels to two keys. Check the
        # dense baked stream, then sample each joint on its own time axis.
        assert max(b-a for a,b in zip(times,times[1:]))<1/59,(path,name,'sampling')
        if name in wall_names:
            hip = channels[(ids['mixamorigHips'], 'translation')]
            span = max(math.dist(p, hip[0]) for p in hip)
            assert span < 0.1, (path, name, 'visual jump duplicates physical height', span)
            assert math.dist(hip[0],hip[-1])<.001,(path,name,'must return to its physical anchor')
            def sample_rotation(bone,fraction):
                key=(ids['mixamorig'+bone],'rotation');samples=channel_times[key]
                index=min(range(len(samples)),key=lambda i:abs(samples[i]-fraction*times[-1]))
                return channels[key][index]
            wall_poses[name]=[sample_rotation(bone,fraction) for fraction in [.2,.4,.65,.85]
                for bone in ['Hips','Spine','LeftUpLeg','RightUpLeg','LeftLeg','RightLeg','LeftArm','RightArm']]
            result[name] = {'maximum_local_hip_offset_m': span}
            continue
        gait = name in ['Walk', 'Run', 'SneakStep', 'SprintLow', 'SprintBlade', 'GuardStrafe','LimpLeft','LimpRight']
        duty = .48 if name in ['Run', 'SprintLow', 'SprintBlade'] else .62
        errors = []
        for side, phase_offset in [('Left', 0), ('Right', .5)]:
            foot = ids[f'mixamorig{side}Foot']
            rest = world(foot, {}, 0, {})[0]
            for frame, t in enumerate(times):
                point = world(foot, channels, frame, {})[0]
                assert all(math.isfinite(v) for v in point)
                phase = (t/times[-1] + phase_offset) % 1
                support_duty=.72 if name=='Limp'+side else duty
                if not gait or phase < support_duty:
                    errors.append(abs(point[1]-rest[1]))
        worst = max(errors)
        assert worst < .03, (path, name, 'planted ankle height error', worst)
        result[name] = {'maximum_planted_ankle_height_error_m': worst, 'samples': len(times)}
    separations=[]
    for i,a in enumerate(wall_names):
        for b in wall_names[i+1:]:
            distance=max(2*math.acos(min(1.,abs(sum(x*y for x,y in zip(q,r))))) for q,r in zip(wall_poses[a],wall_poses[b]))
            assert distance>.20,(path,a,b,'renamed or insufficiently distinct wall performance',distance)
            separations.append(distance)
    result['wall_performance_family']={'count':len(wall_names),'minimum_pairwise_joint_separation_radians':min(separations)}
    return result


if __name__ == '__main__':
    print(json.dumps({sex: inspect(ROOT/'assets/characters'/f'{sex}.glb')
                      for sex in ['male', 'female']}, indent=2))
