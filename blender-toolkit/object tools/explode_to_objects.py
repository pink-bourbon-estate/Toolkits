# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Explode to Objects
# Section: object_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.explode_to_objects' does not match convention 'bt.object_tools_explode_to_objects'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 4. EXPLODE TO OBJECTS (SEPARATE LOOSE PARTS)
# -----------------------------------------------------------------------------
# What it does: If a single 3D model is actually made of several floating parts
# (like a bowl of loose fruit), this splits every separate piece into its own unique object!
class BT_OT_ExplodeToObjects(Operator):
    bl_idname = "bt.explode_to_objects"
    bl_label  = "Explode to Objects"
    bl_description = "Separate each disconnected mesh element into its own object"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # We wrap 'ctx.selected_objects' in list() because separating parts creates 
        # brand-new objects, which would confuse a live loop!
        for obj in list(ctx.selected_objects):
            if obj.type != 'MESH':
                continue
            # Set the current object as the active focus:
            ctx.view_layer.objects.active = obj
            # Hop into Edit Mode, select all geometry, and split by loose parts:
            bpy.ops.object.mode_set(mode='EDIT')
            bpy.ops.mesh.select_all(action='SELECT')
            bpy.ops.mesh.separate(type='LOOSE')
            # Hop back out to Object Mode:
            bpy.ops.object.mode_set(mode='OBJECT')
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_ExplodeToObjects)

def unregister():
    bpy.utils.unregister_class(BT_OT_ExplodeToObjects)