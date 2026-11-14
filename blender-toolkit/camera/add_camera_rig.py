# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Add Camera Rig
# Section: camera
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.add_camera_rig' does not match convention 'bt.camera_add_camera_rig'

import bpy
import math
from mathutils import Euler
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 6. ADD CAMERA RIG (SMART TRACKING CAM)
# -----------------------------------------------------------------------------
# What it does: Creates a camera that automatically points at an invisible target marker
# (called an 'Empty'). Wherever you drag the target marker, the camera turns to watch it,
# like a robotic security camera!
class BT_OT_AddCameraRig(Operator):
    """Create a camera with a target empty and a Track To constraint (V4)."""
    bl_idname = "bt.add_camera_rig"
    bl_label  = "Add Camera Rig"
    bl_description = "Create a camera at (0, -8, 2) tracking a target empty at the origin"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # 1. CREATE THE TARGET (An invisible crosshair marker called an 'Empty')
        bpy.ops.object.empty_add(type='PLAIN_AXES', location=(0, 0, 0))
        target = ctx.active_object
        target.name = "BT_CamTarget"

        # 2. CREATE THE CAMERA (Placed back at Y=-8, Z=2)
        bpy.ops.object.camera_add(location=(0, -8, 2))
        cam_obj = ctx.active_object
        cam_obj.name = "BT_Camera"

        # Rough initial tilt upward:
        cam_obj.rotation_euler = Euler((math.radians(80), 0, 0))

        # 3. ADD THE 'TRACK TO' CONSTRAINT (The Magic Cable!)
        # NUANCE: Constraints in Blender
        # Constraints are dynamic rules. Here, we tell the camera:
        # "Always point your lens (-Z axis) directly at the target marker,
        # with your roof (+Y axis) facing upward toward the sky"!
        constraint = cam_obj.constraints.new(type='TRACK_TO')
        constraint.target     = target
        constraint.track_axis = 'TRACK_NEGATIVE_Z'
        constraint.up_axis    = 'UP_Y'

        # 4. Make this camera the official active camera for rendering pictures:
        ctx.scene.camera = cam_obj

        self.report({'INFO'}, "Camera rig created: BT_Camera tracking BT_CamTarget")
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_AddCameraRig)

def unregister():
    bpy.utils.unregister_class(BT_OT_AddCameraRig)