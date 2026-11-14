# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Arrange In Grid
# Section: align
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.arrange_in_grid' does not match convention 'bt.align_arrange_in_grid'

import bpy
from bpy.props import IntProperty, FloatProperty
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 2. ARRANGE IN GRID (RANKS AND FILES)
# -----------------------------------------------------------------------------
# What it does: Arranges objects into neat rows and columns on the floor plane.
class BT_OT_ArrangeInGrid(Operator):
    bl_idname  = "bt.arrange_in_grid"
    bl_label   = "Arrange in Grid"
    bl_description = "Arrange selected objects in an N×M grid on the XY plane"
    bl_options = {'REGISTER', 'UNDO'}

    # Configurable settings:
    columns: IntProperty(name="Columns", default=4, min=1)
    spacing: FloatProperty(name="Spacing", default=2.0, unit='LENGTH')

    def execute(self, ctx):
        objs = list(ctx.selected_objects)
        if not objs:
            return {'CANCELLED'}

        # NUANCE: Modulo (%) and Floor Division (//)!
        # This is standard arithmetic for converting a flat list into a 2D grid:
        #   - 'i % columns' gives the remainder -> determines Column (X)
        #   - 'i // columns' divides and chops off decimals -> determines Row (Y)
        # Example for 4 columns:
        #   Index 0 -> Col 0, Row 0
        #   Index 1 -> Col 1, Row 0
        #   Index 4 -> Col 0, Row 1 (wraps to the next line!)
        for i, obj in enumerate(objs):
            col = i % self.columns
            row = i // self.columns
            obj.location.x = col * self.spacing
            obj.location.y = -row * self.spacing  # Negative Y arranges downwards

        return {'FINISHED'}

    # NUANCE: What is 'invoke'?
    # Usually clicking a button executes immediately.
    # 'invoke' intercepts the click and pops up a small dialogue box first,
    # letting the user choose column count and spacing before running!
    def invoke(self, ctx, event):
        return ctx.window_manager.invoke_props_dialog(self)

def register():
    bpy.utils.register_class(BT_OT_ArrangeInGrid)

def unregister():
    bpy.utils.unregister_class(BT_OT_ArrangeInGrid)