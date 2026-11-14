# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Apply Default Material
# Section: materials
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.apply_default_material' does not match convention 'bt.materials_apply_default_material'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 1. APPLY DEFAULT MATERIAL (THE PRIMER COAT)
# -----------------------------------------------------------------------------
# What it does: Scans selected models and, if an object has no material assigned,
# applies a clean standard grey shader named "BT_Default"!
class BT_OT_ApplyDefaultMaterial(Operator):
    bl_idname = "bt.apply_default_material"
    bl_label  = "Apply Default Material"
    bl_description = "Assign a BT_Default Principled BSDF material to objects with no material"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # 1. Look for an existing "BT_Default" material so we don't recreate it every time:
        mat = bpy.data.materials.get("BT_Default")
        if not mat:
            # If it doesn't exist yet, build a fresh one with nodes enabled:
            mat = bpy.data.materials.new("BT_Default")
            mat.use_nodes = True

        count = 0
        for obj in ctx.selected_objects:
            if obj.type != 'MESH':
                continue

            # NUANCE: Double-Checking Material Slots!
            # An object can have material slots that are completely empty (None).
            # 'not any(s.material for s in obj.material_slots)' checks whether every
            # slot is empty or missing paint!
            if not any(s.material for s in obj.material_slots):
                # If there are no slots at all, add one with the material:
                if not obj.material_slots:
                    obj.data.materials.append(mat)
                else:
                    # If an empty slot was already sitting there, assign to slot 0:
                    obj.material_slots[0].material = mat
                count += 1

        self.report({'INFO'}, f"Assigned BT_Default to {count} objects")
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_ApplyDefaultMaterial)

def unregister():
    bpy.utils.unregister_class(BT_OT_ApplyDefaultMaterial)