# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Swap Materials
# Section: materials
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.swap_materials' does not match convention 'bt.materials_swap_materials'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 2. SWAP MATERIALS (TRADING COATS BETWEEN TWO OBJECTS)
# -----------------------------------------------------------------------------
# What it does: Select two models (like a sports car and a taxi), press this button,
# and they trade their paint jobs with each other!
class BT_OT_SwapMaterials(Operator):
    bl_idname = "bt.swap_materials"
    bl_label  = "Swap Materials"
    bl_description = "Swap materials between exactly 2 selected objects"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        objs = [o for o in ctx.selected_objects if o.type == 'MESH']

        # NUANCE: Strict Selection Guard!
        # Swapping only makes sense between exactly TWO items.
        # If the user selected 1 or 5 objects, warn them and cancel:
        if len(objs) != 2:
            self.report({'WARNING'}, "Select exactly 2 mesh objects")
            return {'CANCELLED'}

        # Unpack the two objects into variables 'a' and 'b':
        a, b = objs

        # Grab the materials currently sitting in slot 0 on each object:
        mat_a = a.material_slots[0].material if a.material_slots else None
        mat_b = b.material_slots[0].material if b.material_slots else None

        # Ensure both objects have at least one slot ready to receive paint:
        if not a.material_slots: a.data.materials.append(None)
        if not b.material_slots: b.data.materials.append(None)

        # NUANCE: The Classic Swap!
        # In Python, we assign b's paint to a, and a's paint to b:
        a.material_slots[0].material = mat_b
        b.material_slots[0].material = mat_a

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_SwapMaterials)

def unregister():
    bpy.utils.unregister_class(BT_OT_SwapMaterials)