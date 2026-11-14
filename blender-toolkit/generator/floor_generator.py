# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Floor Generator
# Section: generators
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.floor_generator' does not match convention 'bt.generators_floor_generator'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 2. FLOOR GENERATOR (PARAMETRIC TILING)
# -----------------------------------------------------------------------------
# What it does: Spawns a brand new 3D object from thin air and builds a 
# customizable tiled floor with grout gaps and tile thickness!
class BT_OT_FloorGenerator(Operator):
    bl_idname = "bt.floor_generator"
    bl_label  = "Floor Generator"
    bl_description = "Generate a parametric tiled floor via Geometry Nodes"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # Dynamically import GN builders from core
        import sys
        if __package__:
            core_mod = sys.modules.get(__package__.split('.')[0] + ".blender_toolkit_core")
        else:
            core_mod = sys.modules.get("blender_toolkit_core")
            
        p = ctx.scene.blender_toolkit

        # NUANCE: Creating an Object from scratch!
        # Step 1: Create an empty mesh container named "BT_Floor".
        mesh = bpy.data.meshes.new("BT_Floor")
        # Step 2: Wrap that mesh into an actual 3D Scene Object.
        obj  = bpy.data.objects.new("BT_Floor", mesh)
        # Step 3: Put the object into the active collection folder so you can see it!
        ctx.collection.objects.link(obj)
        # Step 4: Make this new floor the primary active selection:
        ctx.view_layer.objects.active = obj

        # Step 5: Attach the modifier and build the tile network!
        if core_mod:
            mod = core_mod._get_or_create_gn_mod(obj, "BT_FloorGenerator")
            core_mod._build_floor_gn(mod, p.gen_fl_w, p.gen_fl_d, p.gen_fl_tw, p.gen_fl_td,
                            p.gen_fl_grout, p.gen_fl_thick, p.gen_fl_vary_z, p.gen_fl_pattern)
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_FloorGenerator)

def unregister():
    bpy.utils.unregister_class(BT_OT_FloorGenerator)