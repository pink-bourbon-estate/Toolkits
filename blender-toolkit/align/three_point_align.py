# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Three Point Align
# Section: align
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.three_point_align' does not match convention 'bt.align_three_point_align'

import bpy
import math
from mathutils import Vector, Euler, Matrix
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 5. THREE-POINT ALIGN (PLANE ROTATION ALIGNMENT)
# -----------------------------------------------------------------------------
# What it does: Takes 3 coordinates, calculates the angle of the triangle they form,
# and tilts the active object so its top (Z axis) matches that plane!
class BT_OT_ThreePointAlign(Operator):
    """Align the active object's rotation to a plane defined by 3 points (V4)."""
    bl_idname  = "bt.three_point_align"
    bl_label   = "3-Point Align"
    bl_description = "Rotate active object to match the normal of the plane defined by P1, P2, P3"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        obj = ctx.active_object
        if not obj:
            self.report({'WARNING'}, "No active object")
            return {'CANCELLED'}

        p = ctx.scene.blender_toolkit
        p1 = Vector(p.tpa_p1)
        p2 = Vector(p.tpa_p2)
        p3 = Vector(p.tpa_p3)

        # Build two directional vectors connecting point 1 to points 2 and 3:
        v1 = p2 - p1
        v2 = p3 - p1

        # NUANCE: The Vector Cross Product (.cross())!
        # If you draw two lines on a piece of paper, their 'cross product' produces
        # a vector pointing directly perpendicular away from the paper.
        # This perpendicular direction is called the 'Surface Normal'.
        normal = v1.cross(v2)

        # Guard check: If the points sit in a straight line, they cannot define a flat plane!
        # '1e-6' means 0.000001 (scientific notation for a tiny threshold):
        if normal.length < 1e-6:
            self.report({'WARNING'}, "Points are collinear — cannot define a plane")
            return {'CANCELLED'}

        # 'normalize()' scales vector length to 1.0, preserving pure direction:
        normal.normalize()

        # Target: Match Blender's standard Up vector (Z = 0, 0, 1):
        z_axis = Vector((0, 0, 1))

        # Find the hinge axis needed to tilt Z toward our new surface normal:
        rot_axis = z_axis.cross(normal)

        if rot_axis.length < 1e-6:
            # If length is zero, the normal is already pointing straight up or down:
            if normal.dot(z_axis) < 0:
                obj.rotation_euler = Euler((math.pi, 0, 0))  # Flip 180° upside down
            else:
                obj.rotation_euler = Euler((0, 0, 0))        # Keep right-side up
        else:
            rot_axis.normalize()
            # Calculate rotation angle using dot product and inverse cosine (acos):
            angle = math.acos(max(-1.0, min(1.0, z_axis.dot(normal))))
            # Construct a 4x4 3D Rotation Matrix and convert to Euler angles:
            rot_mat = Matrix.Rotation(angle, 4, rot_axis)
            obj.rotation_euler = rot_mat.to_euler()

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_ThreePointAlign)

def unregister():
    bpy.utils.unregister_class(BT_OT_ThreePointAlign)