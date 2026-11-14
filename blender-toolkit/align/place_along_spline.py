# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Place Along Spline
# Section: align
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.place_along_spline' does not match convention 'bt.align_place_along_spline'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 4. PLACE ALONG SPLINE (BEADS ON A STRING)
# -----------------------------------------------------------------------------
# What it does: Takes selected objects and spaces them out along a 3D curve line
# (like street lamps down a winding road, or fence pickets along a boundary).
class BT_OT_PlaceAlongSpline(Operator):
    bl_idname  = "bt.place_along_spline"
    bl_label   = "Place Along Spline"
    bl_description = "Distribute selected objects evenly along the active curve"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        curve = ctx.active_object
        if not curve or curve.type != 'CURVE':
            self.report({'WARNING'}, "Active object must be a curve")
            return {'CANCELLED'}

        # Filter out the curve itself so we only move the objects to be distributed:
        objs = [o for o in ctx.selected_objects if o != curve]
        if not objs:
            return {'CANCELLED'}

        # Extract coordinates of all control points along the first curve spline:
        spline = curve.data.splines[0]
        pts = [curve.matrix_world @ p.co.xyz for p in spline.bezier_points] if spline.type == 'BEZIER' \
              else [curve.matrix_world @ p.co.xyz for p in spline.points]

        n = len(objs)
        for i, obj in enumerate(objs):
            # 't' is our percentage along the curve, from 0.0 (start) to 1.0 (end):
            t = i / max(n-1, 1)

            # Find the two control points that our object sits between:
            idx = min(int(t * (len(pts)-1)), len(pts)-2)
            frac = t * (len(pts)-1) - idx

            # NUANCE: Linear Interpolation ('lerp')!
            # 'lerp' smoothly calculates any point between point A and point B
            # based on a fractional percentage (frac).
            obj.location = pts[idx].lerp(pts[idx+1], frac)

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_PlaceAlongSpline)

def unregister():
    bpy.utils.unregister_class(BT_OT_PlaceAlongSpline)