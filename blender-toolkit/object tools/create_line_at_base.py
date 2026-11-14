# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Create Line At Base
# Section: object_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.create_line_at_base' does not match convention 'bt.object_tools_create_line_at_base'

import bpy
from mathutils import Vector
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 4. CREATE LINE AT BASE (DRAW A FOOTPRINT OUTLINE)
# -----------------------------------------------------------------------------
# What it does: Draws a closed 3D rectangular wire frame on the ground directly
# underneath each object! Architectural visualisers use this to see the floor
# "footprint" of furniture and walls.
class BT_OT_CreateLineAtBase(Operator):
    bl_idname = "bt.create_line_at_base"
    bl_label  = "Create Base Rectangle"
    bl_description = "Create a rectangle curve at the bottom bounding box of each selected object"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        for obj in list(ctx.selected_objects):
            if obj.type not in {'MESH', 'CURVE'}:
                continue

            # 1. Transform the 8 bounding-box corner points into global world positions:
            bb = [obj.matrix_world @ Vector(c) for c in obj.bound_box]

            # 2. Find the minimum and maximum boundaries along X, Y, and Z:
            min_x = min(v.x for v in bb); max_x = max(v.x for v in bb)
            min_y = min(v.y for v in bb); max_y = max(v.y for v in bb)
            min_z = min(v.z for v in bb)  # The lowest point (the bottom floor level)

            # 3. Create a brand new 3D Curve data block:
            curve_data = bpy.data.curves.new(f"{obj.name}_Base", 'CURVE')
            curve_data.dimensions = '3D'

            # 4. Add a polygonal line (spline) inside the curve:
            spline = curve_data.splines.new('POLY')

            # NUANCE: spline.points.add(3)
            # A fresh spline starts with 1 point by default.
            # Adding 3 more gives us 4 corner points in total (a complete rectangle)!
            spline.points.add(3)

            # Define the four bottom corners:
            pts = [
                (min_x, min_y, min_z),  # Bottom-Left-Front
                (max_x, min_y, min_z),  # Bottom-Right-Front
                (max_x, max_y, min_z),  # Bottom-Right-Back
                (min_x, max_y, min_z)   # Bottom-Left-Back
            ]

            # Assign each coordinate to a spline point (X, Y, Z, Weight):
            for i, (x, y, z) in enumerate(pts):
                spline.points[i].co = (x, y, z, 1)

            # NUANCE: use_cyclic_u = True
            # 'cyclic' connects the final 4th point back to the 1st point to close the loop!
            spline.use_cyclic_u = True

            # 5. Build the object and link it to the scene so it appears in the 3D viewport:
            new_obj = bpy.data.objects.new(f"{obj.name}_Base", curve_data)
            ctx.collection.objects.link(new_obj)

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_CreateLineAtBase)

def unregister():
    bpy.utils.unregister_class(BT_OT_CreateLineAtBase)