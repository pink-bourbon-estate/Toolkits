# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Delete All Lights
# Section: lighting
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.delete_all_lights' does not match convention 'bt.lighting_delete_all_lights'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 5. DELETE ALL LIGHTS (RESET THE LIGHTING STAGE)
# -----------------------------------------------------------------------------
# What it does: If your lighting setup looks messy and you want to start fresh,
# this sweeps through the scene, selects every light, and deletes them all!
class BT_OT_DeleteAllLights(Operator):
    bl_idname = "bt.delete_all_lights"
    bl_label  = "Delete All Lights"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # Find every entity whose type is specifically 'LIGHT' (ignoring models/cameras):
        lights = [o for o in ctx.scene.objects if o.type == 'LIGHT']
        
        # Deselect everything first so we don't accidentally delete normal 3D models:
        bpy.ops.object.select_all(action='DESELECT')
        
        # Select all lights and delete them:
        for l in lights: 
            l.select_set(True)
        bpy.ops.object.delete()
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_DeleteAllLights)

def unregister():
    bpy.utils.unregister_class(BT_OT_DeleteAllLights)