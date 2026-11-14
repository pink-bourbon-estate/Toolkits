# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Select Missing UV
# Section: uv
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.select_missing_uv' does not match convention 'bt.uv_select_missing_uv'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 2. SELECT MISSING UV (FIND UNMAPPED OBJECTS)
# -----------------------------------------------------------------------------
# What it does: A detective tool! It checks every 3D model in the scene and
# selects any object that is missing a UV map so you can fix it before texturing.
class BT_OT_SelectMissingUV(Operator):
    bl_idname = "bt.select_missing_uv"
    bl_label  = "Select Missing UV"
    bl_description = "Select all objects that have no UV map"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        for obj in ctx.scene.objects:
            # NUANCE: 'not obj.data.uv_layers'
            # In Python, an empty list evaluates to False.
            # 'not obj.data.uv_layers' means: "If this mesh has zero UV coordinate layers, highlight it!"
            if obj.type == 'MESH' and not obj.data.uv_layers:
                obj.select_set(True)

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_SelectMissingUV)

def unregister():
    bpy.utils.unregister_class(BT_OT_SelectMissingUV)