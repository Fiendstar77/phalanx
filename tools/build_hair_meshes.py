"""Ten sculpted, head-fitted 3D hairstyles. Run with Blender --background --python.
Hair-only GLBs, editable sources and a front-view sheet; original heads preserved.
"""
import bpy, json, math
from pathlib import Path
from mathutils import Vector, Quaternion
from mathutils.bvhtree import BVHTree
ROOT=Path(__file__).resolve().parents[1]; OUT=ROOT/'assets/characters/hair'
NAMES=['Short Messy','Buzzed Crop','Swept Back','Bob','Long Straight','High Ponytail','Twin Tails','Curly Fluffy','Mohawk','Warrior Braid']
CENTER=Vector((-.017,-.009,1.60)); TREES=[]; HEAD=[]
FEATURES=['Face_Sculpt','Eyeball_','Iris_','Pupil_','Brow_','Lip_Rim','Mouth_Interior','Teeth_','Tongue','Lid_']
for sex in ['male','female']:
 bpy.ops.wm.open_mainfile(filepath=str(ROOT/f'assets/characters/{sex}-combat.blend'))
 rig=next(o for o in bpy.context.scene.objects if o.type=='ARMATURE');rig.animation_data_clear()
 for bone in rig.pose.bones:bone.rotation_quaternion=Quaternion();bone.location=Vector()
 bpy.context.view_layer.update();head=bpy.data.objects['Face_Sculpt']
 TREES.append(BVHTree.FromPolygons([head.matrix_world@v.co for v in head.data.vertices],[list(p.vertices) for p in head.data.polygons]))
 if sex=='male':
  for o in bpy.context.scene.objects:
   if o.type=='MESH' and any(o.name.startswith(n) for n in FEATURES):HEAD.append((o.name,[tuple(o.matrix_world@v.co) for v in o.data.vertices],[list(p.vertices) for p in o.data.polygons]))
def mat(name,color):
 m=bpy.data.materials.new(name);m.diffuse_color=(*color,1);m.use_nodes=True;p=m.node_tree.nodes.get('Principled BSDF');p.inputs['Base Color'].default_value=(*color,1);p.inputs['Roughness'].default_value=.57;return m
def mesh(name,verts,faces,material):
 d=bpy.data.meshes.new(name);d.from_pydata(verts,[],faces);d.update();o=bpy.data.objects.new(name,d);bpy.context.collection.objects.link(o);d.materials.append(material)
 for p in d.polygons:p.use_smooth=True
 return o
def scalp(theta,phi,clearance=.035):
 direction=Vector((math.sin(theta)*math.cos(phi),math.sin(theta)*math.sin(phi),math.cos(theta)))
 distances=[h[3] for t in TREES if (h:=t.ray_cast(CENTER,direction,1.))[0] is not None]
 radius=max(distances) if distances else 1/math.sqrt((direction.x/.49)**2+(direction.y/.41)**2+(direction.z/.38)**2)
 return CENTER+direction*(radius+clearance)
def lock(name,points,width,material,depth=.45,taper=.75):
 points=[Vector(p) for p in points];verts=[];faces=[];steps=max(20,len(points)*3);sides=10
 def at(t):
  q=t*(len(points)-1);i=min(len(points)-2,int(q));u=q-i;a,b,c,d=points[max(0,i-1)],points[i],points[i+1],points[min(len(points)-1,i+2)]
  return .5*((2*b)+(-a+c)*u+(2*a-5*b+4*c-d)*u*u+(-a+3*b-3*c+d)*u*u*u)
 for i in range(steps+1):
  t=i/steps;p=at(t);tangent=(at(min(1,t+.01))-at(max(0,t-.01))).normalized();side=tangent.cross(Vector((0,1,0))).normalized()
  if side.length<.1:side=tangent.cross(Vector((1,0,0))).normalized()
  normal=tangent.cross(side).normalized();radius=width*(.82+.18*math.sin(math.pi*t))*max(.025,(1-t)**taper)
  for j in range(sides):
   a=math.tau*j/sides;verts.append(p+side*math.cos(a)*radius+normal*math.sin(a)*radius*depth)
 for i in range(steps):
  for j in range(sides):a=i*sides+j;b=i*sides+(j+1)%sides;faces.append((a,b,b+sides,a+sides))
 faces.extend([tuple(reversed(range(sides))),tuple(steps*sides+j for j in range(sides))]);return mesh(name,verts,faces,material)
def cap(material,cropped=False,style=0):
 verts=[];faces=[];rings=18;sides=64
 for i in range(rings+1):
  for j in range(sides):
   phi=math.tau*j/sides;front=max(0,-math.sin(phi));limit=1.83-.77*front
   if style in [1,4,7]:limit+=front*(.16+(.07*math.sin(phi*9)+.03*math.sin(phi*17) if style==1 else .02*math.sin(phi*8)))
   theta=.001+(limit-.001)*i/rings;p=scalp(theta,phi,.014 if cropped else .045)
   if style==3:
    p.z+=.07*front*math.exp(-((theta-.40)/.5)**2)
    p+=(p-CENTER).normalized()*.008*math.sin(p.x*50+theta*2)
   p+=(p-CENTER).normalized()*(.002 if cropped else .012)*math.sin(phi*7+theta*4);verts.append(p)
 for i in range(rings):
  for j in range(sides):a=i*sides+j;b=i*sides+(j+1)%sides;faces.append((a,a+sides,b+sides,b))
 return mesh('Head-fitted continuous scalp',verts,faces,material)
def fringe(colors,messy=False):
 for k in range(5):
  x=(k-2)*.17;lock('Sculpted fringe',[(x*.6,-.15,1.98),(x,-.34,1.93),(x+.035*math.sin(k*3),-.40,1.83),(x+.07,-.39,1.76+(.04*math.sin(k*2) if messy else 0))],.12,colors[k%2],depth=.35)
def curtain(material,length):
 verts=[];faces=[];sides=40;rows=12
 for i in range(rows+1):
  t=i/rows
  for j in range(sides+1):
   phi=math.pi*j/sides;width=.49+.045*math.sin(math.pi*t)-.035*t
   verts.append((CENTER.x+width*math.cos(phi),.43*math.sin(phi)+.035+.025*math.sin(phi*10)*math.sin(math.pi*t),1.83-t*length+.015*math.cos(phi*8)*t))
 for i in range(rows):
  for j in range(sides):a=i*(sides+1)+j;faces.append((a,a+sides+1,a+sides+2,a+1))
 o=mesh('Continuous back and side volume',verts,faces,material);s=o.modifiers.new('Closed hair edge','SOLIDIFY');s.thickness=.035;bpy.context.view_layer.objects.active=o;bpy.ops.object.modifier_apply(modifier=s.name)
HAIR=[];REPORT=[]
for number,name in enumerate(NAMES,1):
 bpy.ops.wm.read_factory_settings(use_empty=True);colors=[mat('Chestnut',(.16,.067,.035)),mat('Raised strands',(.185,.080,.043))];cap(colors[0],number in [2,9],number)
 if number==1:
  for k in range(3):
   phi=k*2.4;p=scalp(.50+k*.10,phi,.025);lock('Messy crown clump',[p,p+Vector((.04*math.cos(phi),.05*math.sin(phi),.065)),p+Vector((.13*math.cos(phi),.10*math.sin(phi),.02))],.13,colors[k%2],depth=.3)
 elif number==3:
  pass # The swept quiff and restrained strand grooves are part of the continuous cap.
 elif number in [4,5]:
  curtain(colors[0],.57 if number==4 else 1.12)
  for side in [-1,1]:lock('Face framing section',[(side*.29,-.26,1.96),(side*.45,-.32,1.80),(side*.50,-.23,1.48),(side*.45,-.12,1.28 if number==4 else .78)],.13,colors[1],depth=.3,taper=.35)
 elif number in [6,7]:
  for side in ([0] if number==6 else [-1,1]):
   x=side*.43
   for k in range(3):
    d=(k-1)*.045;lock('Gathered ponytail',[(x+d,.30,1.96),(x+side*.13+d,.53,1.84),(x+side*.20+d,.56,1.45),(x+side*.11+d,.43,.96)],.13,colors[k%2],depth=.8,taper=.4)
   bpy.ops.mesh.primitive_uv_sphere_add(segments=16,ring_count=8,location=(x,.32,1.93),scale=(.15,.11,.065));bpy.context.object.data.materials.append(colors[1])
 elif number==8:
  for k in range(75):
   phi=k*2.39996323;theta=math.acos(1-(k+.5)/75*1.15)
   if theta>1.72-.56*max(0,-math.sin(phi)):continue
   size=.10+.035*math.sin(k*7)**2;bpy.ops.mesh.primitive_uv_sphere_add(segments=12,ring_count=8,location=scalp(theta,phi,.04),scale=(size,size*.85,size*.8));o=bpy.context.object;o.name='Soft curl cluster';o.rotation_euler=(.2*math.sin(k),.2*math.cos(k),phi);o.data.materials.append(colors[k%2])
   for p in o.data.polygons:p.use_smooth=True
 elif number==9:
  for k in range(7):
   y=-.28+k*.10;z=1.93+.12*math.sin(math.pi*k/6);lock('Broken central crest',[(0,y,z-.06),(.015,y+.02,z+.16),(-.02,y+.06,z+.23)],.085,colors[k%2],depth=.60)
 elif number==10:
  for k in range(3):
   points=[]
   for i in range(30):
    t=i/29;a=t*math.tau*5+k*math.tau/3;points.append((.057*math.cos(a),-.33+.85*t+.025*math.sin(a),1.92+.20*math.sin(math.pi*min(1,t/.65))-.85*max(0,(t-.65)/.35)))
   lock('Interwoven warrior braid',points,.048,colors[k%2],depth=.85,taper=.1)
 bpy.ops.object.select_all(action='SELECT');bpy.context.view_layer.objects.active=next(o for o in bpy.context.scene.objects if o.type=='MESH');bpy.ops.object.join();o=bpy.context.object;o.name=name;bpy.ops.object.transform_apply(location=True,rotation=True,scale=True)
 bpy.context.preferences.filepaths.save_version=0;bpy.ops.wm.save_as_mainfile(filepath=str(OUT/f'{number}-{name.lower().replace(" ","-")}.blend'))
 bpy.ops.export_scene.gltf(filepath=str(OUT/f'{number}.glb'),export_format='GLB',use_selection=True,export_animations=False)
 HAIR.append((name,[tuple(v.co) for v in o.data.vertices],[list(p.vertices) for p in o.data.polygons],[p.material_index for p in o.data.polygons]));REPORT.append({'id':number,'name':name,'vertices':len(o.data.vertices),'faces':len(o.data.polygons)})
# Front-view sheet, fitted on the actual neutral head rather than a generic sphere.
bpy.ops.wm.read_factory_settings(use_empty=True);colors=[mat('Chestnut',(.16,.067,.035)),mat('Raised strands',(.185,.080,.043))];skin=mat('Neutral head fit',(.62,.65,.69));ink=mat('Eye and brow',(.035,.029,.025));white=mat('Eyes',(.88,.89,.90))
for i,(name,verts,faces,slots) in enumerate(HAIR):
 offset=Vector(((i%5-2)*1.72,0,2.6 if i<5 else 0));o=mesh(name,[Vector(p)+offset for p in verts],faces,colors[0]);o.data.materials.append(colors[1])
 for p,slot in zip(o.data.polygons,slots):p.material_index=slot
 for feature,points,polys in HEAD:
  m=white if feature.startswith('Eyeball') else ink if feature.startswith(('Iris','Pupil','Brow','Mouth')) else skin;mesh(feature,[Vector(p)+offset for p in points],polys,m)
 bpy.ops.object.text_add(location=(offset.x,-.5,offset.z+.48),rotation=(math.pi/2,0,0));t=bpy.context.object;t.data.body=f'{i+1}. {name}';t.data.align_x='CENTER';t.data.size=.115;t.data.materials.append(ink)
bpy.ops.object.camera_add(location=(0,-12,2.65),rotation=(math.pi/2,0,0));camera=bpy.context.object;camera.data.type='ORTHO';camera.data.ortho_scale=9.
scene=bpy.context.scene;scene.camera=camera;scene.render.engine='BLENDER_WORKBENCH';scene.display.shading.light='STUDIO';scene.display.shading.color_type='MATERIAL';scene.display.shading.show_shadows=True;scene.display.shading.show_cavity=True;scene.display.shading.cavity_type='BOTH';scene.world=bpy.data.worlds.new('Preview world');scene.world.color=(.85,.85,.85)
scene.render.resolution_x=1800;scene.render.resolution_y=1100;scene.render.resolution_percentage=100;scene.render.image_settings.file_format='PNG';scene.render.film_transparent=False;scene.display.shading.background_type='WORLD';scene.render.filepath=str(ROOT/'docs/images/hair-lineup.png');bpy.ops.render.render(write_still=True)
(OUT/'manifest.json').write_text(json.dumps({'styles':REPORT,'fit':'Raycast envelope of actual male and female Face_Sculpt meshes in neutral rest pose','preview':'docs/images/hair-lineup.png','source_heads_preserved':True},indent=2)+'\n')
print('HAIR_BATCH_READY',json.dumps(REPORT))
