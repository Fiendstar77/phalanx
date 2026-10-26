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
KEYS=['Rage','Pain','Fear','Grin','Shout','Blink','NoseLong','NoseWide','JawWide','CheekSoft',
      'FaceRound','FaceTall','NoseUpturned','MouthSweet','MouthPout','BrowSoft','BrowArch']
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
    assert changes[6] > .045 and changes[7] > .015, (sex, 'nose choices must visibly deform the sculpt', changes)
    assert not any(c['target']['path']=='weights' for a in doc['animations']for c in a['channels'])
    print(sex,'facial morph meshes',morphs,'maximum displacement per control',dict(zip(KEYS,map(lambda x:round(x,4),changes))))
    # Check exported receiver-space winding, not only recalculated normals.
    nodes=doc['nodes'];ids={n.get('name'):i for i,n in enumerate(nodes)}
    def subtract(a,b):return tuple(x-y for x,y in zip(a,b))
    def cross(a,b):return (a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0])
    def add(a,b):return tuple(x+y for x,y in zip(a,b))
    def rotate(q,v):
        t=tuple(2*x for x in cross(q[:3],v))
        return add(v,add(tuple(q[3]*x for x in t),cross(q[:3],t)))
    def parent_point(node,p):
        scaled=tuple(x*y for x,y in zip(p,node.get('scale',[1,1,1])))
        return add(node.get('translation',[0,0,0]),rotate(node.get('rotation',[0,0,0,1]),scaled))
    head=doc['meshes'][nodes[ids['Face_Sculpt']]['mesh']]
    count=sum(len(read(p['attributes']['POSITION'])) for p in head['primitives'])
    assert count>=26000,(sex,'head lacks deformation support',count)
    for side in ('L','R'):
        lid=nodes[ids['Lid_'+side]];eye=nodes[ids['Eye_'+side]]
        centre=parent_point(eye,nodes[ids['Eyeball_'+side]].get('translation',[0,0,0]))
        for primitive in doc['meshes'][lid['mesh']]['primitives']:
            points=[parent_point(lid,p) for p in read(primitive['attributes']['POSITION'])]
            indices=[i[0]for i in read(primitive['indices'])]
            # Every staged closure must stay outside the eye, including
            # interpolation between stages, not merely the fully closed key.
            base=read(primitive['attributes']['POSITION'])
            deltas={name:read(primitive['targets'][KEYS.index(name)]['POSITION']) for name in ('Rage','Grin','Pain','Blink')}
            stages=['Rage','Grin','Pain','Blink'];eye_radius=.123 if sex=='male' else .128
            for closure in [i/40 for i in range(41)]:
                t=closure*4;slot=min(3,int(t));fraction=t-slot
                if slot>0:
                    a=deltas[stages[slot-1]]
                else:a=[(0,0,0)]*len(base)
                b=deltas[stages[slot]]
                for p,da,db in zip(base,a,b):
                    q=parent_point(lid,tuple(x+(1-fraction)*u+fraction*v for x,u,v in zip(p,da,db)))
                    assert math.dist(q,centre)>eye_radius,(sex,side,'lid intersects eyeball',closure)
            for i in range(0,len(indices),3):
                a,b,c=(points[j]for j in indices[i:i+3])
                normal=cross(subtract(b,a),subtract(c,a))
                midpoint=tuple((x+y+z)/3 for x,y,z in zip(a,b,c))
                assert sum(x*y for x,y in zip(normal,subtract(midpoint,centre)))>0,(sex,side,'inward eyelid triangle',i)
    print(sex,'head support vertices',count,'all exported eyelid triangles outward')
