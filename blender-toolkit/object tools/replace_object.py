# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Replace Object
# Section: object_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.replace_object' does not match convention 'bt.object_tools_replace_object'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 2. REPLACE OBJECT (SWAP MESHES)
# -----------------------------------------------------------------------------
# What it does: Takes the mesh shape of the active object and stamps it onto all
# other selected objects (e.g. replacing placeholder cubes with finished lampposts).
class BT_OT_ReplaceObject(Operator):
    bl_idname = "bt.replace_object"
    bl_label  = "Replace with Active"
    bl_description = "Replace selected objects' mesh with the active object's mesh"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        source = ctx.active_object
        if not source or source.type != 'MESH':
            self.report({'WARNING'}, "Active object must be a mesh")
            return {'CANCELLED'}

        for obj in ctx.selected_objects:
            # Skip the active object itself and any non-mesh objects (like lights or cameras):
            if obj == source or obj.type != 'MESH':
                continue
            # Point this object's geometry data to the source's geometry data:
            obj.data = source.data
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_ReplaceObject)

def unregister():
    bpy.utils.unregister_class(BT_OT_ReplaceObject)