"""Filter sculpt normal noise, including every facial morph, after GLB export.

Positions, topology, UVs, joints and animation remain byte-for-byte intact.
The small spatial kernel preserves the face silhouette and avoids averaging
opposite sides of a surface. Standard library only; called by the exporter.
"""
import itertools
import json
import math
from pathlib import Path
import struct


def unit(v):
    length = math.sqrt(sum(x*x for x in v))
    return tuple(x/length for x in v) if length > 1e-9 else (0., 1., 0.)


def smooth(path):
    raw = path.read_bytes()
    size = struct.unpack_from('<I', raw, 12)[0]
    doc = json.loads(raw[20:20+size])
    if doc.get('extras', {}).get('phalanx_surface_normals') == 1:
        return
    binary = bytearray(raw[28+size:])

    def layout(index):
        a = doc['accessors'][index]
        assert a['componentType'] == 5126 and a['type'] == 'VEC3' and 'sparse' not in a
        v = doc['bufferViews'][a['bufferView']]
        return a['count'], v.get('byteOffset', 0)+a.get('byteOffset', 0), v.get('byteStride', 12)

    def read(index):
        count, offset, stride = layout(index)
        return [struct.unpack_from('<3f', binary, offset+i*stride) for i in range(count)]

    def write(index, values):
        count, offset, stride = layout(index)
        assert count == len(values)
        for i, value in enumerate(values):
            struct.pack_into('<3f', binary, offset+i*stride, *value)

    head = next(n for n in doc['nodes'] if n.get('name') == 'Face_Sculpt')
    for primitive in doc['meshes'][head['mesh']]['primitives']:
        points = read(primitive['attributes']['POSITION'])
        base = read(primitive['attributes']['NORMAL'])
        controls = [t['NORMAL'] for t in primitive['targets']]
        fields = [base] + [[unit(tuple(n+d for n, d in zip(normal, delta)))
                           for normal, delta in zip(base, read(index))] for index in controls]
        radius = .016
        keys = [tuple(math.floor(x/radius) for x in p) for p in points]
        buckets = {}
        for i, key in enumerate(keys):
            buckets.setdefault(key, []).append(i)
        centres = {k: tuple(sum(points[i][a] for i in ids)/len(ids) for a in range(3))
                   for k, ids in buckets.items()}
        means = [{k: tuple(sum(field[i][a] for i in ids)/len(ids) for a in range(3))
                  for k, ids in buckets.items()} for field in fields]
        filtered = [[] for _ in fields]
        neighbours = list(itertools.product((-1, 0, 1), repeat=3))
        for p, normal, key in zip(points, base, keys):
            weights = []
            for delta in neighbours:
                k = tuple(x+d for x, d in zip(key, delta))
                if k not in centres or sum(x*y for x, y in zip(normal, means[0][k])) < .3:
                    continue
                distance = sum((x-y)**2 for x, y in zip(p, centres[k]))
                weights.append((k, math.exp(-distance/(2*radius*radius))))
            for output, field in zip(filtered, means):
                output.append(unit(tuple(sum(field[k][a]*w for k, w in weights) for a in range(3))))
        write(primitive['attributes']['NORMAL'], filtered[0])
        for index, field in zip(controls, filtered[1:]):
            write(index, [tuple(t-b for t, b in zip(target, normal))
                          for target, normal in zip(field, filtered[0])])
    doc.setdefault('extras', {})['phalanx_surface_normals'] = 1
    payload = json.dumps(doc, separators=(',', ':')).encode()
    payload += b' '*((-len(payload)) % 4)
    total = 12+8+len(payload)+8+len(binary)
    path.write_bytes(struct.pack('<III', 0x46546c67, 2, total)
                     + struct.pack('<I4s', len(payload), b'JSON')+payload
                     + struct.pack('<I4s', len(binary), b'BIN\0')+binary)
    print(path.name, 'facial base and morph normals filtered; geometry preserved')


if __name__ == '__main__':
    root = Path(__file__).resolve().parents[1]/'assets/characters'
    for sex in ('male', 'female'):
        smooth(root/f'{sex}.glb')
