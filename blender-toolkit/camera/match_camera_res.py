# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Match Camera Res
# Section: camera
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.match_camera_res' does not match convention 'bt.camera_match_camera_res'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 8. MATCH CAMERA RESOLUTION (ONE-CLICK VIDEO SIZES)
# -----------------------------------------------------------------------------
# What it does: Automatically sets the render frame dimensions to standard 
# resolutions (Full HD, 2K QHD, or 4K Ultra HD) based on your dropdown selection!
class BT_OT_MatchCameraRes(Operator):
    """Set render resolution from a preset enum (V4)."""
    bl_idname = "bt.match_camera_res"
    bl_label  = "Set Resolution"
    bl_description = "Set render resolution to the chosen preset"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        p = ctx.scene.blender_toolkit
        
        # Dictionary mapping our dropdown codes to exact pixel dimensions (Width, Height):
        presets = {
            'HD':  (1920, 1080),  # 1080p (Standard YouTube / TV resolution)
            'QHD': (2560, 1440),  # 1440p (Crisp PC monitor resolution)
            '4K':  (3840, 2160),  # 2160p (High-end 4K cinema resolution)
        }
        
        # '.get()' grabs the resolution tuple; falls back to 1080p if not found:
        w, h = presets.get(p.cam_res, (1920, 1080))
        
        # Apply the pixel dimensions directly to Blender's render engine settings:
        ctx.scene.render.resolution_x = w
        ctx.scene.render.resolution_y = h
        
        self.report({'INFO'}, f"Resolution set to {w}×{h}")
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_MatchCameraRes)

def unregister():
    bpy.utils.unregister_class(BT_OT_MatchCameraRes)