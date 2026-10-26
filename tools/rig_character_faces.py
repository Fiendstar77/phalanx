"""Rig the artist's actual heads with expression shapes and a mouth interior.
Run after fit_rigged_eyes.py, before prepare_combat_characters.py, using Blender.
Sources are preserved; facial-rig Blends are the editable animation sources.
"""
import bpy, bmesh, math
from pathlib import Path
from mathutils import Vector, Matrix

ROOT=Path(__file__).resolve().parents[1]
OUT=ROOT/'assets/characters'
KEYS=['Rage','Pain','Fear','Grin','Shout','Blink','NoseLong','NoseWide','JawWide','CheekSoft',
      'FaceRound','FaceTall','NoseUpturned','MouthSweet','MouthPout','BrowSoft','BrowArch']

def mat(name,color,rough=.6):
 m=bpy.data.materials.new(name);m.diffuse_color=(*color,1);m.use_nodes=True
 p=m.node_tree.nodes.get('Principled BSDF');p.inputs['Base Color'].default_value=(*color,1);p.inputs['Roughness'].default_value=rough
 return m

def clamp(x):return max(0,min(1,x))
def smooth(x):x=clamp(x);return x*x*(3-2*x)
def gauss(x,z,cx,cz,rx,rz):return math.exp(-((x-cx)/rx)**2-((z-cz)/rz)**2)

def bone_parent(obj,rig):
 world=obj.matrix_world.copy();obj.parent=rig;obj.parent_type='BONE';obj.parent_bone='mixamorigHead'
 bpy.context.view_layer.update();obj.matrix_world=world

def shapes(obj,warp,world_identity=True):
 obj.shape_key_add(name='Basis')
 for name in KEYS:
  key=obj.shape_key_add(name=name);key.value=0.0
  for src,dest in zip(obj.data.vertices,key.data):
   p=warp(src.co.copy(),name);dest.co=identity(p,name) if world_identity else p
 for p in obj.data.polygons:p.use_smooth=True

for sex in ['male','female']:
 bpy.ops.wm.read_factory_settings(use_empty=True)
 bpy.ops.wm.open_mainfile(filepath=str(OUT/(sex+'-eyed.blend')))
 rig=next(o for o in bpy.context.scene.objects if o.type=='ARMATURE')
 body=max((o for o in bpy.context.scene.objects if o.type=='MESH'),key=lambda o:len(o.data.vertices))
 # Split along existing triangles; retain the artist's actual head and topology.
 cut=.235 if sex=='male' else .305
 head=body.copy();head.data=body.data.copy();bpy.context.collection.objects.link(head);head.name='Face_Sculpt'
 for obj,keep_head in [(head,True),(body,False)]:
  bm=bmesh.new();bm.from_mesh(obj.data)
  doomed=[f for f in bm.faces if ((sum((obj.matrix_world@v.co).z for v in f.verts)/len(f.verts))>cut)!=keep_head]
  bmesh.ops.delete(bm,geom=doomed,context='FACES')
  loose=[v for v in bm.verts if not v.link_faces]
  bmesh.ops.delete(bm,geom=loose,context='VERTS');bm.to_mesh(obj.data);bm.free()
 for mod in list(head.modifiers):head.modifiers.remove(mod)
 world=head.matrix_world.copy()
 for v in head.data.vertices:v.co=world@v.co
 head.parent=None;head.matrix_world=Matrix.Identity(4)
 skin=mat('Face skin',(1,.65,.43),.78);head.data.materials.clear();head.data.materials.append(skin)
 # Add deformation support without changing the authored silhouette or neck
 # boundary. Weld normal seams first, subdivide once, then recompute winding.
 bm=bmesh.new();bm.from_mesh(head.data)
 bmesh.ops.remove_doubles(bm,verts=list(bm.verts),dist=.00001)
 bmesh.ops.subdivide_edges(bm,edges=list(bm.edges),cuts=1,use_grid_fill=True)
 bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces))
 bm.to_mesh(head.data);bm.free()
 # Integrate a rounder nose and soften the brow ridge; no replacement primitive head.
 cx=-.008 if sex=='male' else 0
 mz=.369 if sex=='male' else .403
 my=-.337 if sex=='male' else -.307
 ez=.523 if sex=='male' else .563
 ex=.215 if sex=='male' else .209
 for v in head.data.vertices:
  p=v.co;front=smooth((-p.y-.16)/.12)
  nose=gauss(p.x,p.z,cx,mz+.069,.052,.048)*front
  p.y+=nose*.012
  p.x-=((p.x-cx)*.16*nose)
 def identity(p,key):
  # Apply identity changes to the sculpt AND its lips/teeth/tongue. A taller
  # face must not leave the mouth floating in the old position.
  front=smooth((-p.y-.12)/.12)
  lower=smooth((ez-.055-p.z)/.17)
  if key=='FaceRound':p.x+=(p.x-cx)*.22*lower*front
  if key=='FaceTall':p.z-=.085*lower
  mouth_area=math.exp(-(abs(p.x-cx)/.16)**4-(abs(p.z-mz)/.085)**4)*front
  if key=='MouthSweet':
   p.x-=(p.x-cx)*.28*mouth_area
   p.z+=.024*(abs(p.x-cx)/.10)**1.5*mouth_area
  if key=='MouthPout':
   p.y-=.030*mouth_area
   p.z-=.016*(abs(p.x-cx)/.10)**1.4*mouth_area
  return p
 # Open the sculpt's existing lip seam; project the boundary to a clean ellipse.
 rx=.087 if sex=='male' else .082;rz=.014
 bm=bmesh.new();bm.from_mesh(head.data)
 bmesh.ops.remove_doubles(bm,verts=list(bm.verts),dist=.00001)
 bmesh.ops.recalc_face_normals(bm,faces=list(bm.faces))
 faces=[f for f in bm.faces if (lambda p: p.y<my+.045 and ((p.x-cx)/rx)**2+((p.z-mz)/rz)**2<1)(f.calc_center_median())]
 bmesh.ops.delete(bm,geom=faces,context='FACES')
 for v in bm.verts:
  if v.is_boundary and abs(v.co.x-cx)<rx*1.3 and abs(v.co.z-mz)<.035 and v.co.y<my+.045:
   t=math.atan2((v.co.z-mz)/rz,(v.co.x-cx)/rx)
   v.co.x=cx+rx*math.cos(t);v.co.z=mz+rz*math.sin(t)
 loose=[v for v in bm.verts if not v.link_faces];bmesh.ops.delete(bm,geom=loose,context='VERTS')
 bm.to_mesh(head.data);bm.free()
 def mouth(p,key):
  dx=p.x-cx;dz=p.z-mz;front=smooth((-p.y-.13)/.11)
  strength=math.exp(-(abs(dx)/.19)**4-(abs(dz)/.17)**4)*front*smooth((p.z-cut-.012)/.065)
  opening={'Rage':.082,'Pain':.043,'Fear':.054,'Grin':.040,'Shout':.125}.get(key,0)
  lower=smooth((mz+.010-p.z)/.030)
  p.z-=opening*lower*strength
  if key in ['Rage','Shout']:p.z+=.012*(1-lower)*strength
  widen={'Rage':.25,'Pain':.13,'Fear':-.22,'Grin':.33,'Shout':.18}.get(key,0)
  p.x+=dx*widen*strength
  if key=='Grin':p.z+=.030*(abs(dx)/.12)**1.4*strength
  if key=='Pain':p.z+=.020*(dx/.14)*strength
  return p
 def facial(p,key):
  orig=p.copy();front=smooth((-p.y-.13)/.11);p=mouth(p,key)
  if key in ['NoseLong','NoseWide','NoseUpturned']:
   n=gauss(orig.x,orig.z,cx,mz+.070,.073,.060)*front
   if key=='NoseLong':p.y-=.075*n
   elif key=='NoseWide':p.x+=(orig.x-cx)*.95*n;p.y-=.012*n
   else:p.z+=.045*n;p.y+=.020*n
  if key=='JawWide':p.x+=(orig.x-cx)*.26*gauss(orig.x,orig.z,cx,mz-.03,.25,.12)*front
  if key=='CheekSoft':
   cheek=gauss(abs(orig.x),orig.z,.22,mz+.05,.11,.11)*front
   p.y-=.050*cheek;p.x+=math.copysign(.025*cheek,orig.x-cx)
  for side in [-1,1]:
   dx=orig.x-side*ex;dz=orig.z-ez
   lid=gauss(orig.x,orig.z,side*ex,ez,.125,.13)*front
   if key=='Blink':pass # The separate spherical lids close over the eyes.
   if key=='Rage':p.z-=.016*lid*(1 if dx*side<0 else -.4)
   if key=='Pain' and side<0:p.z-=.025*math.tanh(dz/.035)*lid
  return p
 shapes(head,facial);bone_parent(head,rig)
 black=mat('Ink brows',(0.009,.005,.003),.85)
 ivory=mat('Teeth ivory',(.94,.85,.64),.35)
 oral=mat('Mouth interior',(.055,.003,.008),.95)
 tongue=mat('Tongue coral',(.50,.055,.085),.42)
 lip=mat('Lip rim',(.56,.23,.16),.7)
 def sphere(name,at,scale,material):
  bpy.ops.mesh.primitive_uv_sphere_add(segments=32,ring_count=16,location=at)
  obj=bpy.context.object;obj.name=name
  for v in obj.data.vertices:v.co=Vector(at)+Vector((v.co.x*scale[0],v.co.y*scale[1],v.co.z*scale[2]))
  obj.location=(0,0,0);obj.data.materials.append(material);bpy.context.view_layer.update()
  return obj
 bag=sphere('Mouth_Interior',(cx,my+.085,mz),(rx*1.12,.045,.020),oral)
 shapes(bag,mouth);bone_parent(bag,rig)
 for upper in [True,False]:
  objs=[]
  for i in range(8):
   x=(i-3.5)*.019+cx
   obj=sphere('Tooth',(x,my+.008+abs(x-cx)*.10,mz+(.009 if upper else -.007)),(.009,.010,.009),ivory);objs.append(obj)
  bpy.ops.object.select_all(action='DESELECT')
  for o in objs:o.select_set(True)
  bpy.context.view_layer.objects.active=objs[0];bpy.ops.object.join();obj=objs[0];obj.name='Teeth_Upper' if upper else 'Teeth_Lower'
  def teeth_pose(p,key,upper=upper):
   at=Vector((cx,my+.008,mz+(.009 if upper else -.007)))
   return p+mouth(at.copy(),key)-at
  shapes(obj,teeth_pose);bone_parent(obj,rig)
 obj=sphere('Tongue',(cx,my+.020,mz-.009),(.035,.026,.007),tongue)
 def tongue_pose(p,key):
  at=Vector((cx,my+.020,mz-.009));return p+mouth(at.copy(),key)-at
 shapes(obj,tongue_pose);bone_parent(obj,rig)
 # A continuous small lip rim covers the cut edge and follows the same poses.
 verts=[];faces=[];segments=64;sides=8
 for i in range(segments):
  t=i*math.tau/segments
  for j in range(sides):
   a=j*math.tau/sides;r=.0045
   verts.append((cx+(rx+r*math.cos(a))*math.cos(t),my-.005+abs(rx*math.cos(t))*.10+r*math.sin(a),mz+(rz+r*math.cos(a))*math.sin(t)))
 for i in range(segments):
  for j in range(sides):
   a=i*sides+j;b=i*sides+(j+1)%sides;c=((i+1)%segments)*sides+(j+1)%sides;d=((i+1)%segments)*sides+j;faces.append((a,b,c,d))
 mesh=bpy.data.meshes.new('Lip rim');mesh.from_pydata(verts,[],faces);mesh.update()
 obj=bpy.data.objects.new('Lip_Rim',mesh);bpy.context.collection.objects.link(obj);mesh.materials.append(skin)
 shapes(obj,mouth);bone_parent(obj,rig)
 # Ink-like expressive eyebrows, bent with each pose; follow the sculpt surface.
 for side in [-1,1]:
  verts=[];faces=[];steps=24;sides=8
  for i in range(steps+1):
   t=i/steps;xx=side*(.105+t*.205);zz=ez+.125+.028*math.sin(t*math.pi)-t*.010
   # Fit against the head using the untouched head's evaluated ray surface.
   ok,at,_,_=head.ray_cast(Vector((xx,-2,zz)),Vector((0,1,0)))
   yy=at.y-.009 if ok else my+.005
   width=(.010 if sex=='female' else .014)*(0.65+.35*math.sin(math.pi*t))
   for j in range(sides):
    a=j*math.tau/sides;verts.append((xx,yy+math.cos(a)*width*.55,zz+math.sin(a)*width))
  for i in range(steps):
   for j in range(sides):a=i*sides+j;b=i*sides+(j+1)%sides;faces.append((a,b,b+sides,a+sides))
  mesh=bpy.data.meshes.new('Brow');mesh.from_pydata(verts,[],faces);mesh.update()
  obj=bpy.data.objects.new('Brow_L' if side<0 else 'Brow_R',mesh);bpy.context.collection.objects.link(obj);mesh.materials.append(black)
  def brow(p,key,side=side):
   inner=clamp((.32-abs(p.x))/.22)
   p.z+={'Rage':-.055*inner+.015*(1-inner),'Pain':.040*inner if side>0 else -.030,'Fear':.050*inner+.020,'Grin':-.020 if side<0 else .025*inner,'Shout':-.042*inner,'Blink':-.010}.get(key,0)
   if key=='BrowSoft':p.z+=.038*inner-.018*(1-inner)
   if key=='BrowArch':p.z+=.040*math.sin(clamp((abs(p.x)-.105)/.205)*math.pi)
   return p
  shapes(obj,brow);bone_parent(obj,rig)
 # Blinking lids are actual skin shells over the upper half of each sphere.
 for side,center in zip(['L','R'],[bpy.data.objects['Eyeball_L'].matrix_world.translation,bpy.data.objects['Eyeball_R'].matrix_world.translation]):
  radius=.134 if sex=='male' else .139
  rows=12;cols=32
  # A single pole closes the shell. Duplicate pole vertices produced tiny
  # inverted slivers after triangulation and export rounding.
  samples=[(j,i)for j in range(rows)for i in range(cols+1)]+[(rows,0)]
  def lid_point(j,i,amount):
   phi=math.pi*i/cols
   lower=1.35*(1-amount)-1.48*amount
   alpha=lower+(math.pi/2-lower)*j/rows
   return (center.x+radius*math.cos(phi)*math.cos(alpha),center.y-radius*math.sin(phi)*math.cos(alpha),center.z+radius*math.sin(alpha))
  verts=[lid_point(j,i,0)for j,i in samples];faces=[];pole=len(verts)-1
  for j in range(rows-1):
   for i in range(cols):
    a=j*(cols+1)+i;faces.append((a,a+cols+1,a+cols+2,a+1))
  for i in range(cols):
   a=(rows-1)*(cols+1)+i;faces.append((a,pole,a+1))
  mesh=bpy.data.meshes.new('Lid');mesh.from_pydata(verts,[],faces);mesh.update()
  obj=bpy.data.objects.new('Lid_'+side,mesh);bpy.context.collection.objects.link(obj);mesh.materials.append(skin)
  shapes(obj,lambda p,key:p)
  # Runtime interpolates adjacent quarter-closure shapes. Interpolating
  # straight from the top pole to a closed hemisphere crosses the eyeball.
  for keyname,amount in [('Rage',.25),('Grin',.50),('Pain',.75),('Blink',1.)]:
   key=obj.data.shape_keys.key_blocks[keyname]
   for vertex,(j,i) in zip(key.data,samples):vertex.co=lid_point(j,i,amount)
  bone_parent(obj,rig)
 # Fear contracts the iris along the spherical surface, avoiding floating
 # flat discs or scaling it behind the white eyeball.
 for side in ['L','R']:
  iris=bpy.data.objects['Iris_'+side]
  def iris_pose(p,key):
   if key=='Fear':
    radius=p.length;angle=math.atan2(math.hypot(p.x,p.z),-p.y);phi=math.atan2(p.z,p.x)
    angle*=.55;p=Vector((radius*math.sin(angle)*math.cos(phi),-radius*math.cos(angle),radius*math.sin(angle)*math.sin(phi)))
   return p
  shapes(iris,iris_pose,False)
 for obj in bpy.context.scene.objects:
  if obj.type=='MESH':
   for poly in obj.data.polygons:poly.use_smooth=True
   obj.data.normals_split_custom_set([(0,0,0)]*len(obj.data.loops))
 bpy.context.preferences.filepaths.save_version=0
 bpy.ops.wm.save_as_mainfile(filepath=str(OUT/(sex+'-facial-rig.blend')))
 print('FACIAL RIG READY',sex,len(head.data.vertices),'head vertices',len(KEYS),'shape controls')
