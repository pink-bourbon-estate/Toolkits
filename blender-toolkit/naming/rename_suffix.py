# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Rename Suffix
# Section: naming
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.rename_suffix' does not match convention 'bt.naming_rename_suffix'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 2. ADD SUFFIX (STICK A WORD AT THE END)
# -----------------------------------------------------------------------------
# What it does: Adds text to the very end of every selected object's name.
# Example: Adding "_HighPoly" turns "Sword" into "Sword_HighPoly".
class BT_OT_RenameSuffix(Operator):
    bl_idname = "bt.rename_suffix"
    bl_label  = "Add Suffix"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        p = ctx.scene.blender_toolkit
        for obj in ctx.selected_objects:
            # Glue the suffix text onto the right-hand side:
            obj.name = obj.name + p.nm_suffix
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_RenameSuffix)

def unregister():
    bpy.utils.unregister_class(BT_OT_RenameSuffix)