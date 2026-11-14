# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Select Non Smoothed
# Section: selection
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.select_non_smoothed' does not match convention 'bt.selection_select_non_smoothed'

import bpy
import bmesh
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 5. SELECT NON-SMOOTHED FACES (INSPECTOR FOR FLAT SHADING)
# -----------------------------------------------------------------------------
# What it does: Dives into Edit Mode inside the mesh and highlights only the flat,
# angular faces that have not had "Smooth Shading" enabled.
class BT_OT_SelectNonSmoothed(Operator):
    bl_idname = "bt.select_non_smoothed"
    bl_label  = "Select Non-Smoothed Faces"
    bl_description = "Enter edit mode and select all flat-shaded faces"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        obj = ctx.active_object
        if not obj or obj.type != 'MESH':
            return {'CANCELLED'}

        # 1. Automatically switch Blender into 'EDIT' mode (where you tweak points and faces):
        bpy.ops.object.mode_set(mode='EDIT')

        # 2. Deselect everything first so we start with a clean slate:
        bpy.ops.mesh.select_all(action='DESELECT')

        # 3. Switch the selection tool to FACE mode (instead of vertex/dot mode or edge/line mode):
        bpy.ops.mesh.select_mode(type='FACE')

        # NUANCE: bmesh Edit Mesh workflow!
        # When an object is in Edit Mode, normal mesh data is locked.
        # 'bmesh.from_edit_mesh()' grabs a live, editable scratchpad copy of the mesh:
        bm = bmesh.from_edit_mesh(obj.data)

        # Loop through every face on the mesh:
        for f in bm.faces:
            # If the face has smooth shading turned OFF ('not f.smooth'), select it!
            if not f.smooth:
                f.select = True

        # NUANCE: Updating the Viewport!
        # Any changes made in 'bmesh' are invisible until you push the scratchpad back to Blender:
        bmesh.update_edit_mesh(obj.data)

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_SelectNonSmoothed)

def unregister():
    bpy.utils.unregister_class(BT_OT_SelectNonSmoothed)