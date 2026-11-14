# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Poly Count
# Section: scene
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.poly_count' does not match convention 'bt.scene_poly_count'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 3. POLY COUNT (THE POLYGON SCOREBOARD)
# -----------------------------------------------------------------------------
# What it does: Counts every single polygon (face) in your entire project,
# as well as the polygons on just the objects you have currently highlighted!
# Game developers use this to make sure their scenes aren't too heavy for consoles.
class BT_OT_PolyCount(Operator):
    bl_idname = "bt.poly_count"
    bl_label  = "Count Polygons"
    bl_options = {'REGISTER'}  # No UNDO needed because this only inspects numbers, it doesn't change anything!

    def execute(self, ctx):
        # NUANCE: Generator Expressions with sum()
        # 'sum(len(o.data.polygons) ...)' counts the polygons on each mesh and adds them all up.
        total = sum(len(o.data.polygons) for o in ctx.scene.objects if o.type == 'MESH')
        sel   = sum(len(o.data.polygons) for o in ctx.selected_objects if o.type == 'MESH')

        # NUANCE: Number Formatting with Commas (:,)
        # In Python, writing '{total:,}' turns giant numbers like 1450200 into friendly
        # readable text with commas: "1,450,200"!
        ctx.scene.blender_toolkit.poly_count_result = f"Sel: {sel:,}  |  Total: {total:,}"
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_PolyCount)

def unregister():
    bpy.utils.unregister_class(BT_OT_PolyCount)