"""Build an editable quadruped/eye rig from the preserved authored war-hound GLB.
Run with Blender --background --factory-startup --python tools/rig_war_hound.py.
"""
import bpy
import bmesh
import json
import math
from pathlib import Path
from mathutils import Vector, Quaternion, Matrix

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'outputs/war-hound'
OUT.mkdir(parents=True, exist_ok=True)
bpy.ops.object.select_all(action='SELECT')
bpy.ops.object.delete(use_global=False)
bpy.ops.import_scene.gltf(filepath=str(ROOT/'assets/characters/source/War_Hound.original.glb'))
body = next(o for o in bpy.context.scene.objects if o.type == 'MESH')
world = body.matrix_world.copy()
body.parent = None
body.matrix_world = Matrix.Identity(4)
body.data.transform(world)
body.data.update()
body.name = 'WarHound_Body'
for o in list(bpy.context.scene.objects):
    if o.type == 'EMPTY':
        bpy.data.objects.remove(o, do_unlink=True)


def drool(p):
    # Reviewed thin hanging strands behind the broad tongue, in authored metres.
    x,y,z=p
    def tube(a,b,r):
        a,b=Vector(a),Vector(b);q=Vector(p);d=b-a
        t=max(0.,min(1.,(q-a).dot(d)/d.length_squared))
        return (q-(a+d*t)).length<r
    left=tube((.012,-.685,-.075),(.005,-.685,-.40),.027)
    right=tube((.225,-.675,-.105),(.125,-.680,-.49),.032)
    return left or right


mesh = body.data
before_faces = len(mesh.polygons)
bm = bmesh.new()
bm.from_mesh(mesh)
selected = [f for f in bm.faces if drool(f.calc_center_median())]
bmesh.ops.delete(bm, geom=selected, context='FACES')
bmesh.ops.delete(bm, geom=[v for v in bm.verts if not v.link_faces], context='VERTS')
bm.to_mesh(mesh)
bm.free()
mesh.update()


def material(name, color, roughness=.6, metal=0.):
    m=bpy.data.materials.new(name)
    m.diffuse_color=(*color,1)
    m.use_nodes=True
    p=m.node_tree.nodes.get('Principled BSDF')
    p.inputs['Base Color'].default_value=(*color,1)
    p.inputs['Roughness'].default_value=roughness
    p.inputs['Metallic'].default_value=metal
    return m


ivory=material('Hound ivory sclera',(.70,.65,.48),.35)
iris=material('Hound amber iris',(.62,.27,.025),.3,.08)
pupil=material('Hound dark pupils',(.004,.008,.009),.26)
glint=material('Hound catchlights',(.94,.95,.81),.18)
tongue=material('Hound living tongue',(.34,.065,.075),.48)
inner=material('Hound mouth interior',(.045,.012,.02),.8)
# Restore a concept-inspired palette; the supplied image is an all-white placeholder.
# Color is authored on geometry, then baked through the preserved UVs for portable glTF.
coat=material('Hound concept palette',(.065,.031,.052),.69)
body.data.materials.clear();body.data.materials.append(coat)
colors=mesh.color_attributes.new(name='HoundPalette',type='FLOAT_COLOR',domain='CORNER')
for f in mesh.polygons:
    for li in f.loop_indices:
        v=mesh.vertices[mesh.loops[li].vertex_index];x,y,z=v.co
        color=(.074,.037,.061)  # wine-charcoal flesh, in linear light
        skull=y<-.48 and -.22<z<.57
        crown=z>.53 and abs(x-.09)<.105 and y<-.30
        spine=y>-.26 and z>(.13+.06*math.sin(y*13)) and abs(x)<.28
        claw=z<-.855 and ((y<-.58) or (.11<y<.44))
        if skull or crown or spine or claw:color=(.56,.38,.25)
        # Ear membranes remain flesh; only the central crown is bone.
        if z>.57 and not crown:
            color=(.30,.055,.065) if v.normal.y<-.25 else (.07,.032,.055)
        # Dark muzzle nose above the teeth.
        if y<-.835 and -.04<z<.20 and abs(x-.095)<.21:color=(.026,.012,.021)
        if -.015<x<.21 and y<-.735 and -.40<z<-.10:color=(.38,.045,.055)
        elif -.015<x<.20 and -.68<y<-.50 and -.22<z<.015:color=(.025,.004,.012)
        # Sculpted narrow grooves become dark, retaining the actual mesh detail.
        colors.data[li].color=(*color,1)
nodes=coat.node_tree.nodes;links=coat.node_tree.links
pbr=nodes.get('Principled BSDF');out=nodes.get('Material Output')
attr=nodes.new('ShaderNodeVertexColor');attr.layer_name='HoundPalette'
noise=nodes.new('ShaderNodeTexNoise');noise.inputs['Scale'].default_value=65.;noise.inputs['Detail'].default_value=3.
ramp=nodes.new('ShaderNodeValToRGB');ramp.color_ramp.elements[0].color=(.50,.42,.40,1);ramp.color_ramp.elements[1].color=(1.,.95,.9,1)
links.new(noise.outputs['Fac'],ramp.inputs[0])
mul=nodes.new('ShaderNodeMixRGB');mul.blend_type='MULTIPLY';mul.inputs[0].default_value=.55
links.new(attr.outputs['Color'],mul.inputs[1]);links.new(ramp.outputs[0],mul.inputs[2])
# Bake only reflectance, never the inspection lights or ambient occlusion.
emission=nodes.new('ShaderNodeEmission');links.new(mul.outputs[0],emission.inputs['Color']);links.new(emission.outputs[0],out.inputs['Surface'])
baked=bpy.data.images.new('WarHound_BaseColor',width=2048,height=2048,alpha=False)
tex=nodes.new('ShaderNodeTexImage');tex.image=baked;nodes.active=tex
for obj in bpy.context.selected_objects:obj.select_set(False)
body.select_set(True);bpy.context.view_layer.objects.active=body
scene=bpy.context.scene;scene.render.engine='CYCLES';scene.cycles.device='CPU';scene.cycles.samples=1
scene.render.bake.margin=12
bpy.ops.object.bake(type='EMIT')
baked.filepath_raw=str(ROOT/'assets/characters/war-hound-basecolor.png');baked.file_format='PNG';baked.save();baked.pack()
links.new(tex.outputs['Color'],pbr.inputs['Base Color']);links.new(pbr.outputs[0],out.inputs['Surface'])
# Store the authoring palette alongside the baked export, for later art refinement.
coat.diffuse_color=(.074,.037,.061,1)

bpy.ops.object.armature_add(enter_editmode=True)
rig=bpy.context.object
rig.name='WarHound_Rig'
rig.data.name='WarHound_Skeleton'
rig.data.edit_bones.remove(rig.data.edit_bones[0])
spec={}


def bone(name,head,tail,parent=None,deform=True):
    b=rig.data.edit_bones.new(name)
    b.head=head;b.tail=tail;b.use_deform=deform
    if parent:b.parent=rig.data.edit_bones[parent]
    spec[name]=(Vector(head),Vector(tail))
    return b


bone('Root',(0,0,-.982),(0,0,-.78),deform=False)
bone('Pelvis',(0,.38,-.12),(0,.16,.02),'Root')
bone('Spine',(0,.16,.02),(0,-.10,.08),'Pelvis')
bone('Chest',(0,-.10,.08),(.04,-.30,.20),'Spine')
bone('Neck',(.04,-.30,.20),(.085,-.48,.35),'Chest')
bone('Head',(.085,-.48,.35),(.095,-.78,.34),'Neck')
bone('Jaw',(.09,-.56,-.04),(.095,-.78,-.17),'Head')
bone('Tongue.01',(.095,-.74,-.12),(.105,-.81,-.26),'Jaw')
bone('Tongue.02',(.105,-.81,-.26),(.11,-.77,-.39),'Tongue.01')
bone('Tail.01',(0,.40,.07),(0,.64,-.07),'Pelvis')
bone('Tail.02',(0,.64,-.07),(0,.83,-.29),'Tail.01')
bone('Tail.03',(0,.83,-.29),(0,.91,-.56),'Tail.02')
for side,sign in [('L',1),('R',-1)]:
    bone(f'ForeUpper.{side}',(sign*.25,-.20,-.06),(sign*.30,-.31,-.48),'Chest')
    bone(f'ForeLower.{side}',(sign*.30,-.31,-.48),(sign*.34,-.45,-.84),f'ForeUpper.{side}')
    bone(f'ForeFoot.{side}',(sign*.34,-.45,-.84),(sign*.35,-.63,-.94),f'ForeLower.{side}')
    bone(f'HindUpper.{side}',(sign*.28,.36,-.16),(sign*.37,.34,-.49),'Pelvis')
    bone(f'HindLower.{side}',(sign*.37,.34,-.49),(sign*.39,.60,-.83),f'HindUpper.{side}')
    bone(f'HindFoot.{side}',(sign*.39,.60,-.83),(sign*.40,.44,-.94),f'HindLower.{side}')
# The generated skull is deliberately asymmetric: fit each actual cavity.
eyes={'L':(Vector((.305,-.695,.345)),.126),'R':(Vector((-.112,-.692,.344)),.119)}
for side,(center,radius) in eyes.items():
    bone(f'Eye.{side}',center,center+Vector((0,-.08,0)),'Head')
    bone(f'Ear.{side}',(center.x,-.42,.63),(center.x+(.10 if side=='L' else -.10),-.33,.90),'Head')
bpy.ops.object.mode_set(mode='OBJECT')


def capsule_distance(p,a,b):
    d=b-a
    t=max(0.,min(1.,(p-a).dot(d)/max(d.length_squared,1e-9)))
    return (p-(a+d*t)).length


def mixweights(p,names):
    values=sorted(((1./(.025+capsule_distance(p,*spec[n]))**3,n) for n in names),reverse=True)[:3]
    total=sum(v for v,_ in values)
    return {n:v/total for v,n in values}


def weights(p):
    x,y,z=p
    if y<-.55 and z<-.09 and z>-.55 and -.08<x<.30:
        if -.015<x<.21 and y<-.735 and z<-.12:
            return mixweights(p,['Jaw','Tongue.01','Tongue.02'])
        if not(y<-.78 and z>-.14):
            return {'Jaw':1.}
    if z<-.30 and abs(x)>.13:
        side='L' if x>0 else 'R'
        part='Fore' if y<.10 else 'Hind'
        base=mixweights(p,[f'{part}Upper.{side}',f'{part}Lower.{side}',f'{part}Foot.{side}'])
        blend=max(0.,min(1.,(-z-.30)/.19))
        if blend<1.:
            trunk='Chest' if part=='Fore' else 'Pelvis'
            base={n:w*blend for n,w in base.items()};base[trunk]=base.get(trunk,0.)+1.-blend
        return base
    if y>.56 and z>-.80:
        return mixweights(p,['Tail.01','Tail.02','Tail.03'])
    if y<-.37 and z>.04:
        if z>.62:
            side='L' if x>.09 else 'R'
            ear=min(.7,max(0.,(z-.65)/.28))
            return {'Head':1.-ear,f'Ear.{side}':ear}
        return {'Head':1.}
    return mixweights(p,['Pelvis','Spine','Chest','Neck','Head'])


def bind(obj, assignments):
    obj.parent=rig
    mod=obj.modifiers.new('WarHound skin','ARMATURE');mod.object=rig
    groups={n:obj.vertex_groups.new(name=n) for n in spec if n!='Root'}
    for i,ws in assignments:
        total=sum(ws.values())
        for name,w in ws.items():
            if w>1e-7:groups[name].add([i],w/total,'REPLACE')


bind(body,[(v.index,weights(v.co)) for v in mesh.vertices])
parts={}


def sphere(name,at,scale,mat,bone_name):
    bpy.ops.mesh.primitive_uv_sphere_add(segments=28,ring_count=16,location=at)
    obj=bpy.context.object;obj.name=name;obj.scale=scale
    obj.data.materials.append(mat)
    for f in obj.data.polygons:f.use_smooth=True
    bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
    bind(obj,[(v.index,{bone_name:1.}) for v in obj.data.vertices])
    return obj


for side,(c,r) in eyes.items():
    n=f'Eye.{side}'
    parts[f'eye_{side}']=sphere(f'Eyeball_{side}',c,(r,r,r),ivory,n)
    # Iris, pupil and glint share the eye pivot. They never slide over the face.
    parts[f'iris_{side}']=sphere(f'Iris_{side}',c+Vector((0,-r*.94,0)),(r*.63,r*.15,r*.68),iris,n)
    parts[f'pupil_{side}']=sphere(f'Pupil_{side}',c+Vector((0,-r*1.055,0)),(r*.14,r*.07,r*.49),pupil,n)
    sphere(f'Glint_{side}',c+Vector((-r*.20,-r*1.105,r*.24)),(r*.065,r*.025,r*.065),glint,n)

# A single parent normalizes the authored mesh to game metres and seats the paws.
normalizer=bpy.data.objects.new('HoundOrigin',None)
bpy.context.collection.objects.link(normalizer)
GAME_SCALE=.25  # Pug-sized: approximately 49 cm including the oversized ears.
normalizer.location=(0,0,.9823533893*GAME_SCALE)
normalizer.scale=(GAME_SCALE,)*3
rig.parent=normalizer
rig.show_in_front=True
rig.data.display_type='OCTAHEDRAL'
scene=bpy.context.scene
scene.render.fps=30


def neutral():
    for b in rig.pose.bones:
        b.rotation_mode='QUATERNION';b.rotation_quaternion=Quaternion();b.location=Vector();b.scale=Vector((1,1,1))
    bpy.context.view_layer.update()


def eval_points(obj):
    e=obj.evaluated_get(bpy.context.evaluated_depsgraph_get())
    return [e.matrix_world@v.co for v in e.data.vertices]


neutral()
base=eval_points(body)
rig.pose.bones['ForeUpper.L'].rotation_quaternion=Quaternion((1,0,0),.25)
bpy.context.view_layer.update()
posed=eval_points(body)
deltas=[(a-b).length for a,b in zip(base,posed)]
assert sum(d>.001 for d in deltas)>60,'leg bone does not deform the mesh'
assert max(deltas)<.35,'unbounded deformation'
neutral()
right_before=eval_points(parts['pupil_R'])
left_before=eval_points(parts['pupil_L'])
rig.pose.bones['Eye.L'].rotation_quaternion=Quaternion((0,0,1),.22)
bpy.context.view_layer.update()
left_after=eval_points(parts['pupil_L']);right_after=eval_points(parts['pupil_R'])
left_motion=max((a-b).length for a,b in zip(left_before,left_after))
right_motion=max((a-b).length for a,b in zip(right_before,right_after))
assert left_motion>.002 and right_motion<1e-6,'eyes are not independently bound'
neutral()
zero_weights=sum(not v.groups for v in body.data.vertices)
max_weight_error=max(abs(sum(g.weight for g in v.groups)-1.) for v in body.data.vertices)
assert zero_weights==0 and max_weight_error<1e-5


def orient(name, start, end, rotation=None):
    """Place a deform bone in armature space, retaining its authored roll."""
    b=rig.pose.bones[name]
    rest=b.bone.matrix_local.to_quaternion()
    q=rotation or ((spec[name][1]-spec[name][0]).rotation_difference(end-start) @ rest)
    b.matrix=Matrix.Translation(start) @ q.to_matrix().to_4x4()
    bpy.context.view_layer.update()


def solve_leg(part, side, goal):
    """Two-segment leg IK with a stable anatomical knee plane; baked to the skin."""
    upper=f'{part}Upper.{side}';lower=f'{part}Lower.{side}';foot=f'{part}Foot.{side}'
    a=rig.pose.bones[upper].head.copy()
    l1=(spec[upper][1]-spec[upper][0]).length
    l2=(spec[lower][1]-spec[lower][0]).length
    direction=goal-a;distance=direction.length
    direction.normalize()
    d=max(abs(l1-l2)+.001,min(distance,l1+l2-.002))
    along=(l1*l1-l2*l2+d*d)/(2*d)
    rest_axis=(spec[foot][0]-spec[upper][0]).normalized()
    rest_bend=spec[upper][1]-spec[upper][0]
    rest_bend-=rest_axis*rest_bend.dot(rest_axis)
    bend=rest_axis.rotation_difference(direction) @ rest_bend.normalized()
    bend-=direction*bend.dot(direction)
    bend.normalize()
    knee=a+direction*along+bend*math.sqrt(max(0,l1*l1-along*along))
    ankle=a+direction*d
    orient(upper,a,knee)
    orient(lower,knee,ankle)
    # Paw orientation stays level through stance instead of rotating with the shin.
    orient(foot,ankle,ankle+(spec[foot][1]-spec[foot][0]),rig.data.bones[foot].matrix_local.to_quaternion())
    return (ankle-goal).length


def local_translate(name, world_delta):
    b=rig.pose.bones[name]
    b.location=b.bone.matrix_local.to_3x3().inverted() @ Vector(world_delta)


def create_action(name,duration):
    neutral()
    rig.animation_data_create()
    action=bpy.data.actions.new(name)
    rig.animation_data.action=action
    frames=int(duration*30)
    for frame in range(frames+1):
        neutral();t=frame/frames;phase=2*math.pi*t
        gait=name in {'Walk','Run','Recall'}
        if gait:
            run=name!='Walk';bound=name=='Recall'
            stance=.65 if not run else (.40 if bound else .48)
            stride=.60 if not run else (.80 if bound else .70)
            lift=.17 if not run else (.35 if bound else .27)
            bob=(.018 if not run else .045)*math.cos(phase*2)
            local_translate('Pelvis',(0,0,(-.075 if not run else -.10)+bob))
            rig.pose.bones['Pelvis'].rotation_quaternion=Quaternion((0,1,0),math.sin(phase)*(.025 if not run else .035))
            rig.pose.bones['Spine'].rotation_quaternion=Quaternion((1,0,0),math.sin(phase)*(.015 if not run else .07))
            rig.pose.bones['Chest'].rotation_quaternion=Quaternion((0,1,0),-math.sin(phase)*.018)
            rig.pose.bones['Neck'].rotation_quaternion=Quaternion((1,0,0),-math.sin(phase)*(.012 if not run else .035))
        elif name=='Bite':
            # Crouch -> open -> drive -> snap at the real six-tick bite resolution.
            crouch=math.sin(math.pi*min(1,t/.42)) if t<.42 else 0
            drive=math.exp(-((t-.66)/.17)**2)
            local_translate('Pelvis',(0,-.07*drive,-.06*crouch))
            rig.pose.bones['Spine'].rotation_quaternion=Quaternion((1,0,0),-.08*drive)
            local_translate('Neck',(0,-.08*drive,-.025*crouch))
            rig.pose.bones['Jaw'].rotation_quaternion=Quaternion((1,0,0),.28*math.sin(math.pi*t/.67) if t<.67 else -.15*math.sin(math.pi*(t-.67)/.33))
            rig.pose.bones['Head'].rotation_quaternion=Quaternion((1,0,0),-.10*drive)
        elif name=='Sniff':
            lower=.5-.5*math.cos(phase)
            local_translate('Pelvis',(0,0,-.045*lower))
            rig.pose.bones['Neck'].rotation_quaternion=Quaternion((1,0,0),.23*lower)
            rig.pose.bones['Head'].rotation_quaternion=Quaternion((0,0,1),math.sin(phase*3)*.045*lower)
        elif name=='Down':
            fall=math.sin(math.pi*.5*t)
            rig.pose.bones['Pelvis'].rotation_quaternion=Quaternion((0,1,0),1.15*fall)
            local_translate('Pelvis',(0,0,-.30*fall))
            rig.pose.bones['Head'].rotation_quaternion=Quaternion((0,0,1),-.16*fall)
        else:
            rig.pose.bones['Chest'].scale=Vector((1+math.sin(phase)*.012,1,1+math.sin(phase)*.008))
            if name=='LookAround':
                rig.pose.bones['Neck'].rotation_quaternion=Quaternion((0,0,1),math.sin(phase-.35)*.11)
                rig.pose.bones['Head'].rotation_quaternion=Quaternion((0,0,1),math.sin(phase)*.10) @ Quaternion((1,0,0),math.sin(phase*2)*.025)
        if name!='Down':
            rig.pose.bones['Tail.01'].rotation_quaternion=Quaternion((0,0,1),math.sin(phase)*(.25 if name=='Recall' else .13))
            rig.pose.bones['Tail.02'].rotation_quaternion=Quaternion((0,0,1),math.sin(phase-.5)*.17)
            rig.pose.bones['Tail.03'].rotation_quaternion=Quaternion((0,0,1),math.sin(phase-.9)*.13)
            for side in eyes:
                rig.pose.bones[f'Eye.{side}'].rotation_quaternion=Quaternion((0,0,1),math.sin(phase+.25)*(.16 if name=='LookAround' else .025)) @ Quaternion((1,0,0),math.sin(phase*2)*.025)
                rig.pose.bones[f'Ear.{side}'].rotation_quaternion=Quaternion((0,1,0),math.sin(phase+(.7 if side=='L' else 0))*(.08 if gait else .045))
            rig.pose.bones['Tongue.01'].rotation_quaternion=Quaternion((1,0,0),math.sin(phase*2-.4)*(.07 if gait else .025))
            rig.pose.bones['Tongue.02'].rotation_quaternion=Quaternion((1,0,0),math.sin(phase*2-.9)*(.10 if gait else .04))
            bpy.context.view_layer.update()
            for part in ['Fore','Hind']:
                for side in ['L','R']:
                    goal=spec[f'{part}Foot.{side}'][0].copy()
                    if gait:
                        # Center each step below its shoulder/hip, within limb reach.
                        goal.y+=.20 if part=='Fore' else -.22
                        if name=='Walk':
                            offset={('Fore','L'):0.,('Hind','R'):.25,('Fore','R'):.5,('Hind','L'):.75}[part,side]
                        elif name=='Recall':
                            offset=(0 if part=='Fore' else .52)+(0 if side=='L' else .08)
                        else:
                            offset=(0 if side=='L' else .5)+(0 if part=='Fore' else .5)
                        q=(t+offset)%1
                        if q<stance:
                            goal.y+=stride*(q/stance-.5)
                        else:
                            swing=(q-stance)/(1-stance)
                            smooth=swing*swing*(3-2*swing)
                            goal.y+=stride*(.5-smooth)
                            goal.z+=lift*math.sin(math.pi*swing)**1.3
                    solve_leg(part,side,goal)
        for b in rig.pose.bones:
            b.keyframe_insert('rotation_quaternion',frame=frame)
            b.keyframe_insert('location',frame=frame)
            b.keyframe_insert('scale',frame=frame)
    rig.animation_data.action=None
    track=rig.animation_data.nla_tracks.new();track.name=name
    strip=track.strips.new(name,0,action);strip.action_frame_start=0;strip.action_frame_end=frames
    return action


clips={name:create_action(name,duration) for name,duration in [('Idle',2.4),('Walk',.8),('Run',.5),('Recall',.55),('Bite',.7),('Sniff',2.),('Down',1.1),('LookAround',2.8)]}
for track in rig.animation_data.nla_tracks:track.mute=True
# Validate the baked actions, after keyframe evaluation rather than only the IK solver.
gait_checks={}
for name,duration,stance,stride in [('Walk',.8,.65,.60),('Run',.5,.48,.70),('Recall',.55,.40,.80)]:
    rig.animation_data.action=clips[name];frames=int(duration*30);errors=[]
    for frame in range(frames+1):
        scene.frame_set(frame)
        for part in ['Fore','Hind']:
            for side in ['L','R']:
                offset=({('Fore','L'):0,('Hind','R'):.25,('Fore','R'):.5,('Hind','L'):.75}[part,side] if name=='Walk' else ((0 if part=='Fore' else .52)+(0 if side=='L' else .08)) if name=='Recall' else (0 if side=='L' else .5)+(0 if part=='Fore' else .5))
                q=(frame/frames+offset)%1
                if q>=stance:continue
                n=f'{part}Foot.{side}'
                goal=spec[n][0].copy();goal.y+=(.20 if part=='Fore' else -.22)+stride*(q/stance-.5)
                errors.append((rig.pose.bones[n].head-goal).length*GAME_SCALE)
    gait_checks[name]={'max_stance_ankle_error_m':max(errors),'stance_samples':len(errors)}
assert max(c['max_stance_ankle_error_m'] for c in gait_checks.values())<.015
rig.animation_data.action=None
neutral();scene.frame_set(0)
for obj in bpy.context.scene.objects:obj.select_set(False)
for obj in [body,rig,normalizer,*[o for o in bpy.context.scene.objects if o.type=='MESH' and o!=body]]:obj.select_set(True)
bpy.context.view_layer.objects.active=rig
# Muting prevents the editable file combining independent clips; export temporarily unmutes.
bpy.context.preferences.filepaths.save_version=0
for screen in bpy.data.screens:
    for area in screen.areas:
        if area.type=='VIEW_3D':
            area.spaces.active.shading.color_type='MATERIAL'
            area.spaces.active.region_3d.view_location=Vector((.0125,0,.24))
            area.spaces.active.region_3d.view_distance=.85
            area.spaces.active.region_3d.view_rotation=(Vector((.025,0,.48))-Vector((1.6,-3,1.5))).to_track_quat('-Z','Y')
bpy.ops.wm.save_as_mainfile(filepath=str(ROOT/'assets/characters/war-hound-rig.blend'))
for track in rig.animation_data.nla_tracks:track.mute=False
bpy.ops.export_scene.gltf(filepath=str(ROOT/'assets/characters/war-hound-rigged.glb'),export_format='GLB',use_selection=True,export_animations=True,export_animation_mode='NLA_TRACKS',export_nla_strips=True,export_frame_range=False,export_force_sampling=True,export_skins=True,export_morph_animation=False)
# Animation indices are a runtime contract, independent of exporter track ordering.
import struct
path=ROOT/'assets/characters/war-hound-rigged.glb';raw=path.read_bytes();length=struct.unpack_from('<I',raw,12)[0]
doc=json.loads(raw[20:20+length]);by_name={a['name']:a for a in doc['animations']}
assert set(by_name)==set(clips),list(by_name)
doc['animations']=[by_name[n] for n in clips]
encoded=json.dumps(doc,separators=(',',':')).encode();encoded+=b' '*((-len(encoded))%4)
rest=raw[20+length:];path.write_bytes(struct.pack('<III',0x46546C67,2,20+len(encoded)+len(rest))+struct.pack('<II',len(encoded),0x4E4F534A)+encoded+rest)
for track in rig.animation_data.nla_tracks:track.mute=True
neutral()
report={'source':'War_Hound.original.glb','removed_saliva_faces':before_faces-len(body.data.polygons),'body_vertices':len(body.data.vertices),'bones':len(rig.data.bones),'game_scale':GAME_SCALE,'height_including_ears_m':1.9647067786*GAME_SCALE,'gait':'baked two-bone IK; four-beat walk, diagonal trot, paired recall bound','gait_checks':gait_checks,'basecolor':{'size':[2048,2048],'source':'new concept-inspired procedural palette baked through original UVs; supplied texture was solid white'},'unweighted_vertices':zero_weights,'max_weight_error':max_weight_error,'test_leg_moved_vertices':sum(d>.001 for d in deltas),'test_leg_max_displacement':max(deltas),'left_pupil_motion':left_motion,'opposite_pupil_motion':right_motion,'clips':list(clips),'eye_centers_source':{s:list(c) for s,(c,r) in eyes.items()}}
(OUT/'rig-validation.json').write_text(json.dumps(report,indent=2))
print('RIG_VALIDATION',json.dumps(report))

# Inspection renders use the actual bound mesh, not disconnected duplicate geometry.
scene.render.engine='CYCLES';scene.cycles.device='CPU';scene.cycles.samples=20;scene.cycles.use_denoising=True
scene.world.color=(.12,.12,.12);scene.render.resolution_x=720;scene.render.resolution_y=720;scene.render.resolution_percentage=100
for location,power in [((2,-3,3),430),((-2,-1,2),300),((0,3,3),500)]:
    bpy.ops.object.light_add(type='AREA',location=location);bpy.context.object.data.energy=power;bpy.context.object.data.size=3
bpy.ops.object.camera_add();cam=bpy.context.object;cam.data.type='ORTHO';cam.data.ortho_scale=.825;scene.camera=cam
center=Vector((.0125,0,.24))
for name,loc in [('rig-front',(0,-3,1.0)),('rig-three-quarter',(2,-3,1.6))]:
    cam.location=Vector(loc);cam.rotation_euler=(center-cam.location).to_track_quat('-Z','Y').to_euler();scene.render.filepath=str(OUT/f'{name}.png');bpy.ops.render.render(write_still=True)
rig.animation_data.action=clips['Walk'];scene.frame_set(6)
cam.location=Vector((2,-3,1.3));cam.rotation_euler=(center-cam.location).to_track_quat('-Z','Y').to_euler();scene.render.filepath=str(OUT/'rig-walk.png');bpy.ops.render.render(write_still=True)
rig.animation_data.action=clips['LookAround'];scene.frame_set(21)
cam.data.ortho_scale=.36;center=Vector((.025,-.18,.32));cam.location=Vector((.05,-3,.90));cam.rotation_euler=(center-cam.location).to_track_quat('-Z','Y').to_euler();scene.render.filepath=str(OUT/'rig-eyes.png');bpy.ops.render.render(write_still=True)
rig.animation_data.action=None;neutral();scene.frame_set(0)
