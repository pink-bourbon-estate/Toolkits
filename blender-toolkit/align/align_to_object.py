# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Align To Object
# Section: align
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.align_to_object' does not match convention 'bt.align_align_to_object'

import bpy
from bpy.props import BoolProperty
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 3. ALIGN TO ACTIVE (COPY POSITION / ROTATION)
# -----------------------------------------------------------------------------
# What it does: Snaps selected objects to match the active object's position or rotation.
class BT_OT_AlignToObject(Operator):
    bl_idname  = "bt.align_to_object"
    bl_label   = "Align to Active"
    bl_description = "Align selected objects' position/rotation to the active object"
    bl_options = {'REGISTER', 'UNDO'}

    do_pos: BoolProperty(name="Position", default=True)
    do_rot: BoolProperty(name="Rotation", default=True)

    def execute(self, ctx):
        target = ctx.active_object
        if not target:
            self.report({'WARNING'}, "No active object")
            return {'CANCELLED'}

        for obj in ctx.selected_objects:
            if obj == target:
                continue

            # NUANCE: Using .copy()!
            # In Python, if you write 'obj.location = target.location', both objects
            # reference the same vector memory. Modifying one could accidentally modify both!
            # '.copy()' creates an independent duplicate so they stay separated.
            if self.do_pos:
                obj.location = target.location.copy()
            if self.do_rot:
                obj.rotation_euler = target.rotation_euler.copy()

        return {'FINISHED'}

    def invoke(self, ctx, event):
        return ctx.window_manager.invoke_props_dialog(self)

def register():
    bpy.utils.register_class(BT_OT_AlignToObject)

def unregister():
    bpy.utils.unregister_class(BT_OT_AlignToObject)