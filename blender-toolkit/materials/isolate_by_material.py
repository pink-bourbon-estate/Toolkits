# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Isolate By Material
# Section: materials
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.isolate_by_material' does not match convention 'bt.materials_isolate_by_material'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 8. ISOLATE BY MATERIAL (SOLO A SPECIFIC FINISH)
# -----------------------------------------------------------------------------
# What it does: Hides everything in the scene EXCEPT objects using the active
# object's material. Great for isolating all glass windows or checking all brick walls!
class BT_OT_IsolateByMaterial(Operator):
    bl_idname = "bt.isolate_by_material"
    bl_label  = "Isolate by Material"
    bl_description = "Hide all objects that do NOT share the active material"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        active_mat = ctx.object.active_material if ctx.object else None
        if not active_mat:
            return {'CANCELLED'}

        for obj in ctx.scene.objects:
            # Check if this object uses the active material:
            has = any(s.material == active_mat for s in obj.material_slots)
            # NUANCE: Inverted Boolean!
            # 'hide_viewport = True' hides the object, while False keeps it visible.
            # 'not has' means: if it DOESN'T have the material, hide it!
            obj.hide_viewport = not has
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_IsolateByMaterial)

def unregister():
    bpy.utils.unregister_class(BT_OT_IsolateByMaterial)