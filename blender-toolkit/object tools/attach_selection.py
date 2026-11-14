# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Attach Selection
# Section: object_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.attach_selection' does not match convention 'bt.object_tools_attach_selection'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 5. ATTACH SELECTION (JOIN MULTIPLE OBJECTS INTO ONE)
# -----------------------------------------------------------------------------
# What it does: The exact opposite of Explode! Glues all selected objects together 
# into one single combined mesh.
class BT_OT_AttachSelection(Operator):
    bl_idname = "bt.attach_selection"
    bl_label  = "Attach Selection"
    bl_description = "Join all selected objects into the active object"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # We need at least 2 objects selected and one must be active (the master):
        if not ctx.active_object or len(ctx.selected_objects) < 2:
            self.report({'WARNING'}, "Select 2+ objects with an active target")
            return {'CANCELLED'}
        # Run Blender's native join tool (equivalent to pressing Ctrl+J):
        bpy.ops.object.join()
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_AttachSelection)

def unregister():
    bpy.utils.unregister_class(BT_OT_AttachSelection)