# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Remove Missing UV
# Section: scene
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.remove_missing_uv' does not match convention 'bt.scene_remove_missing_uv'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 4. REMOVE MISSING UV OBJECTS (SELECT BROKEN MESHES)
# -----------------------------------------------------------------------------
# What it does: Despite the name "remove", this operator is actually a quality-control
# highlighter! It searches for any mesh models that do not have UV coordinates and
# selects them so the artist can either fix them or delete them.
class BT_OT_RemoveMissingUV(Operator):
    bl_idname = "bt.remove_missing_uv"
    bl_label  = "Remove Missing UV Objects"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        found = 0
        for obj in ctx.scene.objects:
            # If it's a 3D model and has no UV layers, highlight it with orange selection lines:
            if obj.type == 'MESH' and not obj.data.uv_layers:
                obj.select_set(True)
                found += 1

        self.report({'INFO'}, f"Selected {found} objects with no UV map")
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_RemoveMissingUV)

def unregister():
    bpy.utils.unregister_class(BT_OT_RemoveMissingUV)