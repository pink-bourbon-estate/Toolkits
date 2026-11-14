# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Copy Transform
# Section: align
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.copy_transform' does not match convention 'bt.align_copy_transform'

import bpy
from bpy.props import BoolProperty
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 6. COPY TRANSFORM (CLONE POSITION, ROTATION, AND SCALE)
# -----------------------------------------------------------------------------
# What it does: Takes the transform values from the active object and stamps them
# onto all other selected objects.
class BT_OT_CopyTransform(Operator):
    """Copy loc/rot/scale from active object to all other selected objects (V4)."""
    bl_idname  = "bt.copy_transform"
    bl_label   = "Copy Transform"
    bl_description = "Copy location, rotation and/or scale from active to all other selected objects"
    bl_options = {'REGISTER', 'UNDO'}

    # Checkboxes allowing the user to select which properties to copy:
    copy_loc: BoolProperty(name="Location", default=True)
    copy_rot: BoolProperty(name="Rotation", default=True)
    copy_scl: BoolProperty(name="Scale",    default=True)

    def execute(self, ctx):
        source = ctx.active_object
        if not source:
            self.report({'WARNING'}, "No active object")
            return {'CANCELLED'}

        for obj in ctx.selected_objects:
            if obj == source:
                continue  # Don't copy an object onto itself!

            if self.copy_loc: obj.location       = source.location.copy()
            if self.copy_rot: obj.rotation_euler = source.rotation_euler.copy()
            if self.copy_scl: obj.scale          = source.scale.copy()

        return {'FINISHED'}

    def invoke(self, ctx, event):
        return ctx.window_manager.invoke_props_dialog(self)

def register():
    bpy.utils.register_class(BT_OT_CopyTransform)

def unregister():
    bpy.utils.unregister_class(BT_OT_CopyTransform)