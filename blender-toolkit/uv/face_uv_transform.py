# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Face UV Transform
# Section: uv
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.face_uv_transform' does not match convention 'bt.uv_face_uv_transform'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 5. FACE UV TRANSFORM (WORKSPACE AUTO-ARRANGER)
# -----------------------------------------------------------------------------
# What it does: A quality-of-life UI tool! Instead of manually splitting windows
# to open the UV editor, this button splits your screen in half, turns the new
# window into a UV Editor, enters Edit Mode, and highlights all UVs ready to tweak!
class BT_OT_FaceUVTransform(Operator):
    """Open the UV editor and select all UVs to facilitate UV editing (V4)."""
    bl_idname = "bt.face_uv_transform"
    bl_label  = "Face UV Transform"
    bl_description = "Open UV Editor area and select all UVs for the active object"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        obj = ctx.active_object
        if not obj or obj.type != 'MESH':
            self.report({'WARNING'}, "Active object must be a mesh")
            return {'CANCELLED'}

        # Switch to Edit Mode if currently in Object Mode:
        if obj.mode != 'EDIT':
            bpy.ops.object.mode_set(mode='EDIT')

        # Check if an Image/UV Editor window is already open somewhere on screen:
        uv_area = None
        for area in ctx.screen.areas:
            if area.type == 'IMAGE_EDITOR':
                uv_area = area
                break

        # NUANCE: Screen Splitting via Python!
        # If no UV Editor is open, we can split Blender's screen down the middle!
        if uv_area is None:
            # Find whichever window area currently has the largest width x height:
            largest = max(ctx.screen.areas, key=lambda a: a.width * a.height)
            
            # NUANCE: Context Override!
            # Blender normally requires your mouse cursor to be hovering over an area to split it.
            # By passing an 'override' dictionary, we tell Blender: "Pretend my mouse is in this window!"
            override = ctx.copy()
            override['area'] = largest
            bpy.ops.screen.area_split(override, direction='VERTICAL', factor=0.5)

            # Find the newly created area and change its type to 'IMAGE_EDITOR':
            for area in ctx.screen.areas:
                if area.type != 'VIEW_3D' and area.type != 'IMAGE_EDITOR':
                    area.type = 'IMAGE_EDITOR'
                    uv_area = area
                    break

        # Select all 3D mesh faces:
        bpy.ops.mesh.select_all(action='SELECT')
        
        # NUANCE: try/except guard
        # Try to select all 2D UV points. If the UV editor hasn't loaded fully,
        # pass safely without crashing!
        try:
            bpy.ops.uv.select_all(action='SELECT')
        except Exception:
            pass

        self.report({'INFO'}, "UV Editor opened — UVs selected")
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_FaceUVTransform)

def unregister():
    bpy.utils.unregister_class(BT_OT_FaceUVTransform)