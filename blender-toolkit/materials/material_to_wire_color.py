# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Material To Wire Color
# Section: materials
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.material_to_wire_color' does not match convention 'bt.materials_material_to_wire_color'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 11. MATERIAL TO WIRE COLOUR (MATCH OUTLINES TO PAINT)
# -----------------------------------------------------------------------------
# What it does: Reads the base diffuse colour of each object's material and sets
# the object's viewport outline/wire colour to match! Great for organising complex
# scenes when working in wireframe or solid display mode.
class BT_OT_MaterialToWireColor(Operator):
    bl_idname = "bt.material_to_wire_color"
    bl_label  = "Material → Wire Colour"
    bl_description = "Set each object's wire colour to match its material's base colour"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        for obj in ctx.selected_objects:
            if not obj.material_slots:
                continue
            mat = obj.material_slots[0].material
            if not mat:
                continue

            # Check if this material uses shader nodes (modern Principled BSDF):
            if mat.node_tree:
                for node in mat.node_tree.nodes:
                    # Find the main shader block:
                    if node.type == 'BSDF_PRINCIPLED':
                        # Read the 'Base Color' socket (Red, Green, Blue, Alpha):
                        col = node.inputs['Base Color'].default_value
                        # Assign that colour to the object's display property in RGBA format:
                        obj.color = (col[0], col[1], col[2], 1.0)
                        break
            else:
                # Fallback for simple materials that don't use nodes:
                obj.color = (mat.diffuse_color[0], mat.diffuse_color[1], mat.diffuse_color[2], 1.0)

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_MaterialToWireColor)

def unregister():
    bpy.utils.unregister_class(BT_OT_MaterialToWireColor)