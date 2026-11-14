# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Focus On Selection
# Section: camera
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.focus_on_selection' does not match convention 'bt.camera_focus_on_selection'

import bpy
from mathutils import Vector
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 7. FOCUS CAMERA ON SELECTION (AUTO-FRAME THE SHOT)
# -----------------------------------------------------------------------------
# What it does: Calculates the exact 3D midpoint of whatever objects you have selected,
# and pans the camera to look straight at the centre of them!
class BT_OT_FocusOnSelection(Operator):
    """Set the scene camera to look at the bounding box centre of selected objects (V4)."""
    bl_idname = "bt.focus_on_selection"
    bl_label  = "Focus Camera on Selection"
    bl_description = "Move the scene camera's Track To target to the centre of selected objects"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # Only inspect objects that actually have 3D geometry:
        objs = [o for o in ctx.selected_objects if o.type in {'MESH','CURVE','SURFACE','META','FONT'}]
        if not objs:
            self.report({'WARNING'}, "Select at least one mesh object")
            return {'CANCELLED'}

        # NUANCE: Calculating a Group Centre Point!
        # Gather all corner coordinates from all selected objects:
        all_verts = []
        for obj in objs:
            all_verts.extend([obj.matrix_world @ Vector(c) for c in obj.bound_box])
        
        # Calculate the mathematical average (mean) of all X, Y, and Z positions:
        cx = sum(v.x for v in all_verts) / len(all_verts)
        cy = sum(v.y for v in all_verts) / len(all_verts)
        cz = sum(v.z for v in all_verts) / len(all_verts)
        centre = Vector((cx, cy, cz))

        cam = ctx.scene.camera
        if not cam:
            self.report({'WARNING'}, "No active scene camera")
            return {'CANCELLED'}

        # Case A: If the camera is using a 'TRACK TO' rig, move its tracking target to the centre!
        target_moved = False
        for constraint in cam.constraints:
            if constraint.type == 'TRACK_TO' and constraint.target:
                constraint.target.location = centre
                target_moved = True
                break

        # Case B: If it's a plain camera without a rig, manually swivel it to look at the centre!
        if not target_moved:
            direction = centre - cam.location
            # NUANCE: Quaternion Tracking (.to_track_quat)
            # Quaternions are advanced 4-number maths used by 3D software to calculate smooth rotations 
            # without getting "gimbal lock" (where axes accidentally collide and freeze up)!
            rot_quat = direction.to_track_quat('-Z', 'Y')
            cam.rotation_euler = rot_quat.to_euler()

        self.report({'INFO'}, f"Camera focused on {centre.x:.2f}, {centre.y:.2f}, {centre.z:.2f}")
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_FocusOnSelection)

def unregister():
    bpy.utils.unregister_class(BT_OT_FocusOnSelection)