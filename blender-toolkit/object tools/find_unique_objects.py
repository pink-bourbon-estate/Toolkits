# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Find Unique Objects
# Section: object_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.find_unique_objects' does not match convention 'bt.object_tools_find_unique_objects'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 1. FIND UNIQUE OBJECTS (SEARCH FOR NON-CLONED MESHES)
# -----------------------------------------------------------------------------
# What it does: In Blender, several objects can share the exact same mesh data
# (these are called instances/clones).
# This button hunts through the scene and selects only objects that have a unique,
# one-of-a-kind mesh that nobody else shares!
class BT_OT_FindUniqueObjects(Operator):
    bl_idname = "bt.find_unique_objects"
    bl_label  = "Select Unique Objects"
    bl_description = "Select all objects whose mesh data has only 1 user (not instanced)"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        count = 0
        for obj in ctx.scene.objects:
            # NUANCE: What is 'data.users'?
            # Blender keeps a scorecard counter for every mesh showing how many
            # objects use it.
            # If 'users == 1', exactly ONE object uses this mesh (it is unique)!
            # If 'users > 1', it has twin clones in the scene.
            if obj.type == 'MESH' and obj.data and obj.data.users == 1:
                obj.select_set(True)  # Highlight it in orange
                count += 1

        self.report({'INFO'}, f"Selected {count} unique objects")
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_FindUniqueObjects)

def unregister():
    bpy.utils.unregister_class(BT_OT_FindUniqueObjects)