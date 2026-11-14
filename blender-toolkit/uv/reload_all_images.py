# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Reload All Images
# Section: uv
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.reload_all_images' does not match convention 'bt.uv_reload_all_images'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 6. RELOAD ALL IMAGES (REFRESH TEXTURES FROM DISK)
# -----------------------------------------------------------------------------
# What it does: Sweeps through all image textures in the file and reloads them
# from disk so your updated artwork shows up immediately without restarting Blender!
class BT_OT_ReloadAllImages(Operator):
    bl_idname = "bt.reload_all_images"
    bl_label  = "Reload All Images"
    bl_description = "Reload every image in the blend file from disk"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        count = 0
        for img in bpy.data.images:
            # NUANCE: img.source == 'FILE'
            # Some images in Blender are generated in RAM (like our UV checkerboard grid).
            # We only reload images that actually live as real files on the computer's hard drive!
            if img.source == 'FILE':
                # Try reloading; if a file was moved or deleted, skip it without crashing:
                try: 
                    img.reload()
                    count += 1
                except Exception: 
                    pass

        self.report({'INFO'}, f"Reloaded {count} images")
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_ReloadAllImages)

def unregister():
    bpy.utils.unregister_class(BT_OT_ReloadAllImages)