# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Random Material
# Section: materials
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.random_material' does not match convention 'bt.materials_random_material'

import bpy
import random
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 5. RANDOM MATERIAL (SURPRISE BAG OF TEXTURES)
# -----------------------------------------------------------------------------
# What it does: Grabs all materials loaded in the file and randomly assigns one
# to each selected object! Great for quickly testing scene variation.
class BT_OT_RandomMaterial(Operator):
    bl_idname = "bt.random_material"
    bl_label  = "Random Material"
    bl_description = "Assign a random material from the scene to each selected object"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # NUANCE: Filtering hidden internal materials!
        # In Blender, materials whose names start with a dot '.' are private system materials.
        # This list comprehension filters them out so we only use user materials:
        mats = [m for m in bpy.data.materials if not m.name.startswith('.')]

        # Guard check: If there are no materials in the scene, warn the user and stop:
        if not mats:
            self.report({'WARNING'}, "No materials in scene")
            return {'CANCELLED'}

        random.seed(random.randint(0, 99999))
        for obj in ctx.selected_objects:
            # If the object has no material slot, create an empty slot first:
            if not obj.material_slots:
                obj.data.materials.append(None)
            # 'random.choice(mats)' picks one random item out of the list:
            obj.material_slots[0].material = random.choice(mats)
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_RandomMaterial)

def unregister():
    bpy.utils.unregister_class(BT_OT_RandomMaterial)