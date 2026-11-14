# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — UVW Randomiser
# Section: materials
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.uvw_randomiser' does not match convention 'bt.materials_uvw_randomiser'

import bpy
import random
import math
from mathutils import Vector
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 4. UVW RANDOMISER (NATURAL TEXTURE JITTER)
# -----------------------------------------------------------------------------
# What it does: If you have dozens of wooden planks or stone pavers, this tool
# dives into each material's Mapping node and rolls random offsets, angles,
# and scales so no two boards look identical!
class BT_OT_UVWRandomiser(Operator):
    bl_idname = "bt.uvw_randomiser"
    bl_label  = "UVW Randomiser"
    bl_description = "Randomise Mapping node offset, rotation and/or scale on selected objects"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        p = ctx.scene.blender_toolkit
        random.seed(random.randint(0, 99999))

        for obj in ctx.selected_objects:
            for slot in obj.material_slots:
                if not slot.material or not slot.material.node_tree:
                    continue

                for node in slot.material.node_tree.nodes:
                    # Only tweak 'MAPPING' nodes (which control position, rotation, and scale of textures):
                    if node.type != 'MAPPING':
                        continue

                    # 1. Randomise Location Offset (slide texture along X and Y between 0 and 10 metres):
                    if p.uvw_rand_offset:
                        node.inputs['Location'].default_value[0] = random.uniform(0, 10)
                        node.inputs['Location'].default_value[1] = random.uniform(0, 10)

                    # 2. Randomise Rotation (spin texture up to a full 360° circle: 2 * Pi radians):
                    if p.uvw_rand_rotation:
                        node.inputs['Rotation'].default_value[2] = random.uniform(0, math.pi * 2)

                    # 3. Randomise Scale (resize texture between min and max bounds):
                    if p.uvw_rand_scale:
                        s = random.uniform(p.uvw_rand_scale_min, p.uvw_rand_scale_max)
                        node.inputs['Scale'].default_value = Vector((s, s, s))

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_UVWRandomiser)

def unregister():
    bpy.utils.unregister_class(BT_OT_UVWRandomiser)