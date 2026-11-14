# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Apply All Transforms
# Section: transform
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.apply_all_transforms' does not match convention 'bt.transform_apply_all_transforms'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 5. APPLY ALL TRANSFORMS (FREEZE TRANSFORMS)
# -----------------------------------------------------------------------------
class BT_OT_ApplyAllTransforms(Operator):
    bl_idname = "bt.apply_all_transforms"
    bl_label  = "Apply All Transforms"
    bl_description = "Apply location, rotation and scale (Reset XForms equivalent)"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # NUANCE: Why "Apply Transforms"?
        # If you scale an object to 200%, Blender remembers it is stretched.
        # Applying transforms bakes that scale so Blender treats it as the new 100% normal size.
        # This prevents weird bugs in physics, bevels, and texture mapping.
        bpy.ops.object.transform_apply(location=True, rotation=True, scale=True)
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_ApplyAllTransforms)

def unregister():
    bpy.utils.unregister_class(BT_OT_ApplyAllTransforms)