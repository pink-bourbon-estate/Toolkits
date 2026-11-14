# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Object Lock
# Section: object_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.object_lock' does not match convention 'bt.object_tools_object_lock'

import bpy
from bpy.props import BoolProperty
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 7. OBJECT LOCK (FREEZE TRANSFORMS)
# -----------------------------------------------------------------------------
# What it does: Puts virtual padlocks on an object's location, rotation, and scale
# so you cannot accidentally drag or nudge it out of position!
class BT_OT_ObjectLock(Operator):
    bl_idname = "bt.object_lock"
    bl_label  = "Lock"
    bl_description = "Lock all transforms on selected objects"
    bl_options = {'REGISTER', 'UNDO'}

    lock: BoolProperty(default=True)  # True = Locked 🔒, False = Unlocked 🔓

    def execute(self, ctx):
        for obj in ctx.selected_objects:
            # Each transform axis (X, Y, Z) takes a tuple of three Booleans:
            obj.lock_location = (self.lock, self.lock, self.lock)
            obj.lock_rotation = (self.lock, self.lock, self.lock)
            obj.lock_scale    = (self.lock, self.lock, self.lock)
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_ObjectLock)

def unregister():
    bpy.utils.unregister_class(BT_OT_ObjectLock)