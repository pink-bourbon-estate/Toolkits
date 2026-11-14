# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Remove Materials
# Section: materials
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.remove_materials' does not match convention 'bt.materials_remove_materials'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 9. REMOVE MATERIALS (STRIP TO BARE MESH)
# -----------------------------------------------------------------------------
# What it does: Wipes away all material slots on selected models, resetting them
# to bare grey geometry.
class BT_OT_RemoveMaterials(Operator):
    bl_idname = "bt.remove_materials"
    bl_label  = "Remove All Materials"
    bl_description = "Strip all material slots from selected objects"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        for obj in ctx.selected_objects:
            # Clears the entire list of materials assigned to this mesh:
            obj.data.materials.clear()
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_RemoveMaterials)

def unregister():
    bpy.utils.unregister_class(BT_OT_RemoveMaterials)