# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Quick Rotate
# Section: transform
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.quick_rotate' does not match convention 'bt.transform_quick_rotate'

import bpy
import math
from bpy.props import EnumProperty, FloatProperty
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 6. QUICK ROTATE (FAST 90° / 45° TURNS)
# -----------------------------------------------------------------------------
class BT_OT_QuickRotate(Operator):
    bl_idname  = "bt.quick_rotate"
    bl_label   = "Quick Rotate"
    bl_description = "Rotate selection by a preset angle on a chosen axis"
    bl_options = {'REGISTER', 'UNDO'}

    # Operator Properties: When running this operator, you can pass arguments like (axis='Z', angle=90.0)
    axis:   EnumProperty(items=[('X','X',''),('Y','Y',''),('Z','Z','')], default='Z')
    angle:  FloatProperty(name="Angle", default=90.0)

    def execute(self, ctx):
        # Computers measure angles in Radians, but humans think in Degrees!
        # 'math.radians()' converts human degrees (like 90°) into computer radians (1.5708...):
        rad = math.radians(self.angle)

        for obj in ctx.selected_objects:
            if self.axis == 'X':   obj.rotation_euler.x += rad
            elif self.axis == 'Y': obj.rotation_euler.y += rad
            else:                  obj.rotation_euler.z += rad

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_QuickRotate)

def unregister():
    bpy.utils.unregister_class(BT_OT_QuickRotate)