# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Toggle Light Visibility
# Section: lighting
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.toggle_light_visibility' does not match convention 'bt.lighting_toggle_light_visibility'

import bpy
from bpy.props import BoolProperty
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 7. TOGGLE LIGHT VISIBILITY (THE MASTER LIGHT SWITCH)
# -----------------------------------------------------------------------------
# What it does: Like flipping the main fuse box in a house!
# With one click, it hides or shows every single light in the whole world,
# both in the working viewport and in the final picture render.
class BT_OT_ToggleLightVisibility(Operator):
    bl_idname = "bt.toggle_light_visibility"
    bl_label  = "Toggle Light Visibility"
    bl_description = "Show or hide all lights in the scene"
    bl_options = {'REGISTER', 'UNDO'}

    visible: BoolProperty(default=True)  # True = Turn ON, False = Turn OFF

    def execute(self, ctx):
        for obj in ctx.scene.objects:
            if obj.type == 'LIGHT':
                # NUANCE: Inverted Logic ('not self.visible')
                # In Blender, 'hide = True' means the lamp is INVISIBLE.
                # So if visible is True, 'hide' must become False!
                obj.hide_viewport = not self.visible  # Screen view
                obj.hide_render   = not self.visible  # Camera picture render

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_ToggleLightVisibility)

def unregister():
    bpy.utils.unregister_class(BT_OT_ToggleLightVisibility)