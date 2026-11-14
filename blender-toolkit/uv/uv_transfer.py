# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — UV Transfer
# Section: uv
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.uv_transfer' does not match convention 'bt.uv_uv_transfer'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 3. TRANSFER UVs (COPY UV WRAPPING FROM ONE MODEL TO ANOTHER)
# -----------------------------------------------------------------------------
# What it does: If you have already spent hours perfecting the UV seams on a model,
# and you have other models of a similar shape, this copies the UV map across
# using a 'Data Transfer' modifier and instantly bakes it in!
class BT_OT_UVTransfer(Operator):
    bl_idname = "bt.uv_transfer"
    bl_label  = "Transfer UVs"
    bl_description = "Copy UVs from active object to selected objects via data transfer modifier"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # The primary orange-highlighted object is our 'source' (the master model):
        source = ctx.active_object
        if not source:
            return {'CANCELLED'}

        for obj in ctx.selected_objects:
            if obj == source or obj.type != 'MESH':
                continue

            # 1. Add a temporary Data Transfer modifier to the receiving object:
            mod = obj.modifiers.new("UVTransfer", 'DATA_TRANSFER')
            mod.object           = source      # Set the donor object
            mod.use_loop_data    = True        # Tell it to inspect polygon corners (loops)
            mod.data_types_loops = {'UV'}      # Only transfer UV coordinates (ignore colours/normals)
            mod.loop_mapping     = 'POLYINTERP_NEAREST'  # Transfer to the nearest matching polygon face

            # 2. Fire the transfer operator to copy the UV coordinates across:
            bpy.ops.object.datalayout_transfer(modifier=mod.name)

            # NUANCE: 'modifier_apply'
            # Applying a modifier bakes the changes permanently into the mesh geometry
            # and automatically deletes the modifier to keep the stack clean!
            bpy.ops.object.modifier_apply(modifier=mod.name)

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_UVTransfer)

def unregister():
    bpy.utils.unregister_class(BT_OT_UVTransfer)