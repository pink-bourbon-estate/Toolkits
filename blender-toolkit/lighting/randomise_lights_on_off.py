# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Randomise Lights On Off
# Section: lighting
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.randomise_lights_on_off' does not match convention 'bt.lighting_randomise_lights_on_off'

import bpy
import random
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 8. RANDOMISE LIGHTS ON / OFF (THE 50/50 COIN FLIP)
# -----------------------------------------------------------------------------
# What it does: Loops through every lamp and flips a virtual coin:
# Heads = Light stays on! Tails = Light is turned off in the final picture!
# Awesome for skyscrapers at night where only some office windows are lit up.
class BT_OT_RandomiseLightsOnOff(Operator):
    bl_idname = "bt.randomise_lights_on_off"
    bl_label  = "Randomise Lights On/Off"
    bl_description = "Randomly toggle render visibility on each light in the scene (50/50)"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        random.seed(random.randint(0, 99999))
        for obj in ctx.scene.objects:
            if obj.type == 'LIGHT':
                # NUANCE: Boolean from Comparison (< 0.5)
                # 'random.random()' generates a decimal between 0.0 and 1.0.
                # 'random.random() < 0.5' has an exact 50% chance of being True or False!
                obj.hide_render = random.random() < 0.5

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_RandomiseLightsOnOff)

def unregister():
    bpy.utils.unregister_class(BT_OT_RandomiseLightsOnOff)