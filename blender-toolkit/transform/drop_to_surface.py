# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Drop to Surface
# Section: transform
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.drop_to_surface' does not match convention 'bt.transform_drop_to_surface'

import bpy
from mathutils import Vector
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 4. DROP TO SURFACE (VIRTUAL GRAVITY / LASER SCANNING)
# -----------------------------------------------------------------------------
class BT_OT_DropToSurface(Operator):
    bl_idname  = "bt.drop_to_surface"
    bl_label   = "Drop to Surface"
    bl_description = "Raycast selected objects downward onto the active object surface"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # The 'active_object' is the primary object highlighted in bright orange:
        target = ctx.active_object
        if not target:
            # If the user forgot to select a ground surface, show a yellow warning banner and stop:
            self.report({'WARNING'}, "Set an active (target) object first")
            return {'CANCELLED'}

        # 'depsgraph' is the dependency graph (Blender's evaluated view of scene physics/modifiers):
        depsgraph = ctx.evaluated_depsgraph_get()

        for obj in ctx.selected_objects:
            # Don't drop the floor onto itself!
            if obj == target:
                continue

            # NUANCE: Ray-casting (Firing a virtual laser beam!)
            # 1. Start 1000 units directly above the object:
            origin = obj.matrix_world.translation.copy()
            origin.z += 1000.0

            # 2. Fire a laser beam straight down (Vector 0, 0, -1) to hit the target:
            # 'matrix_world.inverted()' transforms the laser into target-local space:
            hit, loc, nor, _ = target.ray_cast(
                target.matrix_world.inverted() @ origin,
                target.matrix_world.inverted().to_3x3() @ Vector((0,0,-1)),
                depsgraph=depsgraph
            )

            # 3. If the laser hits the surface, move the object right onto the landing point!
            if hit:
                world_loc = target.matrix_world @ loc
                min_z = min((obj.matrix_world @ Vector(c)).z for c in obj.bound_box)
                # Calculate the exact difference between the landing spot and object base:
                obj.location.z += world_loc.z - min_z

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_DropToSurface)

def unregister():
    bpy.utils.unregister_class(BT_OT_DropToSurface)