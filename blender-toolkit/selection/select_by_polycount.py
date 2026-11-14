# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Select By Poly Count
# Section: selection
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.select_by_polycount' does not match convention 'bt.selection_select_by_polycount'

import bpy
from bpy.props import IntProperty
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 4. SELECT BY POLY COUNT (FILTER BY COMPLEXITY / DETAIL)
# -----------------------------------------------------------------------------
# What it does: Selects objects based on how many polygons (faces) they have.
# Great for video game creators who want to find models that are too heavy or detailed!
class BT_OT_SelectByPolyCount(Operator):
    bl_idname = "bt.select_by_polycount"
    bl_label  = "Select by Poly Count"
    bl_description = "Select objects within a polygon count range"
    bl_options = {'REGISTER', 'UNDO'}

    # User sliders for minimum and maximum face count:
    min_count: IntProperty(name="Min Polys", default=0)
    max_count: IntProperty(name="Max Polys", default=1000)

    def execute(self, ctx):
        for obj in ctx.scene.objects:
            if obj.type == 'MESH':
                # 'len(obj.data.polygons)' counts how many flat facets make up the 3D shape:
                n = len(obj.data.polygons)

                # NUANCE: Chained Comparisons in Python!
                # Instead of writing 'n >= min and n <= max', Python lets you write:
                # 'self.min_count <= n <= self.max_count' just like a real maths equation!
                if self.min_count <= n <= self.max_count:
                    obj.select_set(True)

        return {'FINISHED'}

    def invoke(self, ctx, event):
        # Opens a pop-up window so the user can adjust min/max sliders before running:
        return ctx.window_manager.invoke_props_dialog(self)

def register():
    bpy.utils.register_class(BT_OT_SelectByPolyCount)

def unregister():
    bpy.utils.unregister_class(BT_OT_SelectByPolyCount)