# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Rename Prefix
# Section: naming
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.rename_prefix' does not match convention 'bt.naming_rename_prefix'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 1. ADD PREFIX (STICK A WORD AT THE FRONT)
# -----------------------------------------------------------------------------
# What it does: Adds text to the very beginning of every selected object's name.
# Example: Adding "SM_" turns "Chair" into "SM_Chair" (Static Mesh Chair).
class BT_OT_RenamePrefix(Operator):
    bl_idname = "bt.rename_prefix"
    bl_label  = "Add Prefix"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        p = ctx.scene.blender_toolkit
        for obj in ctx.selected_objects:
            # NUANCE: String Concatenation (+)
            # In Python, using '+' between pieces of text glues them together!
            obj.name = p.nm_prefix + obj.name
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_RenamePrefix)

def unregister():
    bpy.utils.unregister_class(BT_OT_RenamePrefix)