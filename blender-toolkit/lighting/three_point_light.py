# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Three Point Light
# Section: lighting
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.three_point_light' does not match convention 'bt.lighting_three_point_light'

import bpy
import math
from bpy.types import Operator

def _add_light(ctx, name, ltype, energy, loc, rot, color=(1,1,1), radius=0.5):
    from mathutils import Euler
    bpy.ops.object.light_add(type=ltype, location=loc)
    light = ctx.active_object
    light.name = name
    light.data.energy = energy
    light.data.color  = color
    light.rotation_euler = Euler(rot)
    if hasattr(light.data, 'shadow_soft_size'): light.data.shadow_soft_size = radius
    return light

# -----------------------------------------------------------------------------
# 2. THREE-POINT LIGHTING (THE HOLLYWOOD CLASSIC)
# -----------------------------------------------------------------------------
# What it does: The most famous photography lighting trick in the world!
#   1. KEY LIGHT: The main, brightest lamp (1000W) shining from the front-right.
#   2. FILL LIGHT: A softer lamp (300W) on the front-left to soften harsh shadows.
#   3. BACK / RIM LIGHT: A lamp behind the subject (500W) creating a glowing edge 
#      so the character pops out from the background!
class BT_OT_ThreePointLight(Operator):
    bl_idname = "bt.three_point_light"
    bl_label  = "3-Point Lighting"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # Coordinates: (X = left/right, Y = forward/back, Z = up/down)
        # Rotations use math.radians() to convert human degrees into computer math!
        _add_light(ctx, "BT_Key",   'AREA', 1000, (4,  -4, 6), (math.radians(60), 0, math.radians(45)))
        _add_light(ctx, "BT_Fill",  'AREA',  300, (-4, -3, 4), (math.radians(50), 0, math.radians(-45)))
        _add_light(ctx, "BT_Back",  'AREA',  500, (0,   4, 5), (math.radians(-45),0, math.radians(180)))
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_ThreePointLight)

def unregister():
    bpy.utils.unregister_class(BT_OT_ThreePointLight)