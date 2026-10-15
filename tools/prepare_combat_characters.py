"""Build derived game assets; preserve the artist's source GLBs.
Run with Blender Python (bpy): python tools/prepare_combat_characters.py.
Shared donor weights suit these matching T-pose chibi bases; motion is authored
procedurally here, not motion capture. Preview the exports before acceptance.
"""
import bpy, bmesh, math, json
from pathlib import Path
from mathutils import Vector, Quaternion
from mathutils.kdtree import KDTree
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'assets/characters';OUT.mkdir(exist_ok=True)
CLIPS=[('Idle',1.6),('Walk',.9),('Run',.6),('Attack',.65),('Throw',1.1),('Hit',.38),('Death',1.1)]
def clean():bpy.ops.wm.read_factory_settings(use_empty=True)
def import_base(f):
 bpy.ops.import_scene.gltf(filepath=str(ROOT/'assets'/f))
 return max((o for o in bpy.context.scene.objects if o.type=='MESH'),key=lambda o:len(o.data.vertices))
def rot(rig,name,axis,angle):
 b=rig.pose.bones.get('mixamorig'+name)
 if not b:return
 basis=b.bone.matrix_local.to_quaternion();b.rotation_mode='QUATERNION'
 b.rotation_quaternion=b.rotation_quaternion @ (basis.inverted()@Quaternion(axis,angle)@basis)
def bake(rig):
 for name,duration in CLIPS:
  action=bpy.data.actions.new(name);rig.animation_data_create();rig.animation_data.action=action
  frames=round(duration*30)
  for frame in range(frames+1):
   u=frame/frames;phase=u*math.tau;wave=math.sin(phase)
   for b in rig.pose.bones:b.rotation_mode='QUATERNION';b.rotation_quaternion=Quaternion();b.location=Vector()
   for side,sign in [('Left',1),('Right',-1)]:
    rot(rig,side+'Arm',(0,1,0),sign*1.18)
    rot(rig,side+'ForeArm',(1,0,0),-.28)
   if name in ['Walk','Run']:
    stride=.46 if name=='Walk' else .78
    for side,sign in [('Left',1),('Right',-1)]:
     rot(rig,side+'UpLeg',(1,0,0),wave*sign*stride)
     rot(rig,side+'Leg',(1,0,0),max(0,-wave*sign)*stride*.85)
     rot(rig,side+'Arm',(1,0,0),-wave*sign*stride*.65)
    rig.pose.bones['mixamorigHips'].location.z=abs(wave)*.018
    rot(rig,'Spine',(1,0,0),-.08 if name=='Run' else -.025)
   elif name in ['Attack','Throw']:
    pulse=math.sin(math.pi*u)**2
    rot(rig,'Spine2',(0,0,1),pulse*.30)
    rot(rig,'RightArm',(1,0,0),-pulse*(2.25 if name=='Throw' else 1.4))
    rot(rig,'RightForeArm',(1,0,0),-math.sin(math.pi*u)*.8)
    rot(rig,'LeftArm',(1,0,0),-.3*pulse)
   elif name=='Hit':
    pulse=math.sin(math.pi*u);rot(rig,'Spine',(1,0,0),.30*pulse);rot(rig,'Head',(1,0,0),.18*pulse)
   elif name=='Death':
    drop=min(1,u/.8);ease=drop*drop*(3-2*drop)
    rot(rig,'Hips',(1,0,0),-1.48*ease)
    # Hips bone's local Y runs vertically in this imported rig.
    rig.pose.bones['mixamorigHips'].location.y=-.65*ease
    rot(rig,'LeftLeg',(1,0,0),.4*ease)
   else:rot(rig,'Spine2',(1,0,0),math.sin(phase)*.015)
   for b in rig.pose.bones:
    b.keyframe_insert('rotation_quaternion',frame=frame)
    if b.name=='mixamorigHips':b.keyframe_insert('location',frame=frame)
  rig.animation_data.action=None
  track=rig.animation_data.nla_tracks.new();track.name=name;strip=track.strips.new(name,0,action);strip.action_frame_start=0;strip.action_frame_end=frames
 bpy.context.scene.render.fps=30
 # Neutral authored rest state: exporter bakes each NLA track independently.
 for b in rig.pose.bones:b.rotation_quaternion=Quaternion();b.location=Vector()
 bpy.context.scene.frame_set(0)
def export(sex,mesh,rig):
 bpy.ops.object.select_all(action='DESELECT');mesh.select_set(True);rig.select_set(True)
 bpy.context.view_layer.objects.active=rig
 bpy.ops.export_scene.gltf(filepath=str(OUT/(sex+'.glb')),export_format='GLB',use_selection=True,export_animations=True,export_animation_mode='NLA_TRACKS',export_nla_strips=True,export_frame_range=False,export_force_sampling=True,export_skins=True)
clean();donor=import_base('phalanx_base_male.glb');rig=next(o for o in bpy.context.scene.objects if o.type=='ARMATURE')
# Capture donor geometry and weights in shared world coordinates before cleaning.
kd=KDTree(len(donor.data.vertices));weights=[]
for v in donor.data.vertices:
 kd.insert(donor.matrix_world@v.co,v.index)
 weights.append([(donor.vertex_groups[g.group].name,g.weight)for g in v.groups])
kd.balance()
# Keep the exact imported male skin weights and skeleton.
for o in list(bpy.context.scene.objects):
 if o.type=='MESH' and o!=donor:bpy.data.objects.remove(o,do_unlink=True)
bake(rig);export('male',donor,rig)
# Reimport the original rig: animation data must not leak between exports.
clean();male=import_base('phalanx_base_male.glb');rig=next(o for o in bpy.context.scene.objects if o.type=='ARMATURE')
female=import_base('Full_base_female_phalanx.glb')
# Studio exports disconnected per-triangle vertices. Weld before smoothing or
# decimation; collapsing isolated triangles would punch holes across the body.
mesh=female.data
bm=bmesh.new();bm.from_mesh(mesh);bmesh.ops.remove_doubles(bm,verts=list(bm.verts),dist=0.00001);bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces));bm.to_mesh(mesh);bm.free();mesh.update()
# Smooth only the cheek band, below eyes and above jaw, on the front surface.
original=[v.co.copy() for v in mesh.vertices];adj=[set() for _ in mesh.vertices]
for e in mesh.edges:
 a,b=e.vertices;adj[a].add(b);adj[b].add(a)
changed=0
for _ in range(5):
 coords=[v.co.copy()for v in mesh.vertices]
 for i,v in enumerate(mesh.vertices):
  p=female.matrix_world@original[i]
  w=math.exp(-((abs(p.x)-.205)/.085)**2-((p.z-1.345)/.09)**2)
  w*=max(0,min(1,(-p.y-.11)/.12))
  if w>.002 and adj[i]:
   avg=sum((coords[k]for k in adj[i]),Vector())/len(adj[i]);v.co=v.co.lerp(avg,w*.55)
# Reduce the high lateral cheek ridge while preserving nose, eyes and mouth.
for i,v in enumerate(mesh.vertices):
 p=female.matrix_world@original[i];w=math.exp(-((abs(p.x)-.20)/.07)**2-((p.z-1.36)/.065)**2)*max(0,min(1,(-p.y-.11)/.12))
 if w>.01:v.co.x*=1-.055*w;changed+=1
# LOD-sized render mesh; preserve topology/UVs on the original file.
bpy.context.view_layer.objects.active=female
mod=female.modifiers.new('Game mesh reduction','DECIMATE');mod.ratio=.19;bpy.ops.object.modifier_apply(modifier=mod.name)
female.data.normals_split_custom_set([(0,0,0)]*len(female.data.loops))
for poly in female.data.polygons:poly.use_smooth=True
for group in male.vertex_groups:
 if group.name not in female.vertex_groups:female.vertex_groups.new(name=group.name)
for v in female.data.vertices:
 _,idx,_=kd.find(female.matrix_world@v.co)
 for name,w in weights[idx]:female.vertex_groups[name].add([v.index],w,'REPLACE')
world=female.matrix_world.copy();female.parent=rig;female.matrix_world=world
mod=female.modifiers.new('Shared chibi skin','ARMATURE');mod.object=rig
for o in list(bpy.context.scene.objects):
 if o.type=='MESH' and o!=female:bpy.data.objects.remove(o,do_unlink=True)
bake(rig);export('female',female,rig)
(OUT/'manifest.json').write_text(json.dumps({'source_models_preserved':True,'female_cheek_vertices_affected':changed,'clips':[n for n,_ in CLIPS],'method':'Authored procedural motion; female weights transferred from matching male T-pose; female render copy decimated to 19 percent.'},indent=2))
print('COMBAT ASSETS READY',changed)
