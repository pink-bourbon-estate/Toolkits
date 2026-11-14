# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Select Instances
# Section: selection
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.select_instances' does not match convention 'bt.selection_select_instances'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 2. SELECT INSTANCES (FIND TWIN MESHES)
# -----------------------------------------------------------------------------
# What it does: In 3D graphics, you can make "Linked Duplicates" (clones that share
# the exact same internal mesh data to save computer memory). This button selects
# all identical clones of the active object!
class BT_OT_SelectInstances(Operator):
    bl_idname = "bt.select_instances"
    bl_label  = "Select Instances"
    bl_description = "Select all objects that share the same mesh data as the active object"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        if not ctx.object:
            return {'CANCELLED'}

        # 'ctx.object.data' points to the underlying 3D geometry structure (the vertices and faces):
        data = ctx.object.data

        for obj in ctx.scene.objects:
            # Check if this object points to the exact same mesh memory address:
            if obj.data == data:
                obj.select_set(True)

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_SelectInstances)

def unregister():
    bpy.utils.unregister_class(BT_OT_SelectInstances)