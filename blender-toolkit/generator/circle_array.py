# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Circle Array
# Section: generators
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.circle_array' does not match convention 'bt.generators_circle_array'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 1. CIRCLE ARRAY (CIRCULAR MULTIPLIER)
# -----------------------------------------------------------------------------
# What it does: Clones the selected model and arranges the copies in a perfect
# circle (like seats on a Ferris wheel or numbers on a clock)!
class BT_OT_CircleArray(Operator):
    bl_idname = "bt.circle_array"
    bl_label  = "Circle Array"
    bl_description = "Array instances of selected object in a circle via Geometry Nodes"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # Dynamically import GN builders from core
        import sys
        if __package__:
            core_mod = sys.modules.get(__package__.split('.')[0] + ".blender_toolkit_core")
        else:
            core_mod = sys.modules.get("blender_toolkit_core")

        p = ctx.scene.blender_toolkit
        for obj in ctx.selected_objects:
            # 1. Attach the Geometry Nodes modifier to the object:
            if core_mod:
                mod = core_mod._get_or_create_gn_mod(obj, "BT_CircleArray")

                # 2. Wire up the internal nodes using the user's chosen count, radius, and axis:
                core_mod._build_circle_array_gn(mod, p.gen_ca_count, p.gen_ca_radius, p.gen_ca_axis, p.gen_ca_align)
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_CircleArray)

def unregister():
    bpy.utils.unregister_class(BT_OT_CircleArray)