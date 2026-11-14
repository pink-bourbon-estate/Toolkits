# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Scale Procedural Maps
# Section: materials
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.scale_procedural_maps' does not match convention 'bt.materials_scale_procedural_maps'

import bpy
from mathutils import Vector
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 7. SCALE PROCEDURAL MAPS (ZOOM TEXTURES IN OR OUT)
# -----------------------------------------------------------------------------
# What it does: Adjusts the scale of textures across multiple materials all at
# once using Multiply, Set, or Add modes.
class BT_OT_ScaleProceduralMaps(Operator):
    bl_idname = "bt.scale_procedural_maps"
    bl_label  = "Scale Mapping Nodes"
    bl_description = "Multiply/set/add scale on all Mapping nodes in selected objects' materials"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        p = ctx.scene.blender_toolkit
        for obj in ctx.selected_objects:
            for slot in obj.material_slots:
                if not slot.material or not slot.material.node_tree:
                    continue
                for node in slot.material.node_tree.nodes:
                    if node.type == 'MAPPING':
                        scl = node.inputs['Scale'].default_value
                        f = p.map_scale_factor

                        # NUANCE: Three operational math modes!
                        # 1. MULTIPLY: Doubles or halves the texture scale.
                        # 2. SET: Hard-codes the exact scale on all 3 axes.
                        # 3. ADD: Increments the current scale by a fixed offset.
                        if p.map_scale_mode == 'MULTIPLY':
                            node.inputs['Scale'].default_value = Vector((scl[0]*f, scl[1]*f, scl[2]*f))
                        elif p.map_scale_mode == 'SET':
                            node.inputs['Scale'].default_value = Vector((f, f, f))
                        else:
                            node.inputs['Scale'].default_value = Vector((scl[0]+f, scl[1]+f, scl[2]+f))
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_ScaleProceduralMaps)

def unregister():
    bpy.utils.unregister_class(BT_OT_ScaleProceduralMaps)