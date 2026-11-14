# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Randomise Light Hue
# Section: lighting
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.randomise_light_hue' does not match convention 'bt.lighting_randomise_light_hue'

import bpy
import random
import colorsys
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 5. RANDOMISE LIGHT HUE (NEON LIGHT JITTER)
# -----------------------------------------------------------------------------
# What it does: Takes selected lights and gives each one a unique, bright colour tint!
# Great for sci-fi city streets, carnival rides, or disco dance floors.
class BT_OT_RandomiseLightHue(Operator):
    bl_idname = "bt.randomise_light_hue"
    bl_label  = "Randomise Light Hue"
    bl_description = "Assign a random colour to each selected light"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        random.seed(random.randint(0, 99999))
        for obj in ctx.selected_objects:
            if obj.type != 'LIGHT':
                continue

            # NUANCE: HSV Colour Tuning!
            # - h: Pick any random point on the colour rainbow (0.0 to 1.0).
            # - s: Keeps saturation between 40% and 80% (rich colour, but not blindingly radioactive)!
            # - v = 1.0: Full brightness!
            h = random.random()
            s = 0.4 + random.random() * 0.4
            r, g, b = colorsys.hsv_to_rgb(h, s, 1.0)

            # Assign the RGB tint to the lamp's data:
            obj.data.color = (r, g, b)

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_RandomiseLightHue)

def unregister():
    bpy.utils.unregister_class(BT_OT_RandomiseLightHue)