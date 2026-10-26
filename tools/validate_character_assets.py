"""Check real exported geometry, skin weights and animation channels (stdlib)."""
import json,struct,math
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
EXPECTED=['Idle','Walk','Run','Attack','Throw','Hit','Death','AttackQuick','AttackHeavy','AttackDesperate','ThrowUnderhand','HitHeavy','LimbRecoil','DeathBackward','DeathDelayed','NearDeathPanic','WoundedStagger','WireLunge','WhirlwindCut','VaultStrike','WireRecoil','WireDeath']
report={}
for sex in ['female','male']:
 p=ROOT/'assets/characters'/(sex+'.glb');raw=p.read_bytes();size=struct.unpack_from('<I',raw,12)[0];j=json.loads(raw[20:20+size]);binary=raw[28+size:]
 def read(index):
  a=j['accessors'][index];v=j['bufferViews'][a['bufferView']];fmt={5121:'B',5123:'H',5125:'I',5126:'f'}[a['componentType']];dim={'SCALAR':1,'VEC2':2,'VEC3':3,'VEC4':4,'MAT4':16}[a['type']];s=struct.Struct('<'+fmt*dim);start=v.get('byteOffset',0)+a.get('byteOffset',0);stride=v.get('byteStride',s.size)
  return [s.unpack_from(binary,start+i*stride)for i in range(a['count'])]
 assert len(j['skins'])==1
 names=[x['name']for x in j['animations']];assert names==EXPECTED,(sex,names)
 skin=j['skins'][0];assert any(j['nodes'][i].get('name')=='mixamorigHead'for i in skin['joints'])
 # The weapon's +Y axis mounts across the palm (+/- hand X). Validate the
 # exported guard itself: a reversed wrist roll otherwise passes clip tests.
 parents={child:i for i,node in enumerate(j['nodes'])for child in node.get('children',[])}
 idle=j['animations'][0]
 rotations={c['target']['node']:read(idle['samplers'][c['sampler']]['output'])[0]
            for c in idle['channels']if c['target']['path']=='rotation'}
 def multiply(a,b):
  x,y,z,w=a;X,Y,Z,W=b
  return (w*X+x*W+y*Z-z*Y,w*Y-x*Z+y*W+z*X,w*Z+x*Y-y*X+z*W,w*W-x*X-y*Y-z*Z)
 def global_rotation(i):
  q=rotations.get(i,j['nodes'][i].get('rotation',[0,0,0,1]))
  return multiply(global_rotation(parents[i]),q)if i in parents else q
 for side,axis in [('Right',(-1,0,0,0)),('Left',(1,0,0,0))]:
  wrist=next(i for i,node in enumerate(j['nodes'])if node.get('name')=='mixamorig'+side+'Hand')
  q=global_rotation(wrist);blade=multiply(multiply(q,axis),(-q[0],-q[1],-q[2],q[3]))
  assert blade[1]>.70,(sex,side,'weapon guard must point upward',blade)
  assert blade[0]*(1 if side=='Left' else -1)>.15,(sex,side,'blade must clear the head silhouette',blade)
 faces=0;verts=0
 for m in j['meshes']:
  for prim in m['primitives']:
   a=prim['attributes']
   if 'WEIGHTS_0' not in a:continue # Bone-parented eyeballs/iris caps are rigid meshes.
   positions=read(a['POSITION']);weights=read(a['WEIGHTS_0']);joints=read(a['JOINTS_0']);indices=[v[0]for v in read(prim['indices'])]
   assert len(positions)==len(weights)==len(joints)
   assert all(all(math.isfinite(x)for x in p)for p in positions)
   assert all(abs(sum(w)-1)<.002 for w in weights)
   assert all(all(0<=x<len(skin['joints'])for x in v)for v in joints)
   assert all(0<=i<len(positions)for i in indices)
   assert len(indices)>20000,'An intact body must retain thousands of triangles.'
   # Degenerate output faces indicate a broken collapse or export.
   degenerate=0
   for k in range(0,len(indices),3):
    a,b,c=[positions[i]for i in indices[k:k+3]];u=[b[i]-a[i]for i in range(3)];v=[c[i]-a[i]for i in range(3)];cross=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]]
    if sum(x*x for x in cross)<1e-20:degenerate+=1
   assert degenerate/(len(indices)//3)<.002,(sex,degenerate)
   faces+=len(indices)//3;verts+=len(positions)
 for clip in j['animations']:
  assert len(clip['channels'])>60
  # Named clips and valid channels are insufficient: REST-mode baking used to
  # export seven completely static animations. Require real changing samples.
  motion=False
  for channel in clip['channels']:
   values=read(clip['samplers'][channel['sampler']]['output'])
   if values and any(sum((a-b)**2 for a,b in zip(value,values[0]))>1e-8 for value in values):
    motion=True;break
  assert motion,(sex,clip['name'],'clip is static')
  if clip['name'] in ['WireLunge','WhirlwindCut','VaultStrike','WireRecoil']:
   channel=next(c for c in clip['channels']if c['target']['path']=='translation'and j['nodes'][c['target']['node']].get('name')=='mixamorigHips')
   positions=read(clip['samplers'][channel['sampler']]['output'])
   distance=lambda p:sum((a-b)**2 for a,b in zip(p,positions[0]))**.5
   assert max(map(distance,positions))>.4,(sex,clip['name'],'missing aerial motion')
   assert distance(positions[-1])<.005,(sex,clip['name'],'must return to its ground anchor')
  for sampler in clip['samplers']:
   times=[t[0]for t in read(sampler['input'])];assert times[-1]>0 and times==sorted(times)
   assert all(all(math.isfinite(x)for x in v)for v in read(sampler['output']))
 report[sex]={'vertices':verts,'triangles':faces,'clips':names,'skins':len(j['skins']),'bytes':len(raw)}
print(json.dumps(report,indent=2))
