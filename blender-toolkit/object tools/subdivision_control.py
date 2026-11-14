# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Subdivision Control
# Section: object_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.subdivision_control' does not match convention 'bt.object_tools_subdivision_control'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 8. SUBDIVISION CONTROL (BATCH SMOOTHNESS SETTINGS)
# -----------------------------------------------------------------------------
# What it does: Adjusts the smoothness (Subdivision Surface modifier) on dozens
# of objects at the exact same time!
class BT_OT_SubdivisionControl(Operator):
    bl_idname = "bt.subdivision_control"
    bl_label  = "Apply Subdivision"
    bl_description = "Set subdivision viewport/render levels on all selected objects"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        p = ctx.scene.blender_toolkit
        for obj in ctx.selected_objects:
            # NUANCE: What is next(...)?
            # 'next(...)' looks for the first modifier of type 'SUBSURF'.
            # If the object doesn't have one, it safely returns 'None' instead of an error!
            mod = next((m for m in obj.modifiers if m.type == 'SUBSURF'), None)
            
            # If no Subdivision modifier exists, add one:
            if not mod:
                mod = obj.modifiers.new("Subdivision", 'SUBSURF')
                
            # Apply viewport and render quality settings from our properties dashboard:
            if p.sub_do_view:   mod.levels        = p.sub_levels_view
            if p.sub_do_render: mod.render_levels = p.sub_levels_render
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_SubdivisionControl)

def unregister():
    bpy.utils.unregister_class(BT_OT_SubdivisionControl)