# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Drop to Floor
# Section: transform
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.drop_to_floor' does not match convention 'bt.transform_drop_to_floor'

import bpy
from mathutils import Vector
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 3. DROP TO FLOOR (SNAP TO GROUND Z = 0)
# -----------------------------------------------------------------------------
class BT_OT_DropToFloor(Operator):
    bl_idname = "bt.drop_to_floor"
    bl_label  = "Drop to Floor"
    bl_description = "Move selected objects so their lowest point sits at Z=0"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        for obj in ctx.selected_objects:
            # NUANCE: Object Type Filter!
            # Cameras, lights, and empties don't have physical geometry.
            # 'continue' tells Python: "Skip this item and jump straight to the next one!"
            if obj.type not in {'MESH','CURVE','SURFACE','META','FONT'}:
                continue

            # NUANCE: World Matrix (@) & Bounding Box (bound_box)
            # 'obj.bound_box' gives the 8 corner coordinates of the object in its local space.
            # '@' is Python's matrix multiplication symbol!
            # 'obj.matrix_world @ Vector(c)' translates local corners into actual global 3D space.
            # 'min(...)' finds the lowest Z coordinate among all 8 corners:
            min_z = min((obj.matrix_world @ Vector(c)).z for c in obj.bound_box)

            # Shift the object downward by that exact amount so the base touches ground level (Z = 0):
            obj.location.z -= min_z

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_DropToFloor)

def unregister():
    bpy.utils.unregister_class(BT_OT_DropToFloor)