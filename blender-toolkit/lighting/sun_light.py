# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Sun Light
# Section: lighting
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.sun_light' does not match convention 'bt.lighting_sun_light'

import bpy
import math
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 4. SUN LIGHT (OUTDOOR SKY LIGHT)
# -----------------------------------------------------------------------------
# What it does: Spawns a 'SUN' light source high in the sky (Z=10) tilted at a 45° angle,
# casting crisp daylight across the entire world!
class BT_OT_SunLight(Operator):
    bl_idname = "bt.sun_light"
    bl_label  = "Add Sun"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # Dynamically import GN builders from core
        import sys
        if __package__:
            core_mod = sys.modules.get(__package__.split('.')[0] + ".blender_toolkit_core")
        else:
            core_mod = sys.modules.get("blender_toolkit_core")

        if core_mod:
            core_mod._add_light(ctx, "BT_Sun", 'SUN', 5, (0,0,10), (math.radians(-45), 0, math.radians(45)))
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_SunLight)

def unregister():
    bpy.utils.unregister_class(BT_OT_SunLight)