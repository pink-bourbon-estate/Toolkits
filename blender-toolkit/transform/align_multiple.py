# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Align Multiple Objects
# Section: align
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.align_multiple' does not match convention 'bt.align_align_multiple'

import bpy
from mathutils import Vector
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 1. ALIGN MULTIPLE OBJECTS (LINE THEM UP IN A ROW)
# -----------------------------------------------------------------------------
# What it does: Lines up a queue of objects one after another along the X, Y, or Z axis,
# taking their physical widths into account so they do not collide!
class BT_OT_AlignMultipleObjects(Operator):
    bl_idname  = "bt.align_multiple"
    bl_label   = "Align Multiple Objects"
    bl_description = "Align selected objects along an axis (order of selection preserved)"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        p = ctx.scene.blender_toolkit

        # Convert selected objects into a Python list:
        objs = list(ctx.selected_objects)

        # NUANCE: A Guard Clause!
        # You cannot line up a single object with other objects.
        # If fewer than 2 objects are selected, report a warning and stop execution:
        if len(objs) < 2:
            self.report({'WARNING'}, "Select 2+ objects")
            return {'CANCELLED'}

        axis = p.align_axis
        offset = p.align_offset
        use_bbox = p.align_bbox

        # NUANCE: Functions inside a function (Local Helper Functions)!
        # 'half_size' calculates how far the right/top side of an object sticks out
        # from its centre point along an axis:
        def half_size(obj, ax):
            bb = [obj.matrix_world @ Vector(c) for c in obj.bound_box]
            if ax == 'X': return max(v.x for v in bb) - obj.location.x
            if ax == 'Y': return max(v.y for v in bb) - obj.location.y
            return max(v.z for v in bb) - obj.location.z

        # 'half_size_neg' calculates how far the left/bottom side sticks out:
        def half_size_neg(obj, ax):
            bb = [obj.matrix_world @ Vector(c) for c in obj.bound_box]
            if ax == 'X': return obj.location.x - min(v.x for v in bb)
            if ax == 'Y': return obj.location.y - min(v.y for v in bb)
            return obj.location.z - min(v.z for v in bb)

        # NUANCE: Loop starting from Index 1!
        # 'range(1, len(objs))' starts at the second object (index 1).
        # We leave the first object (index 0) exactly where it is as our anchor point!
        for i in range(1, len(objs)):
            prev = objs[i-1]  # The previous object in the queue
            cur = objs[i]    # The current object we need to reposition

            # Calculate total separation gap:
            # (Half-width of previous + Half-width of current + User extra gap spacing):
            sep = (half_size(prev, axis) + half_size_neg(cur, axis) + offset) if use_bbox else offset

            # Push the current object right next to the previous one:
            if axis == 'X':   cur.location.x = prev.location.x + sep
            elif axis == 'Y': cur.location.y = prev.location.y + sep
            else:             cur.location.z = prev.location.z + sep

            # Keep the other two axis values locked to match the first anchor object:
            if axis != 'X': cur.location.x = objs[0].location.x
            if axis != 'Y': cur.location.y = objs[0].location.y
            if axis != 'Z': cur.location.z = objs[0].location.z

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_AlignMultipleObjects)

def unregister():
    bpy.utils.unregister_class(BT_OT_AlignMultipleObjects)