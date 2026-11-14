# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Set Pivot Bottom
# Section: transform
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.set_pivot_bottom' does not match convention 'bt.transform_set_pivot_bottom'

import bpy
from mathutils import Vector
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 7. SET PIVOT TO BOTTOM (CENTRE THE ORIGIN AT THE BASE)
# -----------------------------------------------------------------------------
class BT_OT_SetPivotBottom(Operator):
    bl_idname  = "bt.set_pivot_bottom"
    bl_label   = "Pivot to Bottom"
    bl_description = "Set object origin to the bottom-centre of the bounding box"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # NUANCE: What is an Object Origin / Pivot Point?
        # The origin is the tiny orange dot an object rotates around and scales from.
        # Usually it is in the centre of a model, but for furniture, trees, or characters,
        # you want the origin at the bottom so they sit nicely on floors!
        for obj in ctx.selected_objects:
            # Get the 8 corners of the bounding box in world space:
            bbox = [obj.matrix_world @ Vector(c) for c in obj.bound_box]

            # 1. Find the lowest Z point (the bottom):
            min_z = min(v.z for v in bbox)

            # 2. Average the 8 corner X and Y coordinates to find the true horizontal centre:
            cx = sum(v.x for v in bbox) / 8
            cy = sum(v.y for v in bbox) / 8

            new_origin = Vector((cx, cy, min_z))

            # 3. Temporarily teleport Blender's 3D cursor to that bottom-centre point:
            ctx.scene.cursor.location = new_origin

            # 4. Tell Blender to snap the object's origin dot to the 3D cursor:
            bpy.ops.object.origin_set(type='ORIGIN_CURSOR')

        # Put the 3D cursor back to the world centre (0, 0, 0) to keep the scene tidy:
        ctx.scene.cursor.location = (0,0,0)
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_SetPivotBottom)

def unregister():
    bpy.utils.unregister_class(BT_OT_SetPivotBottom)