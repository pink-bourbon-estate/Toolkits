# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — UVW Box Map
# Section: uv
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.uvw_box_map' does not match convention 'bt.uv_uvw_box_map'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 1. UVW BOX MAP (TRIPLANAR PROJECTION VIA GEOMETRY NODES)
# -----------------------------------------------------------------------------
# What it does: Instead of carefully cutting seams on a 3D model, this uses
# our automated Geometry Nodes setup to project the texture onto the model
# from the Top, Front, and Sides at the exact scale requested!
class BT_OT_UVWBoxMap(Operator):
    bl_idname = "bt.uvw_box_map"
    bl_label  = "UVW Box Map"
    bl_description = "Apply triplanar box-map UV projection via Geometry Nodes"
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
            if obj.type != 'MESH':
                continue

            # NUANCE: Calling our helper functions!
            # 1. '_get_or_create_gn_mod' checks if the modifier is already on the object.
            # 2. '_build_uvw_box_gn' assembles the Lego node graph using the size and blend values!
            if core_mod:
                mod = core_mod._get_or_create_gn_mod(obj, "BT_UVWBoxMap")
                core_mod._build_uvw_box_gn(mod, p.gen_uvw_size, p.gen_uvw_blend)

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_UVWBoxMap)

def unregister():
    bpy.utils.unregister_class(BT_OT_UVWBoxMap)