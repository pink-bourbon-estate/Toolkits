# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Merge Duplicate Materials
# Section: materials
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.merge_duplicate_materials' does not match convention 'bt.materials_merge_duplicate_materials'

import bpy
import re
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 10. MERGE DUPLICATE MATERIALS (CLEAN UP .001, .002 COPIES)
# -----------------------------------------------------------------------------
# What it does: When importing models from the internet or other files, Blender often
# creates messy duplicate materials like "Wood.001", "Wood.002", "Wood.003".
# This tool uses regular expressions to find all duplicates and merge them back
# into a single shared master material!
class BT_OT_MergeDuplicateMaterials(Operator):
    bl_idname = "bt.merge_duplicate_materials"
    bl_label  = "Merge Duplicate Materials"
    bl_description = "Find materials with the same base name and consolidate them"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        groups: dict = {}
        for mat in bpy.data.materials:
            # NUANCE: Regular Expression pattern: r'\.\d+$'
            # - '\.' looks for a literal dot (.)
            # - '\d+' looks for one or more digits (numbers)
            # - '$' means right at the end of the text string!
            # 're.sub(..., "", mat.name)' strips off ".001", leaving just the base name!
            base = re.sub(r'\.\d+$', '', mat.name)
            groups.setdefault(base, []).append(mat)

        merged = 0
        for base, mats in groups.items():
            if len(mats) < 2:
                continue  # No duplicates to clean up if there's only 1 version

            keep = mats[0]  # The master original material we want to preserve

            # NUANCE: user_remap()
            # This points every object currently using the duplicate material
            # to use 'keep' instead, ensuring no objects are left without paint!
            for dup in mats[1:]:
                dup.user_remap(keep)
                bpy.data.materials.remove(dup)  # Safely delete the duplicate from memory
                merged += 1

        self.report({'INFO'}, f"Merged {merged} duplicate materials")
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_MergeDuplicateMaterials)

def unregister():
    bpy.utils.unregister_class(BT_OT_MergeDuplicateMaterials)