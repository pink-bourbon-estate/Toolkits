# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Select No Material
# Section: selection
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.select_no_material' does not match convention 'bt.selection_select_no_material'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 3. SELECT NO MATERIAL (FIND UNPAINTED MODELS)
# -----------------------------------------------------------------------------
# What it does: Useful quality-control tool! Finds any objects you forgot to texture
# or colour in before rendering.
class BT_OT_SelectNoMaterial(Operator):
    bl_idname = "bt.select_no_material"
    bl_label  = "Select No Material"
    bl_description = "Select all objects with no material assigned"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        for obj in ctx.scene.objects:
            # NUANCE: Two-stage filter!
            # 1. 'obj.type == "MESH"': Make sure it's a 3D model (ignore cameras and lamps).
            # 2. 'not any(s.material for s in obj.material_slots)': Make sure none of its slots contain paint!
            if obj.type == 'MESH' and not any(s.material for s in obj.material_slots):
                obj.select_set(True)

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_SelectNoMaterial)

def unregister():
    bpy.utils.unregister_class(BT_OT_SelectNoMaterial)