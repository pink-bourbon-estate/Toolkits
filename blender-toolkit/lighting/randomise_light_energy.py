# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Randomise Light Energy
# Section: lighting
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.randomise_light_energy' does not match convention 'bt.lighting_randomise_light_energy'

import bpy
import random
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 6. RANDOMISE LIGHT ENERGY (DYNAMIC BRIGHTNESS RANGE)
# -----------------------------------------------------------------------------
# What it does: Randomises the power (Watts) of selected lights between the
# minimum and maximum numbers set on your sidebar slider!
class BT_OT_RandomiseLightEnergy(Operator):
    bl_idname = "bt.randomise_light_energy"
    bl_label  = "Randomise Light Energy"
    bl_description = "Set each selected light's energy to a random value in the given range"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        p = ctx.scene.blender_toolkit
        random.seed(random.randint(0, 99999))

        for obj in ctx.selected_objects:
            if obj.type == 'LIGHT':
                # 'random.uniform(min, max)' picks a decimal number inside the user's boundaries:
                obj.data.energy = random.uniform(p.light_energy_min, p.light_energy_max)

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_RandomiseLightEnergy)

def unregister():
    bpy.utils.unregister_class(BT_OT_RandomiseLightEnergy)