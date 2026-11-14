# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Re-Randomise
# Section: transform
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.re_randomise' does not match convention 'bt.transform_re_randomise'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 2. RE-RANDOMISE (REROLL THE DICE)
# -----------------------------------------------------------------------------
class BT_OT_ReRandomise(Operator):
    bl_idname = "bt.re_randomise"
    bl_label  = "Re-Randomise"
    bl_description = "Apply a fresh random seed and re-run randomise"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # NUANCE: Running an operator from another operator!
        # Instead of repeating the math code, this button simply tells Blender:
        # "Run the BT_OT_RandomTransform operator again!"
        return bpy.ops.bt.random_transform()

def register():
    bpy.utils.register_class(BT_OT_ReRandomise)

def unregister():
    bpy.utils.unregister_class(BT_OT_ReRandomise)