# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Select Unparented
# Section: selection
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.select_unparented' does not match convention 'bt.selection_select_unparented'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 6. SELECT UNPARENTED (FIND ORPHAN OBJECTS)
# -----------------------------------------------------------------------------
# What it does: In 3D graphics, you can "parent" objects together like a bicycle:
# if you move the bike frame (parent), the wheels and handlebars (children) follow.
# This button selects objects that are completely independent and have no parent!
class BT_OT_SelectUnparented(Operator):
    bl_idname = "bt.select_unparented"
    bl_label  = "Select Unparented"
    bl_description = "Select all objects with no parent"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        for obj in ctx.scene.objects:
            # If 'obj.parent is None', it has no master object controlling it:
            if obj.parent is None:
                obj.select_set(True)

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_SelectUnparented)

def unregister():
    bpy.utils.unregister_class(BT_OT_SelectUnparented)