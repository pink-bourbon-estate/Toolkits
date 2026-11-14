# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Apply UV Grid
# Section: uv
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.apply_uv_grid' does not match convention 'bt.uv_apply_uv_grid'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 4. APPLY UV GRID (THE CHECKERBOARD TEST MATERIAL)
# -----------------------------------------------------------------------------
# What it does: Creates a colourful chessboard/grid pattern texture and applies
# it to the model. 3D artists use checkerboards to check for stretching:
# if any squares look squashed into rectangles, the UV mapping needs adjusting!
class BT_OT_ApplyUVGrid(Operator):
    """Create a UV grid texture and assign it as a new material on selected objects (V4)."""
    bl_idname = "bt.apply_uv_grid"
    bl_label  = "UV Grid Texture"
    bl_description = "Create a 1024 UV grid image and assign it as a material on selected objects"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # 1. CREATE OR REUSE THE CHECKERBOARD IMAGE
        img_name = "BT_UVGrid"
        # Check if the image was already generated previously so we don't duplicate images in RAM:
        img = bpy.data.images.get(img_name)
        if not img:
            # Generate a fresh 1024x1024 pixel checkerboard pattern:
            bpy.ops.image.new(
                name=img_name,
                width=1024,
                height=1024,
                generated_type='UV_GRID',  # Blender's built-in numbered grid pattern
            )
            img = bpy.data.images.get(img_name)

        # 2. CREATE OR REUSE THE SHADER MATERIAL
        mat_name = "BT_UVGrid_Mat"
        mat = bpy.data.materials.get(mat_name)
        if not mat:
            # Build a new material that uses node shaders:
            mat = bpy.data.materials.new(name=mat_name)
            mat.use_nodes = True
            nodes = mat.node_tree.nodes
            links = mat.node_tree.links
            nodes.clear()  # Clear starter nodes

            # Create the 3 essential material nodes:
            out  = nodes.new('ShaderNodeOutputMaterial') # Final surface output
            bsdf = nodes.new('ShaderNodeBsdfPrincipled')     # The main light/colour shader
            tex  = nodes.new('ShaderNodeTexImage')          # The texture image loader
            tex.image = img                                  # Slot our checkerboard picture into the node

            # Place them neatly across the screen:
            out.location  = (300, 0)
            bsdf.location = (0, 0)
            tex.location  = (-300, 0)

            # Connect Image Colour -> Principled Base Colour -> Material Surface:
            links.new(tex.outputs['Color'], bsdf.inputs['Base Color'])
            links.new(bsdf.outputs['BSDF'], out.inputs['Surface'])

        # 3. ASSIGN THE MATERIAL TO SELECTED OBJECTS
        count = 0
        for obj in ctx.selected_objects:
            if obj.type != 'MESH':
                continue
            # If the object has no material slots, add one; otherwise replace slot 0:
            if not obj.data.materials:
                obj.data.materials.append(mat)
            else:
                obj.material_slots[0].material = mat
            count += 1

        self.report({'INFO'}, f"Applied UV grid material to {count} object(s)")
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_ApplyUVGrid)

def unregister():
    bpy.utils.unregister_class(BT_OT_ApplyUVGrid)