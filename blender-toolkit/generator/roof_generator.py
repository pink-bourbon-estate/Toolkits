# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Roof Generator
# Section: generators
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.roof_generator' does not match convention 'bt.generators_roof_generator'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 4. ROOF GENERATOR (INSTANT ARCHITECTURE)
# -----------------------------------------------------------------------------
# What it does: Takes any flat building outline shape and automatically pulls it
# upward into a pitched roof with an overhang!
class BT_OT_RoofGenerator(Operator):
    bl_idname = "bt.roof_generator"
    bl_label  = "Roof Generator"
    bl_description = "Generate a roof from selected polygon footprint via Geometry Nodes"
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
            # Attach the roof modifier and extrude the top edge into a ridge:
            if core_mod:
                mod = core_mod._get_or_create_gn_mod(obj, "BT_RoofGenerator")
                core_mod._build_roof_gn(mod, p.roof_height, p.roof_overhang, p.roof_type)
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_RoofGenerator)

def unregister():
    bpy.utils.unregister_class(BT_OT_RoofGenerator)