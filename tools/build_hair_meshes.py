"""Six head-fitted 3D starter hairstyles authored from the supplied silhouettes.
These are new mesh approximations, not PNGs converted into volumetric truth.
"""
import bpy,math
from pathlib import Path
from mathutils import Vector
ROOT=Path(__file__).resolve().parents[1];OUT=ROOT/'assets/characters/hair';OUT.mkdir(exist_ok=True)
NAMES=['Tousled','Wild','Swept','Bob','Long','Twintails']
def mat(name,color):
 m=bpy.data.materials.new(name);m.diffuse_color=(*color,1);m.use_nodes=True;p=m.node_tree.nodes.get('Principled BSDF');p.inputs['Base Color'].default_value=(*color,1);p.inputs['Roughness'].default_value=.38;return m
def mesh(name,verts,faces,material):
 d=bpy.data.meshes.new(name);d.from_pydata(verts,[],faces);d.update();o=bpy.data.objects.new(name,d);bpy.context.collection.objects.link(o);d.materials.append(material)
 for p in d.polygons:p.use_smooth=True
 return o
def lock(name,points,width,material):
 points=[Vector(p)for p in points];verts=[];faces=[];steps=16;sides=8
 def position(t):
  q=t*(len(points)-1);i=min(len(points)-2,int(q));u=q-i;a=points[max(0,i-1)];b=points[i];c=points[i+1];d=points[min(len(points)-1,i+2)]
  return .5*((2*b)+(-a+c)*u+(2*a-5*b+4*c-d)*u*u+(-a+3*b-3*c+d)*u*u*u)
 for i in range(steps+1):
  t=i/steps;p=position(t);tangent=(position(min(1,t+.015))-position(max(0,t-.015))).normalized();side=tangent.cross(Vector((0,1,0))).normalized()
  if side.length<.1:side=Vector((1,0,0))
  normal=tangent.cross(side).normalized();radius=width*(.8+.25*math.sin(math.pi*t))*(1-t)**.45+.001
  for j in range(sides):
   a=j/sides*math.tau;verts.append(p+side*math.cos(a)*radius+normal*math.sin(a)*radius*.44)
 for i in range(steps):
  for j in range(sides):a=i*sides+j;b=i*sides+(j+1)%sides;faces.append((a,b,b+sides,a+sides))
 faces.extend([tuple(reversed(range(sides))),tuple(steps*sides+j for j in range(sides))]);mesh(name,verts,faces,material)
for style,name in enumerate(NAMES):
 bpy.ops.wm.read_factory_settings(use_empty=True);bpy.context.preferences.filepaths.save_version=0;colors=[mat('Chestnut',(.16,.067,.035)),mat('Copper highlights',(.24,.105,.055)),mat('Deep strands',(.105,.039,.022))]
 verts=[];faces=[];rings=14;sides=48
 for i in range(rings+1):
  for j in range(sides):
   phi=j/sides*math.tau;front=max(0,-math.sin(phi));limit=1.76-.65*front;theta=.025+i/rings*limit
   verts.append((.44*math.sin(theta)*math.cos(phi),.405*math.sin(theta)*math.sin(phi),1.60+.435*math.cos(theta)))
 for i in range(rings):
  for j in range(sides):a=i*sides+j;b=i*sides+(j+1)%sides;faces.append((a,a+sides,b+sides,b))
 mesh('Continuous scalp',verts,faces,colors[0])
 # Swept, pointed fringe clumps have a full elliptical cross section.
 for k in range(9):
  x=(k-4)*.075;sweep=(.11 if style==2 else -.045 if style==1 else .03)*math.sin(k*.8+1);endz=1.63+(.07 if k%2 else -.025)
  if style==1:endz+=.07*(k%3)
  if style==2:sweep=-.14;endz+=.06
  if style==4:sweep=(-1 if x<0 else 1)*.13;endz+=.05
  lock('Fringe',[(x*.55,-.10,1.99),(x,-.27,1.92),(x+sweep,-.36,1.79),(x+sweep*1.4,-.39,endz)],.072,colors[k%3])
 # Locks across the back and temples make the silhouette volumetric.
 for k in range(15):
  phi=k/14*math.pi;phi=phi+.0 # back (+Y), with side locks at both ends
  x=math.cos(phi);y=math.sin(phi);length=.17 if style<3 else .43 if style==3 else .87 if style==4 else .20
  lock('Back lock',[(x*.28,y*.24,1.94),(x*.405,y*.35,1.77),(x*.43,y*.36,1.55),(x*.39,y*.33,1.55-length)],.072,colors[k%3])
 if style==1:
  for k in range(7):
   x=(k-3)*.09;lock('Crown spike',[(x,-.02,1.97),(x+.04,.00,2.06),(x+.10,.03,2.10)],.07,colors[k%3])
 if style==5:
  for side in [-1,1]:
   for k in range(5):
    d=(k-2)*.035;lock('Ponytail',[(side*.34,.10,1.88),(side*(.53+d),.12,1.75),(side*(.58+d),.15,1.42),(side*(.50+d),.13,1.08),(side*(.62+d),.10,.86)],.085,colors[k%3])
 # One editable object per hairstyle, with a few strand-shading material slots.
 bpy.ops.object.select_all(action='SELECT');bpy.context.view_layer.objects.active=next(o for o in bpy.context.scene.objects if o.type=='MESH');bpy.ops.object.join();bpy.context.object.name=name
 bpy.ops.wm.save_as_mainfile(filepath=str(OUT/(str(style+1)+'-'+name.lower()+'.blend')))
 bpy.ops.export_scene.gltf(filepath=str(OUT/(str(style+1)+'.glb')),export_format='GLB',use_selection=True,export_animations=False)
 print('HAIR READY',style+1,name,len(bpy.context.object.data.polygons))
