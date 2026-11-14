# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Make Instances
# Section: object_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.make_instances' does not match convention 'bt.object_tools_make_instances'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 3. MAKE INSTANCES (LINKED DUPLICATES)
# -----------------------------------------------------------------------------
# What it does: Similar to Replace Object, but it actively counts how many items
# it turned into memory-efficient "instances" and prints a friendly report!
class BT_OT_MakeInstances(Operator):
    """Replace selected objects' mesh with the active object's mesh data (linked duplicate) (V4)."""
    bl_idname = "bt.make_instances"
    bl_label  = "Make Selected Instances"
    bl_description = "Replace selected objects' mesh with active object's mesh so they share data (true instances)"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        source = ctx.active_object
        if not source or source.type != 'MESH':
            self.report({'WARNING'}, "Active object must be a mesh")
            return {'CANCELLED'}

        count = 0  # Counter variable to keep track of how many objects were linked
        for obj in ctx.selected_objects:
            if obj == source or obj.type != 'MESH':
                continue
            obj.data = source.data
            count += 1  # Add 1 to our count for every successful replacement

        # NUANCE: self.report({'INFO'}, ...)
        # This displays a friendly blue info banner at the bottom of Blender's screen!
        # The 'f"... {source.name}"' is an f-string (formatted string) that injects the name directly.
        self.report({'INFO'}, f"Made {count} object(s) instance of '{source.name}'")
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_MakeInstances)

def unregister():
    bpy.utils.unregister_class(BT_OT_MakeInstances)