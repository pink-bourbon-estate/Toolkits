# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Purge Unused
# Section: scene
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.purge_unused' does not match convention 'bt.scene_purge_unused'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 1. PURGE UNUSED DATA (EMPTYING THE RECYCLE BIN)
# -----------------------------------------------------------------------------
# What it does: In Blender, when you delete a material, image, or mesh from an object,
# Blender keeps it in the background as an "orphan" just in case you want it back.
# This operator purges (permanently deletes) all of that leftover ghost data!
class BT_OT_PurgeUnused(Operator):
    bl_idname = "bt.purge_unused"
    bl_label  = "Purge Unused Data"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # NUANCE: do_recursive=True
        # Some unused items rely on OTHER unused items (like an unused material that
        # holds an unused picture).
        # 'do_recursive=True' tells Blender: "Keep peeling away the layers until EVERY
        # orphan item and all its connected clutter are completely gone!"
        bpy.ops.outliner.orphans_purge(do_recursive=True)
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_PurgeUnused)

def unregister():
    bpy.utils.unregister_class(BT_OT_PurgeUnused)