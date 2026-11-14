# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Random Map Phase
# Section: materials
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.random_map_phase' does not match convention 'bt.materials_random_map_phase'

import bpy
import random
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 6. RANDOM MAP PHASE (TEXTURE OFFSET JITTER)
# -----------------------------------------------------------------------------
# What it does: If you put the exact same wood texture on 20 chairs, they all show
# identical wood knots in the exact same spots, which looks fake and repetitive!
# This tool nudges the texture position randomly so every object looks unique.
class BT_OT_RandomMapPhase(Operator):
    bl_idname = "bt.random_map_phase"
    bl_label  = "Random Map Phase"
    bl_description = "Randomise the mapping offset on all Mapping nodes across selected objects"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        random.seed(random.randint(0, 99999))
        for obj in ctx.selected_objects:
            for slot in obj.material_slots:
                # Skip slots that have no material or don't use shader nodes:
                if not slot.material or not slot.material.node_tree:
                    continue
                # Dive inside the material shader graph to find 'MAPPING' nodes:
                for node in slot.material.node_tree.nodes:
                    if node.type == 'MAPPING':
                        # Randomise the X and Y texture offset coordinates between 0 and 10 metres:
                        node.inputs['Location'].default_value[0] = random.uniform(0, 10)
                        node.inputs['Location'].default_value[1] = random.uniform(0, 10)
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_RandomMapPhase)

def unregister():
    bpy.utils.unregister_class(BT_OT_RandomMapPhase)