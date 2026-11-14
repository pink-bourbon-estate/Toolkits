# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Quick Measure
# Section: object_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.quick_measure' does not match convention 'bt.object_tools_quick_measure'

import bpy
from mathutils import Vector
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 12. QUICK MEASURE (TAPE MEASURE INSPECTOR)
# -----------------------------------------------------------------------------
# What it does: Calculates the exact Width, Depth, and Height (X, Y, Z in metres)
# of each selected object and prints the measurements in Blender's info bar!
class BT_OT_QuickMeasure(Operator):
    bl_idname  = "bt.quick_measure"
    bl_label   = "Quick Measure"
    bl_description = "Print bounding-box dimensions of selected objects to Info bar"
    bl_options = {'REGISTER'}  # No UNDO needed here because this tool only reads data, it doesn't change anything!

    def execute(self, ctx):
        for obj in ctx.selected_objects:
            # Transform all 8 bounding box corners into world coordinates:
            bb = [obj.matrix_world @ Vector(c) for c in obj.bound_box]
            
            # Subtract minimum coordinate from maximum coordinate on each axis to find size:
            dims = Vector((
                max(v.x for v in bb) - min(v.x for v in bb),
                max(v.y for v in bb) - min(v.y for v in bb),
                max(v.z for v in bb) - min(v.z for v in bb),
            ))
            # NUANCE: '.3f' prints decimals to 3 decimal places (e.g. 1.250 m)
            self.report({'INFO'}, f"{obj.name}: X={dims.x:.3f}  Y={dims.y:.3f}  Z={dims.z:.3f} m")
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_QuickMeasure)

def unregister():
    bpy.utils.unregister_class(BT_OT_QuickMeasure)