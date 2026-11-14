# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Random Face Select
# Section: mesh_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.random_face_select' does not match convention 'bt.mesh_tools_random_face_select'

import bpy
import bmesh
import random
from bpy.props import FloatProperty
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 2. RANDOM FACE SELECT (SPOTTED HIGHLIGHTER)
# -----------------------------------------------------------------------------
# What it does: Jumps into Edit Mode and highlights a random scattering of faces.
# This is useful when you want to paint different colours onto random floor tiles or building windows!
class BT_OT_RandomFaceSelect(Operator):
    bl_idname = "bt.random_face_select"
    bl_label  = "Random Face Select"
    bl_description = "Randomly select a percentage of faces in edit mode"
    bl_options = {'REGISTER', 'UNDO'}

    pct: FloatProperty(name="Select %", default=20.0, min=0.0, max=100.0)

    def execute(self, ctx):
        obj = ctx.active_object
        if not obj or obj.type != 'MESH':
            return {'CANCELLED'}

        # 1. Hop straight into EDIT mode so we can select internal parts:
        bpy.ops.object.mode_set(mode='EDIT')

        # 2. Clear any existing face selections:
        bpy.ops.mesh.select_all(action='DESELECT')

        # NUANCE: 'from_edit_mesh' vs 'from_mesh'
        # In the previous tool, the model was in Object Mode, so we used 'bm.from_mesh()'.
        # Here, the object is in EDIT Mode, so we must use 'bmesh.from_edit_mesh()'
        # to connect directly to the live edit session!
        bm = bmesh.from_edit_mesh(obj.data)
        random.seed(random.randint(0, 99999))

        # Roll the dice for every face; if it hits the percentage, select it!
        for f in bm.faces:
            if random.random() * 100 < self.pct:
                f.select = True

        # Refresh the edit screen so the highlighted orange faces light up:
        bmesh.update_edit_mesh(obj.data)
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_RandomFaceSelect)

def unregister():
    bpy.utils.unregister_class(BT_OT_RandomFaceSelect)