# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Place at Vertices
# Section: object_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.place_at_vertices' does not match convention 'bt.object_tools_place_at_vertices'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 10. PLACE AT VERTICES (SPAWN OBJECTS ON CORNER DOTS)
# -----------------------------------------------------------------------------
# What it does: Spawns a copy of the active object at every vertex (corner point)
# of the target mesh (e.g. putting fence posts at every corner of a garden outline)!
class BT_OT_PlaceAtVertices(Operator):
    bl_idname = "bt.place_at_vertices"
    bl_label  = "Place at Vertices"
    bl_description = "Instance the active object at each vertex of the selected mesh"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        source = ctx.active_object
        targets = [o for o in ctx.selected_objects if o != source and o.type == 'MESH']
        if not source or not targets:
            return {'CANCELLED'}

        for target in targets:
            for vert in target.data.vertices:
                # Calculate vertex position in global world space using matrix multiplication (@):
                world_pos = target.matrix_world @ vert.co
                
                # Clone the source object and place it at that vertex position:
                new_obj = source.copy()
                new_obj.data = source.data
                new_obj.location = world_pos
                
                # NUANCE: Linking to Scene Collection
                # In Blender, simply creating an object in memory isn't enough;
                # you must link it into a collection so it actually appears in the 3D world!
                ctx.collection.objects.link(new_obj)

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_PlaceAtVertices)

def unregister():
    bpy.utils.unregister_class(BT_OT_PlaceAtVertices)