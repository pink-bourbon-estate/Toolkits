# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Weld Vertices
# Section: mesh_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.weld_vertices' does not match convention 'bt.mesh_tools_weld_vertices'

import bpy
import bmesh
from bpy.props import FloatProperty
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 4. WELD VERTICES (MERGE BY DISTANCE / REPAIR SEAMS)
# -----------------------------------------------------------------------------
# What it does: If a 3D model has tiny cracks where points sit right on top of
# each other without touching, this tool acts like a soldering iron! It fuses
# duplicate vertices together if they are closer than the threshold distance.
class BT_OT_WeldVertices(Operator):
    bl_idname = "bt.weld_vertices"
    bl_label  = "Weld Vertices"
    bl_description = "Merge vertices by distance on all selected objects"
    bl_options = {'REGISTER', 'UNDO'}

    # Threshold: How close do two dots need to be before they snap together?
    # 0.001 metres = 1 millimetre!
    threshold: FloatProperty(name="Threshold", default=0.001, unit='LENGTH')

    def execute(self, ctx):
        for obj in ctx.selected_objects:
            if obj.type != 'MESH':
                continue

            bm = bmesh.new()
            bm.from_mesh(obj.data)

            # NUANCE: 'remove_doubles'
            # This is Blender's classic clean-up algorithm.
            # It scans all vertices ('bm.verts') and merges any that sit closer
            # than 'self.threshold' into a single shared point.
            bmesh.ops.remove_doubles(bm, verts=bm.verts, dist=self.threshold)

            # Bake the merged result back into the model and clean up RAM:
            bm.to_mesh(obj.data)
            bm.free()

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_WeldVertices)

def unregister():
    bpy.utils.unregister_class(BT_OT_WeldVertices)