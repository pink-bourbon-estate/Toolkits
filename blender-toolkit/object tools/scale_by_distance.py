# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Scale By Distance
# Section: object_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.scale_by_distance' does not match convention 'bt.object_tools_scale_by_distance'

import bpy
from bpy.props import FloatProperty
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 11. SCALE BY DISTANCE (PROXIMITY SHRINK / GROW)
# -----------------------------------------------------------------------------
# What it does: Objects close to the active object become large, while objects
# further away shrink (or vice versa)! Great for sci-fi or dynamic motion graphics.
class BT_OT_ScaleByDistance(Operator):
    bl_idname  = "bt.scale_by_distance"
    bl_label   = "Scale by Distance"
    bl_description = "Scale selected objects proportionally by distance from active object"
    bl_options = {'REGISTER', 'UNDO'}

    max_dist: FloatProperty(name="Max Distance", default=10.0, unit='LENGTH')
    min_scl:  FloatProperty(name="Min Scale",    default=0.1)
    max_scl:  FloatProperty(name="Max Scale",    default=1.0)

    def execute(self, ctx):
        ref = ctx.active_object
        if not ref:
            return {'CANCELLED'}

        for obj in ctx.selected_objects:
            if obj == ref:
                continue
            # Calculate distance length between the object and the reference object:
            d = (obj.location - ref.location).length
            
            # 't' is distance as a 0.0 to 1.0 fraction (clamped with min):
            t = min(d / max(self.max_dist, 0.001), 1.0)
            
            # Interpolate between max scale and min scale:
            s = self.max_scl - t * (self.max_scl - self.min_scl)
            obj.scale = (s, s, s)

        return {'FINISHED'}

    def invoke(self, ctx, event):
        return ctx.window_manager.invoke_props_dialog(self)

def register():
    bpy.utils.register_class(BT_OT_ScaleByDistance)

def unregister():
    bpy.utils.unregister_class(BT_OT_ScaleByDistance)