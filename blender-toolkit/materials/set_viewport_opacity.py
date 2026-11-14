# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Set Viewport Opacity
# Section: materials
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.set_viewport_opacity' does not match convention 'bt.materials_set_viewport_opacity'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 5. SET VIEWPORT OPACITY (GHOST / X-RAY MODE)
# -----------------------------------------------------------------------------
# What it does: Makes selected objects semi-transparent (see-through) directly in
# the 3D viewport without touching their actual render materials!
# Perfect for looking inside a car body to tweak the engine or driver's seat.
class BT_OT_SetViewportOpacity(Operator):
    bl_idname = "bt.set_viewport_opacity"
    bl_label  = "Set Viewport Opacity"
    bl_description = "Set the viewport alpha of selected objects (enables transparency overlay)"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        p = ctx.scene.blender_toolkit
        for obj in ctx.selected_objects:
            # NUANCE: RGBA Index 3 (Alpha Channel)!
            # 'obj.color' is a list of four numbers: [Red, Green, Blue, Alpha].
            # Index 0 is Red, 1 is Green, 2 is Blue, and 3 is Alpha (transparency).
            # 1.0 = fully solid; 0.5 = 50% see-through ghost; 0.0 = completely invisible!
            obj.color[3] = p.viewport_opacity

            # Tell Blender to actually draw this object using viewport transparency:
            obj.show_transparent = True

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_SetViewportOpacity)

def unregister():
    bpy.utils.unregister_class(BT_OT_SetViewportOpacity)