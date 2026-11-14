# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Studio Light
# Section: lighting
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.studio_light' does not match convention 'bt.lighting_studio_light'

import bpy
import math
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 3. STUDIO LIGHTING (CLEAN PRODUCT SHOWCASE)
# -----------------------------------------------------------------------------
# What it does: Creates balanced, clean commercial lighting (Top, Front, and Side)
# perfect for showcasing cars, sneakers, or 3D character sculpts!
class BT_OT_StudioLight(Operator):
    bl_idname = "bt.studio_light"
    bl_label  = "Studio Lighting"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # Dynamically import GN builders from core
        import sys
        if __package__:
            core_mod = sys.modules.get(__package__.split('.')[0] + ".blender_toolkit_core")
        else:
            core_mod = sys.modules.get("blender_toolkit_core")

        if core_mod:
            core_mod._add_light(ctx, "BT_Top",   'AREA', 800,  (0, 0,  8), (0, 0, 0))
            core_mod._add_light(ctx, "BT_Front", 'AREA', 400,  (0, -6, 4), (math.radians(45), 0, 0))
            core_mod._add_light(ctx, "BT_Right", 'AREA', 200,  (6,  0, 3), (0, math.radians(-45), 0))
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_StudioLight)

def unregister():
    bpy.utils.unregister_class(BT_OT_StudioLight)