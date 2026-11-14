# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Template Tool
# Section: template
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.section_toolname' should match convention

import bpy
from bpy.types import Operator

class BT_OT_TemplateTool(Operator):
    bl_idname = "bt.template_toolname"
    bl_label  = "Template Tool"
    bl_description = "Description of the tool"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_TemplateTool)

def unregister():
    bpy.utils.unregister_class(BT_OT_TemplateTool)