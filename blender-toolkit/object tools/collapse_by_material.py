# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Collapse By Material
# Section: object_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.collapse_by_material' does not match convention 'bt.object_tools_collapse_by_material'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 6. COLLAPSE BY MATERIAL (COMBINE BY PAINT COLOUR)
# -----------------------------------------------------------------------------
# What it does: In video games, having 100 separate wooden chairs slows down the game.
# This tool finds every object sharing the "Wood" material and joins them into one piece!
class BT_OT_CollapseByMaterial(Operator):
    bl_idname = "bt.collapse_by_material"
    bl_label  = "Collapse by Material"
    bl_description = "Join all selected objects that share the same material"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        objs = [o for o in ctx.selected_objects if o.type == 'MESH']
        
        # NUANCE: Grouping with a Python Dictionary!
        # 'mat_groups' will look like: { Material_Wood: [chair1, chair2], Material_Metal: [pipe1, pipe2] }
        mat_groups: dict = {}
        for obj in objs:
            key = obj.material_slots[0].material if obj.material_slots else None
            # 'setdefault' creates an empty list if the material hasn't been seen yet, then appends the object:
            mat_groups.setdefault(key, []).append(obj)

        # Loop through each material group and join them together:
        for mat, group in mat_groups.items():
            if len(group) < 2:
                continue  # Nothing to join if there is only 1 object with this material

            # Clear selection, highlight only this group, and join them:
            bpy.ops.object.select_all(action='DESELECT')
            for o in group:
                o.select_set(True)
            ctx.view_layer.objects.active = group[0]
            bpy.ops.object.join()

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_CollapseByMaterial)

def unregister():
    bpy.utils.unregister_class(BT_OT_CollapseByMaterial)