# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Rename Number
# Section: naming
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.rename_number' does not match convention 'bt.naming_rename_number'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 4. NUMBER OBJECTS (SEQUENTIAL NUMBERING WITH ZERO PADDING)
# -----------------------------------------------------------------------------
# What it does: Names a group of objects sequentially with tidy numbers.
# Example: "Wall_001", "Wall_002", "Wall_003"...
class BT_OT_RenameNumber(Operator):
    bl_idname = "bt.rename_number"
    bl_label  = "Number Objects"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        p = ctx.scene.blender_toolkit
        # NUANCE: enumerate()
        # 'enumerate()' counts each item as the loop runs, giving both:
        #   - 'i': The counter index (0, 1, 2, 3...)
        #   - 'obj': The actual selected 3D object
        for i, obj in enumerate(ctx.selected_objects):
            # NUANCE: str() and .zfill()
            # 1. 'i + p.nm_start' calculates the actual number (e.g. 0 + 1 = 1).
            # 2. 'str(...)' converts that number into text.
            # 3. '.zfill(padding)' pads the number with leading zeros so they sort neatly!
            #    If padding is 3: number 7 becomes "007", 42 becomes "042".
            n = str(i + p.nm_start).zfill(p.nm_pad)

            # Assemble the new name using an f-string:
            obj.name = f"{p.nm_base}_{n}"
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_RenameNumber)

def unregister():
    bpy.utils.unregister_class(BT_OT_RenameNumber)