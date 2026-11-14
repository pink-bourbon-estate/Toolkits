# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Random Extrude
# Section: mesh_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.random_extrude' does not match convention 'bt.mesh_tools_random_extrude'

import bpy
import bmesh
import random
from bpy.props import FloatProperty
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 3. RANDOM EXTRUDE (THE SCI-FI GREEBLE / CITY BUILDER)
# -----------------------------------------------------------------------------
# What it does: In 3D modelling, "extrude" means grabbing a flat face and pulling
# it outward like a telescope to create a 3D block.
# This tool picks random faces and extrudes them at different heights, turning a
# plain cube into a futuristic sci-fi city or a rocky asteroid!
class BT_OT_RandomExtrude(Operator):
    bl_idname = "bt.random_extrude"
    bl_label  = "Random Extrude"
    bl_description = "Randomly extrude faces on the active mesh"
    bl_options = {'REGISTER', 'UNDO'}

    min_h: FloatProperty(name="Min Height", default=0.0,  unit='LENGTH')
    max_h: FloatProperty(name="Max Height", default=0.1,  unit='LENGTH')
    pct:   FloatProperty(name="Face %",     default=30.0, min=0.0, max=100.0)

    def execute(self, ctx):
        obj = ctx.active_object
        if not obj or obj.type != 'MESH':
            return {'CANCELLED'}

        bm = bmesh.new()
        bm.from_mesh(obj.data)
        random.seed(random.randint(0, 99999))

        # Select which faces will be extruded based on user percentage:
        faces = [f for f in bm.faces if random.random() * 100 < self.pct]

        for f in faces:
            # Pick a random extrusion height between min_h and max_h:
            h = random.uniform(self.min_h, self.max_h)

            # NUANCE: extrude_face_region
            # This cuts around the face and creates new wall faces connecting back down.
            result = bmesh.ops.extrude_face_region(bm, geom=[f])

            # Filter the result list to find the newly-created vertices:
            verts = [e for e in result['geom'] if isinstance(e, bmesh.types.BMVert)]

            # NUANCE: Normal Vectors (f.normal * h)
            # 'f.normal' is an arrow pointing straight out from the surface of the face.
            # By multiplying the normal direction by height (h), we push the extruded vertices
            # outward along the direction the face is pointing!
            for v in verts:
                v.co += f.normal * h

        bm.to_mesh(obj.data)
        bm.free()
        return {'FINISHED'}

    def invoke(self, ctx, event):
        return ctx.window_manager.invoke_props_dialog(self)

def register():
    bpy.utils.register_class(BT_OT_RandomExtrude)

def unregister():
    bpy.utils.unregister_class(BT_OT_RandomExtrude)