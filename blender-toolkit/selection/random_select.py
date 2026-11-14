# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Random Select
# Section: selection
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.random_select' does not match convention 'bt.selection_random_select'

import bpy
import random
from bpy.props import FloatProperty
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 7. RANDOM DESELECT (THINNING OUT A CROWD)
# -----------------------------------------------------------------------------
# What it does: If you have 500 trees selected and only want to keep 250 of them,
# this button flips a virtual coin for each one and randomly drops a percentage!
class BT_OT_RandomSelect(Operator):
    bl_idname = "bt.random_select"
    bl_label  = "Random Deselect"
    bl_description = "Randomly deselect objects from current selection"
    bl_options = {'REGISTER', 'UNDO'}

    # Slider for what percentage of your selection you want to keep (default 50%):
    keep_pct: FloatProperty(name="Keep %", default=50.0, min=0.0, max=100.0)

    def execute(self, ctx):
        random.seed(random.randint(0, 99999))

        # NUANCE: Why 'list(ctx.selected_objects)'?
        # If you loop over a collection while modifying it (turning off selection),
        # Python gets confused and skips items!
        # Wrapping it in 'list(...)' takes a fixed snapshot of the selection first so the loop stays safe.
        for obj in list(ctx.selected_objects):
            # 'random.random()' gives a float between 0.0 and 1.0.
            # Multiply by 100 to get a percentage between 0% and 100%.
            # If the roll is higher than keep_pct, drop it from the selection:
            if random.random() * 100 > self.keep_pct:
                obj.select_set(False)

        return {'FINISHED'}

    def invoke(self, ctx, event):
        return ctx.window_manager.invoke_props_dialog(self)

def register():
    bpy.utils.register_class(BT_OT_RandomSelect)

def unregister():
    bpy.utils.unregister_class(BT_OT_RandomSelect)