# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Simple Scatter
# Section: generators
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.simple_scatter' does not match convention 'bt.generators_simple_scatter'

import bpy
import random
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 3. SIMPLE SCATTER (SURFACE SPRINKLER)
# -----------------------------------------------------------------------------
# What it does: Scatters copies of the selected model across a surface 
# (like sprinkling rocks onto ground terrain, or trees across a hill)!
class BT_OT_SimpleScatter(Operator):
    bl_idname = "bt.simple_scatter"
    bl_label  = "Simple Scatter"
    bl_description = "Scatter instances of selection onto the active surface via Geometry Nodes"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # Dynamically import GN builders from core
        import sys
        if __package__:
            core_mod = sys.modules.get(__package__.split('.')[0] + ".blender_toolkit_core")
        else:
            core_mod = sys.modules.get("blender_toolkit_core")

        p = ctx.scene.blender_toolkit
        # Roll a random seed so the scatter distribution looks fresh:
        seed = random.randint(0, 99999)
        p.gen_sc_seed = seed

        for obj in ctx.selected_objects:
            if obj.type != 'MESH': 
                continue
            # Attach the scatter modifier and plug in the count, scale, and rotation limits:
            if core_mod:
                mod = core_mod._get_or_create_gn_mod(obj, "BT_SimpleScatter")
                core_mod._build_scatter_gn(mod, p.gen_sc_count, seed, p.gen_sc_scl_min, p.gen_sc_scl_max, p.gen_sc_rot_z)
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_SimpleScatter)

def unregister():
    bpy.utils.unregister_class(BT_OT_SimpleScatter)