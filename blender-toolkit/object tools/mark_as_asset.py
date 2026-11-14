# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Mark As Asset
# Section: object_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.mark_as_asset' does not match convention 'bt.object_tools_mark_as_asset'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 13. MARK AS ASSET (ADD TO ASSET BROWSER)
# -----------------------------------------------------------------------------
# What it does: Marks models as official Blender assets with one click so they
# appear in your Asset Browser library for easy drag-and-drop into future projects!
class BT_OT_MarkAsAsset(Operator):
    """Mark selected objects as Blender assets (V4)."""
    bl_idname = "bt.mark_as_asset"
    bl_label  = "Add to Asset Library"
    bl_description = "Mark selected objects as Blender assets so they appear in the Asset Browser"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        count = 0
        for obj in ctx.selected_objects:
            # 'obj.asset_mark()' flags the item as an asset in Blender's database:
            obj.asset_mark()
            count += 1
        self.report({'INFO'}, f"Marked {count} object(s) as assets")
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_MarkAsAsset)

def unregister():
    bpy.utils.unregister_class(BT_OT_MarkAsAsset)