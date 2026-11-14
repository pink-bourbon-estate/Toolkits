# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Modifier Utility
# Section: object_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.modifier_utility' does not match convention 'bt.object_tools_modifier_utility'

import bpy
from bpy.props import EnumProperty
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 9. MODIFIER UTILITY (BATCH ADD MODIFIERS)
# -----------------------------------------------------------------------------
# What it does: Adds a chosen modifier (like Bevel, Weld, or Solidify) to every
# selected object in one click, without adding accidental duplicates!
class BT_OT_ModifierUtility(Operator):
    bl_idname = "bt.modifier_utility"
    bl_label  = "Add Modifier to All"
    bl_description = "Add a modifier to all selected objects"
    bl_options = {'REGISTER', 'UNDO'}

    mod_type: EnumProperty(name="Modifier", items=[
        ('SUBSURF','Subdivision',''), ('BEVEL','Bevel',''), ('SOLIDIFY','Solidify',''),
        ('WEIGHTED_NORMAL','Weighted Normal',''), ('TRIANGULATE','Triangulate',''),
        ('DECIMATE','Decimate',''), ('WELD','Weld',''),
    ], default='WEIGHTED_NORMAL')

    def execute(self, ctx):
        for obj in ctx.selected_objects:
            if obj.type != 'MESH':
                continue
            # Check if this modifier type is already present so we don't double up:
            if not any(m.type == self.mod_type for m in obj.modifiers):
                # '.title()' converts 'BEVEL' into friendly 'Bevel' for the name:
                obj.modifiers.new(self.mod_type.title(), self.mod_type)
        return {'FINISHED'}

    def invoke(self, ctx, event):
        return ctx.window_manager.invoke_props_dialog(self)

def register():
    bpy.utils.register_class(BT_OT_ModifierUtility)

def unregister():
    bpy.utils.unregister_class(BT_OT_ModifierUtility)