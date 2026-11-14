# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Attach Splines
# Section: object_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.attach_splines' does not match convention 'bt.object_tools_attach_splines'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 2. ATTACH SPLINES (JOIN CURVES TOGETHER)
# -----------------------------------------------------------------------------
# What it does: Curves are smooth lines used for roads, wires, or train tracks.
# If you draw 3 separate curves, this joins them into a single curve object!
class BT_OT_AttachSplines(Operator):
    bl_idname = "bt.attach_splines"
    bl_label  = "Attach Splines"
    bl_description = "Join all selected curve objects into the active curve"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # Guard Check: Make sure the primary highlighted object is actually a curve:
        if not ctx.active_object or ctx.active_object.type != 'CURVE':
            self.report({'WARNING'}, "Active object must be a curve")
            return {'CANCELLED'}

        # Filter the selection to make sure we only grab curves (ignoring lamps or meshes):
        curves = [o for o in ctx.selected_objects if o.type == 'CURVE']
        if len(curves) < 2:
            return {'CANCELLED'}

        # Join all the selected curves together into the active one:
        bpy.ops.object.join()
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_AttachSplines)

def unregister():
    bpy.utils.unregister_class(BT_OT_AttachSplines)