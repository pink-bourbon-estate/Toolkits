# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Rename Find Replace
# Section: naming
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.rename_find_replace' does not match convention 'bt.naming_rename_find_replace'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 3. FIND & REPLACE (SWAP WORDS IN NAMES)
# -----------------------------------------------------------------------------
# What it does: Searches object names for a specific word and replaces it with another.
# Example: Find "Tree_Oak" and replace with "Tree_Pine".
class BT_OT_RenameFindReplace(Operator):
    bl_idname = "bt.rename_find_replace"
    bl_label  = "Find & Replace"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        p = ctx.scene.blender_toolkit
        for obj in ctx.selected_objects:
            # NUANCE: .replace(old, new)
            # Python's built-in text search-and-replace method.
            # If the search text isn't found, it safely leaves the name unchanged.
            obj.name = obj.name.replace(p.nm_find, p.nm_replace)
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_RenameFindReplace)

def unregister():
    bpy.utils.unregister_class(BT_OT_RenameFindReplace)