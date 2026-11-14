# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Select By Material
# Section: selection
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.select_by_material' does not match convention 'bt.selection_select_by_material'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 1. SELECT BY MATERIAL (FIND MATCHING COLOURS / TEXTURES)
# -----------------------------------------------------------------------------
# What it does: Looks at the material on your highlighted object (e.g. "Shiny Gold")
# and selects every other model in the whole scene wearing that same material!
class BT_OT_SelectByMaterial(Operator):
    bl_idname = "bt.select_by_material"
    bl_label  = "Select by Material"
    bl_description = "Select all objects that use the active material"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # NUANCE: The Safe "Active Material" Check!
        # An object might not exist, or it might not have any paint/material on it at all.
        # This one-line check avoids crashing if you click the button with nothing selected:
        active_mat = ctx.object.active_material if ctx.object else None

        if not active_mat:
            self.report({'WARNING'}, "No active material")
            return {'CANCELLED'}

        # Loop through every single object in the entire 3D project:
        for obj in ctx.scene.objects:
            # NUANCE: What is 'any(...)'?
            # A 3D model can have multiple material slots (e.g. tyre rubber, window glass, car paint).
            # 'any(...)' checks every slot on the model: if even ONE slot matches active_mat,
            # it evaluates to True!
            if any(slot.material == active_mat for slot in obj.material_slots):
                # 'select_set(True)' turns on the orange selection outline for this object:
                obj.select_set(True)

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_SelectByMaterial)

def unregister():
    bpy.utils.unregister_class(BT_OT_SelectByMaterial)