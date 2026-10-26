"""Bake combat and wire-fu clips on the artist-rigged, eye-equipped bases.
Run with blender -b --python tools/prepare_combat_characters.py.
Original source models and editable eyed Blends are preserved.
"""
import bpy, math, json, struct
from pathlib import Path
from mathutils import Vector, Quaternion, Matrix
ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'assets/characters';OUT.mkdir(exist_ok=True)
CLIPS=[('Idle',1.6),('Walk',.9),('Run',.6),('Attack',.65),('Throw',1.1),('Hit',.38),('Death',1.1),('AttackQuick',.42),('AttackHeavy',.95),('AttackDesperate',.60),('ThrowUnderhand',.85),('HitHeavy',.70),('LimbRecoil',.85),('DeathBackward',1.35),('DeathDelayed',1.90),('NearDeathPanic',1.4),('WoundedStagger',1.1)]
CLIPS += [('WireLunge',.82),('WhirlwindCut',1.05),('VaultStrike',1.10),('WireRecoil',.85),('WireDeath',1.45)]
def clean():bpy.ops.wm.read_factory_settings(use_empty=True)
def rot(rig,name,axis,angle):
 b=rig.pose.bones.get('mixamorig'+name)
 if not b:return
 basis=b.bone.matrix_local.to_quaternion();b.rotation_mode='QUATERNION'
 b.rotation_quaternion=b.rotation_quaternion @ (basis.inverted()@Quaternion(axis,angle)@basis)
def ease(x):
 x=max(0,min(1,x));return x*x*(3-2*x)
def pulse(u,peak=.35):
 return ease(u/peak) if u<peak else 1-ease((u-peak)/(1-peak))
def root_offset(rig,height=0.,forward=0.,side=0.):
 hips=rig.pose.bones['mixamorigHips']
 hips.location=hips.bone.matrix_local.to_quaternion().inverted()@Vector((side,-forward,height))
def aim_bone(rig,bone,direction):
 # Rotate the current bone in armature space, then recover its local basis.
 # This retains the authored roll instead of guessing an Euler bend axis.
 bpy.context.view_layer.update()
 current=bone.matrix.to_quaternion()
 wanted=(current@Vector((0,1,0))).rotation_difference(direction.normalized())@current
 parent=bone.parent.matrix@bone.parent.bone.matrix_local.inverted()@bone.bone.matrix_local
 bone.rotation_quaternion=parent.to_quaternion().inverted()@wanted
 bpy.context.view_layer.update()
def guard_hand(rig,side,sign):
 upper=rig.pose.bones['mixamorig'+side+'Arm']
 lower=rig.pose.bones['mixamorig'+side+'ForeArm']
 hand=rig.pose.bones['mixamorig'+side+'Hand']
 bpy.context.view_layer.update()
 shoulder=upper.head.copy()
 goal=Vector((sign*.25,-.24,shoulder.z-.20))
 a=upper.length;b=lower.length;delta=goal-shoulder
 distance=max(abs(a-b)+.001,min(delta.length,(a+b)*.985))
 axis=delta.normalized();hint=Vector((sign,-.05,-.6))
 bend=(hint-axis*hint.dot(axis)).normalized()
 along=(a*a-b*b+distance*distance)/(2*distance)
 elbow=shoulder+axis*along+bend*math.sqrt(max(0,a*a-along*along))
 aim_bone(rig,upper,elbow-shoulder)
 aim_bone(rig,lower,shoulder+axis*distance-elbow)
def orient_grip(rig,hand,sign):
 bpy.context.view_layer.update()
 current=hand.matrix.to_quaternion()
 blade=current@Vector((sign,0,0))
 # Keep the blade outside the oversized head silhouette on both bodies.
 wanted=blade.rotation_difference(Vector((sign*.30,-.15,1.)).normalized())@current
 parent=hand.parent.matrix@hand.parent.bone.matrix_local.inverted()@hand.bone.matrix_local
 hand.rotation_quaternion=parent.to_quaternion().inverted()@wanted
def fighting_stance(rig):
 # A bent-elbow guard, a split stance, and a closed index/thumb grip. The
 # imported rigs use these chains to deform their grouped fingers as a fist.
 for side,sign in [('Left',1),('Right',-1)]:
  rot(rig,side+'Arm',(0,1,0),sign*1.10)
  rot(rig,side+'Arm',(1,0,0),-.35 if side=='Right' else -.55)
  rot(rig,side+'ForeArm',(1,0,0),-.90 if side=='Right' else -1.05)
  rot(rig,side+'UpLeg',(0,1,0),sign*.10)
  rot(rig,side+'UpLeg',(1,0,0),sign*.12)
  rot(rig,side+'Leg',(1,0,0),.16)
  for joint,angle in [(1,-.75),(2,-1.25),(3,-.85)]:
   bone=rig.pose.bones.get('mixamorig'+side+'HandIndex'+str(joint))
   if bone:bone.rotation_quaternion=Quaternion((1,0,0),angle)
  for joint,angle in [(1,-.18),(2,-.42),(3,-.48)]:
   bone=rig.pose.bones.get('mixamorig'+side+'HandThumb'+str(joint))
   if bone:bone.rotation_quaternion=Quaternion((1,0,0),angle)
 rot(rig,'Spine2',(0,0,1),-.12)
 root_offset(rig,-.025)
 for side,sign in [('Left',1),('Right',-1)]:
  guard_hand(rig,side,sign)
  hand=rig.pose.bones['mixamorig'+side+'Hand']
  hand.rotation_quaternion=hand.rotation_quaternion@Quaternion((0,1,0),sign*math.pi/2)
  orient_grip(rig,hand,sign)
def wire_pose(rig,name,u):
 arc=math.sin(math.pi*u)**.85
 strike=pulse(u,.46)
 if name=='WireLunge':
  root_offset(rig,.95*arc,.24*arc)
  rot(rig,'Spine2',(0,0,1),-.65*strike)
  rot(rig,'RightArm',(1,0,0),-1.25*pulse(u,.25)+1.0*ease((u-.30)/.22)*(1-ease((u-.7)/.3)))
  rot(rig,'RightArm',(0,0,1),-.75*strike)
  rot(rig,'LeftArm',(1,0,0),-.45*arc)
  rot(rig,'RightUpLeg',(1,0,0),-.85*arc)
  rot(rig,'LeftUpLeg',(1,0,0),.55*arc)
  rot(rig,'LeftLeg',(1,0,0),.85*arc)
 elif name=='WhirlwindCut':
  root_offset(rig,.48*arc)
  rot(rig,'Hips',(0,0,1),-math.tau*ease(u))
  rot(rig,'RightArm',(0,1,0),.78*arc)
  rot(rig,'RightForeArm',(1,0,0),.60*arc)
  rot(rig,'LeftArm',(0,1,0),-.50*arc)
  rot(rig,'Spine2',(0,0,1),-.45*math.sin(math.pi*u))
  rot(rig,'LeftUpLeg',(1,0,0),.72*arc)
  rot(rig,'RightLeg',(1,0,0),.45*arc)
 elif name=='VaultStrike':
  root_offset(rig,1.8*arc,.15*arc)
  rot(rig,'Hips',(1,0,0),-math.tau*ease(u))
  rot(rig,'RightUpLeg',(1,0,0),-1.10*arc)
  rot(rig,'RightLeg',(1,0,0),1.1*arc)
  rot(rig,'LeftUpLeg',(1,0,0),.75*arc)
  rot(rig,'LeftLeg',(1,0,0),.6*arc)
  rot(rig,'RightArm',(1,0,0),-1.10*arc)
  rot(rig,'LeftArm',(0,1,0),-.7*arc)
 elif name=='WireRecoil':
  root_offset(rig,.75*arc,-.30*arc)
  rot(rig,'Spine',(1,0,0),.65*arc)
  rot(rig,'Head',(1,0,0),.35*arc)
  rot(rig,'LeftArm',(0,1,0),-.65*arc)
  rot(rig,'RightArm',(0,1,0),.65*arc)
  rot(rig,'LeftUpLeg',(1,0,0),-.65*arc)
  rot(rig,'RightLeg',(1,0,0),.70*arc)
 elif name=='WireDeath':
  fall=ease((u-.35)/.6)
  root_offset(rig,1.2*arc-.65*fall,-.28*arc)
  rot(rig,'Hips',(1,0,0),1.48*fall)
  rot(rig,'Hips',(0,0,1),.9*fall)
  rot(rig,'LeftArm',(0,1,0),-.70*arc)
  rot(rig,'RightArm',(0,1,0),.70*arc)
  rot(rig,'LeftLeg',(1,0,0),.75*arc)
  rot(rig,'RightUpLeg',(1,0,0),-.55*arc)
def varied_pose(rig,name,u):
 hips=rig.pose.bones['mixamorigHips']
 if name=='AttackQuick':
  hit=pulse(u,.32)
  rot(rig,'Spine2',(0,0,1),-.32*hit);rot(rig,'RightArm',(1,0,0),-1.8*hit)
  rot(rig,'RightForeArm',(1,0,0),-.55*hit);rot(rig,'LeftArm',(1,0,0),-.30*hit)
 elif name=='AttackHeavy':
  wind=math.sin(math.pi*min(1,u/.65))**2
  strike=ease((u-.44)/.12)*(1-ease((u-.65)/.35))
  rot(rig,'Spine2',(0,0,1),.48*wind-.65*strike)
  rot(rig,'Spine',(1,0,0),-.15*wind+.30*strike)
  rot(rig,'RightArm',(1,0,0),-2.5*wind+1.1*strike)
  rot(rig,'RightForeArm',(1,0,0),-.9*wind)
  rot(rig,'LeftArm',(1,0,0),-.7*wind)
  rot(rig,'LeftLeg',(1,0,0),.25*wind);hips.location.y=-.035*wind
 elif name=='AttackDesperate':
  flail=math.sin(u*math.tau*2)*math.sin(math.pi*u)
  hit=pulse(u,.4)
  rot(rig,'Spine2',(0,0,1),flail*.42)
  rot(rig,'RightArm',(1,0,0),-hit*1.4+flail*.55)
  rot(rig,'LeftArm',(1,0,0),-hit*.9-flail*.6)
  rot(rig,'Head',(0,0,1),flail*.18)
  hips.location.y=-hit*.055
 elif name=='ThrowUnderhand':
  wind=pulse(u,.42);release=ease((u-.38)/.20)*(1-ease((u-.7)/.3))
  rot(rig,'RightArm',(1,0,0),.65*wind-1.45*release)
  rot(rig,'RightForeArm',(1,0,0),.15*wind-.65*release)
  rot(rig,'Spine',(1,0,0),.18*wind-.15*release)
  rot(rig,'LeftArm',(1,0,0),-.65*wind)
 elif name in ['HitHeavy','LimbRecoil']:
  recoil=pulse(u,.23 if name=='HitHeavy' else .18)
  rot(rig,'Spine',(1,0,0),(.48 if name=='HitHeavy' else .25)*recoil)
  rot(rig,'Spine2',(0,0,1),(.18 if name=='HitHeavy' else -.42)*recoil)
  rot(rig,'Head',(1,0,0),.24*recoil)
  rot(rig,'Head',(0,0,1),-.20*recoil)
  for side in ['Left','Right']:
   rot(rig,side+'ForeArm',(1,0,0),-1.15*recoil)
   rot(rig,side+'Leg',(1,0,0),.25*recoil)
  hips.location.y=-.075*recoil
 elif name=='DeathBackward':
  fall=ease(u/.80)
  rot(rig,'Hips',(1,0,0),1.48*fall);hips.location.y=-.62*fall
  rot(rig,'LeftArm',(1,0,0),-.65*math.sin(math.pi*min(1,u/.8)))
  rot(rig,'RightArm',(1,0,0),-.85*math.sin(math.pi*min(1,u/.8)))
  rot(rig,'LeftLeg',(1,0,0),.4*fall);rot(rig,'Head',(1,0,0),-.2*fall)
 elif name=='DeathDelayed':
  fall=ease((u-.42)/.43);wobble=math.sin(u*math.tau*3)*(1-fall)*min(1,u/.18)
  rot(rig,'Head',(0,0,1),wobble*.22)
  rot(rig,'Spine2',(0,0,1),wobble*.12)
  rot(rig,'Hips',(1,0,0),-.70*fall);rot(rig,'Hips',(0,1,0),1.2*fall)
  hips.location.y=-.64*fall
  rot(rig,'RightForeArm',(1,0,0),-.6*(1-fall)*math.sin(math.pi*u))
 elif name in ['NearDeathPanic','WoundedStagger']:
  wave=math.sin(u*math.tau);shake=math.sin(u*math.tau*5)
  moving=name=='WoundedStagger'
  # Crouch, clutch the abdomen and look frantically for help. No root travel:
  # locomotion and collision remain owned by the authoritative simulation.
  rot(rig,'Spine',(1,0,0),-.24-.05*wave)
  rot(rig,'Spine2',(0,0,1),.045*shake)
  rot(rig,'Head',(0,0,1),.30*math.sin(u*math.tau*2))
  rot(rig,'Head',(1,0,0),-.10+.035*shake)
  for side,sign in [('Left',1),('Right',-1)]:
   rot(rig,side+'Arm',(1,0,0),-.65+sign*.12*wave)
   rot(rig,side+'ForeArm',(1,0,0),-1.1+sign*.10*shake)
   rot(rig,side+'UpLeg',(1,0,0),(.24*wave*sign if moving else .04*wave))
   rot(rig,side+'Leg',(1,0,0),.24+(.25*max(0,-wave*sign) if moving else .035*shake))
  hips.location.y=-.09+.025*wave

def bake(rig):
 rig.data.pose_position="POSE"
 for name,duration in CLIPS:
  action=bpy.data.actions.new(name);rig.animation_data_create();rig.animation_data.action=action
  frames=round(duration*30)
  for frame in range(frames+1):
   u=frame/frames;phase=u*math.tau;wave=math.sin(phase)
   for b in rig.pose.bones:b.rotation_mode='QUATERNION';b.rotation_quaternion=Quaternion();b.location=Vector()
   fighting_stance(rig)
   if name in [n for n,_ in CLIPS[17:]]:
    wire_pose(rig,name,u)
   elif name in [n for n,_ in CLIPS[7:17]]:
    varied_pose(rig,name,u)
   elif name in ['Walk','Run']:
    stride=.46 if name=='Walk' else .78
    for side,sign in [('Left',1),('Right',-1)]:
     rot(rig,side+'UpLeg',(1,0,0),wave*sign*stride)
     rot(rig,side+'Leg',(1,0,0),max(0,-wave*sign)*stride*.85)
     rot(rig,side+'Arm',(1,0,0),-wave*sign*stride*.65)
    rig.pose.bones['mixamorigHips'].location.z=abs(wave)*.018
    rot(rig,'Spine',(1,0,0),-.08 if name=='Run' else -.025)
   elif name in ['Attack','Throw']:
    power=math.sin(math.pi*u)**2
    rot(rig,'Spine2',(0,0,1),power*.70)
    rot(rig,'RightArm',(1,0,0),-power*(1.6 if name=='Throw' else .75))
    rot(rig,'RightForeArm',(1,0,0),-math.sin(math.pi*u)*.8)
    rot(rig,'RightArm',(0,0,1),-.85*power if name=='Attack' else 0.)
    rot(rig,'LeftArm',(1,0,0),-.3*power)
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
 bpy.ops.object.select_all(action='DESELECT')
 for obj in bpy.context.scene.objects:
  if obj.type in {'MESH','ARMATURE'}:obj.select_set(True)
 bpy.context.view_layer.objects.active=rig
 bpy.ops.export_scene.gltf(filepath=str(OUT/(sex+'.glb')),export_format='GLB',use_selection=True,export_animations=True,export_animation_mode='NLA_TRACKS',export_nla_strips=True,export_frame_range=False,export_force_sampling=True,export_skins=True,export_morph_animation=False)
 # NLA export order is not a stable clip API. Write the runtime's named order.
 path=OUT/(sex+'.glb');raw=path.read_bytes();length=struct.unpack_from('<I',raw,12)[0]
 document=json.loads(raw[20:20+length]);clips={c['name']:c for c in document['animations']}
 document['animations']=[clips[name] for name,_ in CLIPS]
 payload=json.dumps(document,separators=(',',':')).encode();payload+=b' '*((-len(payload))%4)
 remainder=raw[20+length:];total=12+8+len(payload)+len(remainder)
 path.write_bytes(struct.pack('<III',0x46546c67,2,total)+struct.pack('<I',len(payload))+b'JSON'+payload+remainder)

for sex in ['male', 'female']:
 clean()
 bpy.ops.wm.open_mainfile(filepath=str(OUT/(sex+'-facial-rig.blend')))
 rig=next(o for o in bpy.context.scene.objects if o.type=='ARMATURE')
 for obj in bpy.context.scene.objects:
  if obj.type=='MESH' and obj.data.shape_keys:
   for key in obj.data.shape_keys.key_blocks:key.value=0.0
 body=max((o for o in bpy.context.scene.objects if o.type=='MESH'),key=lambda o:len(o.data.vertices))
 # Normalize centered studio bases to the runtime's feet-at-zero coordinates.
 # Bake the body world transform into vertices; move skeleton rest coordinates
 # by the same offset. Bone-parented eyes follow that rest-space translation.
 offset=Vector((0,0,1.0006 if sex=='male' else .985))
 points=[body.matrix_world@v.co+offset for v in body.data.vertices]
 body.matrix_world=Matrix.Identity(4)
 for vertex,point in zip(body.data.vertices,points):vertex.co=point
 bpy.context.view_layer.objects.active=rig
 bpy.ops.object.mode_set(mode='EDIT')
 for bone in rig.data.edit_bones:
  bone.head+=offset;bone.tail+=offset
 bpy.ops.object.mode_set(mode='OBJECT')
 bpy.context.view_layer.update()
 for poly in body.data.polygons:poly.use_smooth=True
 body.data.normals_split_custom_set([(0,0,0)]*len(body.data.loops))
 bake(rig)
 bpy.context.preferences.filepaths.save_version=0
 bpy.ops.wm.save_as_mainfile(filepath=str(OUT/(sex+'-combat.blend')))
 export(sex,body,rig)
(OUT/'manifest.json').write_text(json.dumps({'source_models_preserved':True,'clips':[n for n,_ in CLIPS],'method':'Artist-rigged new bases, independent bone-parented eyes; authored procedural combat motion. Feet normalized to ground.'},indent=2)+'\n')
