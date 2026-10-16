"""Check the new independent eye rigs in edited assets and runtime exports."""
import json
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
for sex in ('male', 'female'):
    for suffix in ('-eyed', ''):
        path = ROOT / 'assets/characters' / f'{sex}{suffix}.glb'
        raw = path.read_bytes()
        length = struct.unpack_from('<I', raw, 12)[0]
        doc = json.loads(raw[20:20+length])
        nodes = doc['nodes']
        ids = {node.get('name'): i for i, node in enumerate(nodes)}
        parents = {child: i for i, node in enumerate(nodes) for child in node.get('children', [])}
        head = ids['mixamorigHead']
        for side in ('L', 'R'):
            bone, eye, iris = (ids[prefix+side] for prefix in ('Eye_', 'Eyeball_', 'Iris_'))
            assert parents[bone] == head, (path, side, 'eye bone must follow head')
            assert parents[eye] == bone, (path, side, 'eyeball must follow eye bone')
            assert parents[iris] == eye, (path, side, 'iris must follow eyeball')
            assert 'mesh' in nodes[eye] and 'mesh' in nodes[iris]
            assert len(doc['meshes'][nodes[iris]['mesh']]['primitives']) >= 3
        assert len(doc.get('skins', [])) == 1
        print(f'{path.name}: head-parented eye bones, independent eyeballs and curved irises OK')

# Runtime facial shapes must contain real deformation and no weights channels
# that allow the body clip player to overwrite the live expression controller.
import math
KEYS=['Rage','Pain','Fear','Grin','Shout','Blink','NoseLong','NoseWide','JawWide','CheekSoft']
for sex in ('male','female'):
    raw=(ROOT/'assets/characters'/f'{sex}.glb').read_bytes()
    length=struct.unpack_from('<I',raw,12)[0]
    doc=json.loads(raw[20:20+length]);binary=raw[28+length:]
    def read(index):
        a=doc['accessors'][index];dim={'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4}[a['type']]
        def values(view,offset,count,fmt):
            v=doc['bufferViews'][view];record=struct.Struct('<'+fmt)
            start=v.get('byteOffset',0)+offset;stride=v.get('byteStride',record.size)
            return [record.unpack_from(binary,start+i*stride)for i in range(count)]
        fmt={5121:'B',5123:'H',5125:'I',5126:'f'}[a['componentType']]*dim
        data=values(a['bufferView'],a.get('byteOffset',0),a['count'],fmt)if 'bufferView'in a else [(0,)*dim]*a['count']
        if 'sparse'in a:
            sparse=a['sparse'];ids=sparse['indices'];v=sparse['values']
            indices=values(ids['bufferView'],ids.get('byteOffset',0),sparse['count'],{5121:'B',5123:'H',5125:'I'}[ids['componentType']])
            updates=values(v['bufferView'],v.get('byteOffset',0),sparse['count'],fmt)
            for (at,),update in zip(indices,updates):data[at]=update
        return data
    changes=[0.0]*len(KEYS);morphs=0
    for mesh in doc['meshes']:
        if not mesh.get('weights'):continue
        assert mesh['extras']['targetNames']==KEYS
        assert all(w==0 for w in mesh['weights'])
        morphs+=1
        for primitive in mesh['primitives']:
            for i,target in enumerate(primitive['targets']):
                delta=read(target['POSITION'])
                assert all(math.isfinite(x) for p in delta for x in p)
                changes[i]=max(changes[i],max((sum(x*x for x in p)**.5 for p in delta),default=0))
    assert morphs>=10,(sex,morphs)
    assert all(.001<x<.5 for x in changes),(sex,changes)
    assert not any(c['target']['path']=='weights' for a in doc['animations']for c in a['channels'])
    print(sex,'facial morph meshes',morphs,'maximum displacement per control',dict(zip(KEYS,map(lambda x:round(x,4),changes))))
