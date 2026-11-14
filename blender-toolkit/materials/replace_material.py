# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Replace Material
# Section: materials
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.replace_material' does not match convention 'bt.materials_replace_material'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 3. REPLACE MATERIAL (GLOBAL SCENE REPLACEMENT)
# -----------------------------------------------------------------------------
# What it does: Searches through every object in the entire Blender scene.
# Wherever it spots Material A (e.g. "Red_Brick"), it swaps it for Material B (e.g. "Concrete")!
class BT_OT_ReplaceMaterial(Operator):
    bl_idname = "bt.replace_material"
    bl_label  = "Replace Material"
    bl_description = "Replace all scene uses of one material with another"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        p = ctx.scene.blender_toolkit

        # Look up both materials by the names typed into the text boxes:
        src = bpy.data.materials.get(p.rep_mat_find)
        dst = bpy.data.materials.get(p.rep_mat_replace)

        # Safety checks: Make sure both names are real and valid!
        if not src:
            self.report({'WARNING'}, f"Material '{p.rep_mat_find}' not found")
            return {'CANCELLED'}
        if not dst:
            self.report({'WARNING'}, f"Material '{p.rep_mat_replace}' not found")
            return {'CANCELLED'}

        count = 0
        # Loop through every object and check all its material slots:
        for obj in ctx.scene.objects:
            for slot in obj.material_slots:
                if slot.material == src:
                    slot.material = dst  # Swap the paint!
                    count += 1

        self.report({'INFO'}, f"Replaced {count} slots")
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_ReplaceMaterial)

def unregister():
    bpy.utils.unregister_class(BT_OT_ReplaceMaterial)