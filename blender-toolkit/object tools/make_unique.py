# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Make Unique
# Section: object_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.make_unique' does not match convention 'bt.object_tools_make_unique'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 1. MAKE UNIQUE (BREAK THE TWIN LINK)
# -----------------------------------------------------------------------------
# What it does: If two objects share the exact same mesh (like identical twins),
# editing one will accidentally edit the other! This tool gives each object its
# own private, independent copy of the mesh data.
class BT_OT_MakeUnique(Operator):
    bl_idname = "bt.make_unique"
    bl_label  = "Make Unique"
    bl_description = "Give each selected object its own copy of the mesh data"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        for obj in ctx.selected_objects:
            # NUANCE: .copy() on Object Data
            # 'obj.data' holds the actual 3D geometry (vertices, edges, faces).
            # Calling '.copy()' clones the mesh in memory and assigns the new 
            # clone back to the object, severing the shared connection!
            if obj.data:
                obj.data = obj.data.copy()
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_MakeUnique)

def unregister():
    bpy.utils.unregister_class(BT_OT_MakeUnique)