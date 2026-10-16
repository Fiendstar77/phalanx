"""Fit independent, head-parented eye rigs to the artist's new character bases.

Run: blender -b --python tools/fit_rigged_eyes.py
Sources are preserved. Outputs: assets/characters/{male,female}-eyed.{blend,glb}.
Blender coordinates: Z up, face towards -Y. Socket fits are measured per model.
"""
import bpy
import math
from pathlib import Path
from mathutils import Matrix, Vector

ROOT = Path(__file__).resolve().parents[1]
OUT = ROOT / 'assets/characters'
FITS = {
    'male': ('phalanx_base_male.glb', [(-.223, -.211, .523), (.207, -.211, .523)], .123),
    'female': ('phalanx_female_base.glb', [(-.209, -.180, .563), (.209, -.180, .563)], .128),
}


def material(name, color, roughness):
    m = bpy.data.materials.new(name)
    m.diffuse_color = (*color, 1)
    m.use_nodes = True
    shader = m.node_tree.nodes.get('Principled BSDF')
    shader.inputs['Base Color'].default_value = (*color, 1)
    shader.inputs['Roughness'].default_value = roughness
    return m


def iris_mesh(name, radius, palette):
    # A spherical cap, not a floating flat disc: pupil and iris share the eyeball pivot.
    segments = 64
    rings = [0, .07, .14, .21, .25, .29, .36, .43, .49, .52]
    vertices = []
    for angle in rings:
        for i in range(segments):
            theta = i * math.tau / segments
            r = radius * 1.004
            vertices.append((r * math.sin(angle) * math.cos(theta),
                             -r * math.cos(angle),
                             r * math.sin(angle) * math.sin(theta)))
    faces = []
    for j in range(len(rings) - 1):
        for i in range(segments):
            a = j * segments + i
            b = j * segments + (i + 1) % segments
            faces.append((a, b, b + segments, a + segments))
    mesh = bpy.data.meshes.new(name)
    mesh.from_pydata(vertices, [], faces)
    mesh.update()
    obj = bpy.data.objects.new(name, mesh)
    bpy.context.collection.objects.link(obj)
    for mat in palette:
        mesh.materials.append(mat)
    for poly in mesh.polygons:
        ring, segment = divmod(poly.index, segments)
        poly.material_index = (0 if ring < 4 or ring == 8 else
                               1 if ring == 4 else 2 + ((segment * 17) % 4))
        poly.use_smooth = True
    return obj


def build(sex, source, centers, radius):
    bpy.ops.wm.read_factory_settings(use_empty=True)
    bpy.ops.import_scene.gltf(filepath=str(OUT / source))
    body = max((o for o in bpy.context.scene.objects if o.type == 'MESH'), key=lambda o: len(o.data.vertices))
    rig = next(o for o in bpy.context.scene.objects if o.type == 'ARMATURE')
    for o in list(bpy.context.scene.objects):
        if o.type == 'MESH' and o != body:
            bpy.data.objects.remove(o, do_unlink=True)
    rig.data.pose_position = 'POSE'
    white = material('Eye sclera — warm ivory', (.91, .89, .83), .20)
    palette = [material('Pupil and limbal ring', (.008, .005, .003), .20),
               material('Amber pupil collar', (.29, .12, .025), .25)]
    for i, color in enumerate([(.16, .065, .018), (.25, .115, .032), (.34, .18, .055), (.20, .09, .022)]):
        palette.append(material('Hazel iris fibre ' + str(i), color, .24))
    bpy.context.view_layer.objects.active = rig
    rig.select_set(True)
    bpy.ops.object.mode_set(mode='EDIT')
    for side, center in zip(['L', 'R'], centers):
        b = rig.data.edit_bones.new('Eye_' + side)
        b.head = rig.matrix_world.inverted() @ Vector(center)
        b.tail = rig.matrix_world.inverted() @ (Vector(center) + Vector((0, -.07, 0)))
        b.parent = rig.data.edit_bones['mixamorigHead']
        b.use_deform = False
    bpy.ops.object.mode_set(mode='OBJECT')
    for side, center in zip(['L', 'R'], centers):
        bpy.ops.mesh.primitive_uv_sphere_add(segments=48, ring_count=32, radius=radius, location=center)
        eye = bpy.context.object
        eye.name = 'Eyeball_' + side
        eye.data.materials.append(white)
        for p in eye.data.polygons:
            p.use_smooth = True
        iris = iris_mesh('Iris_' + side, radius, palette)
        iris.parent = eye
        iris.matrix_parent_inverse = Matrix.Identity(4)
        iris.location = (0, 0, 0)
        # Preserve the fitted world transform when attaching to the bone's tail space.
        world = eye.matrix_world.copy()
        eye.parent = rig
        eye.parent_type = 'BONE'
        eye.parent_bone = 'Eye_' + side
        bpy.context.view_layer.update()
        eye.matrix_world = world
    bpy.context.view_layer.update()
    for side, center in zip(['L', 'R'], centers):
        actual = bpy.data.objects['Eyeball_' + side].matrix_world.translation
        assert (actual - Vector(center)).length < 1e-5, (actual, center)
    bpy.ops.object.select_all(action='DESELECT')
    for o in [body, rig] + [o for o in bpy.context.scene.objects if o.name.startswith(('Eyeball_', 'Iris_'))]:
        o.select_set(True)
    bpy.context.view_layer.objects.active = rig
    bpy.context.preferences.filepaths.save_version = 0
    bpy.ops.wm.save_as_mainfile(filepath=str(OUT / (sex + '-eyed.blend')))
    bpy.ops.export_scene.gltf(filepath=str(OUT / (sex + '-eyed.glb')), export_format='GLB',
                             use_selection=True, export_skins=True, export_animations=True,
                             export_extras=True)
    print('EYES FITTED', sex)


for sex, (source, centers, radius) in FITS.items():
    build(sex, source, centers, radius)
