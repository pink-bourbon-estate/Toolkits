# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Remove Empty Collections
# Section: scene
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.remove_empty_collections' does not match convention 'bt.scene_remove_empty_collections'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 2. REMOVE EMPTY COLLECTIONS (DELETE EMPTY FOLDERS)
# -----------------------------------------------------------------------------
# What it does: Collections are like folders in your 3D Outliner list.
# As you move models around, you often end up with dozens of empty folders.
# This hunts down every folder that has nothing inside it and removes it!
class BT_OT_RemoveEmptyCollections(Operator):
    bl_idname = "bt.remove_empty_collections"
    bl_label  = "Remove Empty Collections"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # NUANCE: Double-Checking Emptiness!
        # A collection is only truly empty if:
        #   1. 'not c.objects'  -> It contains no 3D objects.
        #   2. 'not c.children' -> It contains no sub-folders either!
        to_remove = [c for c in bpy.data.collections if not c.objects and not c.children]

        # Loop through our hit-list and delete each one from Blender's memory:
        for c in to_remove:
            bpy.data.collections.remove(c)

        self.report({'INFO'}, f"Removed {len(to_remove)} empty collections")
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_RemoveEmptyCollections)

def unregister():
    bpy.utils.unregister_class(BT_OT_RemoveEmptyCollections)