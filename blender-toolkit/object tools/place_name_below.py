# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Place Name Below
# Section: object_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.place_name_below' does not match convention 'bt.object_tools_place_name_below'

import bpy
from mathutils import Vector
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 5. PLACE NAME BELOW OBJECT (FLOATING 3D NAME TAGS)
# -----------------------------------------------------------------------------
# What it does: Creates a floating 3D text label directly underneath each selected
# object showing its name (like a museum exhibit label)!
class BT_OT_PlaceNameBelow(Operator):
    bl_idname = "bt.place_name_below"
    bl_label  = "Place Name Below Object"
    bl_description = "Create a text object displaying each selected object's name beneath it"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        for obj in list(ctx.selected_objects):
            # Find the bottom floor level of the object:
            bb = [obj.matrix_world @ Vector(c) for c in obj.bound_box]
            min_z = min(v.z for v in bb)

            # NUANCE: Creating 3D Text via bpy.data.curves.new(..., 'FONT')
            # In Blender, text objects are internally managed as specialized curves called 'FONT'!
            font_data = bpy.data.curves.new(f"Label_{obj.name}", 'FONT')
            font_data.body = obj.name       # The actual letters/words to write
            font_data.align_x = 'CENTER'    # Centre-align the text like a headline
            font_data.size = 0.15           # Font scale size (0.15 metres tall)

            # Create the 3D text object entity:
            text_obj = bpy.data.objects.new(f"Label_{obj.name}", font_data)

            # Position the text slightly below the object's base (min_z - 0.2m):
            text_obj.location = (obj.location.x, obj.location.y, min_z - 0.2)

            # Link into the scene:
            ctx.collection.objects.link(text_obj)

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_PlaceNameBelow)

def unregister():
    bpy.utils.unregister_class(BT_OT_PlaceNameBelow)