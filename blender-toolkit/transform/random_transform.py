# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Random Transform
# Section: transform
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.random_transform' does not match convention 'bt.transform_random_transform'

import bpy
import random
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 1. RANDOM TRANSFORM (THE JITTER TOOL)
# -----------------------------------------------------------------------------
class BT_OT_RandomTransform(Operator):
    # 'bl_idname' is the secret computer code name for this tool.
    # Other scripts can run this button by calling bpy.ops.bt.random_transform()
    bl_idname = "bt.random_transform"
    bl_label  = "Randomise"
    bl_description = "Apply random position / rotation / scale to selected objects"

    # NUANCE: The Undo System!
    # 'REGISTER' lets Blender track this tool.
    # 'UNDO' makes Blender take a snapshot before running. If the user doesn't
    # like how the objects scattered, pressing Ctrl+Z reverts everything!
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # 'ctx' (context) tells us what the user is currently doing and selecting.
        # 'p' points directly to the settings dashboard we created in BTProps!
        p = ctx.scene.blender_toolkit

        # Generate a random integer between 0 and 99,999 to start the dice roll:
        seed = random.randint(0, 99999)
        p.rt_seed = seed
        random.seed(seed)

        # NUANCE: The 'for' loop!
        # This goes through every single selected object in your 3D scene, one at a time.
        # 'obj' becomes each item on your list during the loop:
        for obj in ctx.selected_objects:

            # CHECK 1: POSITION (X, Y, Z)
            # If the position checkbox is ticked (True), add a random nudge:
            if p.rt_do_pos:
                # 'random.uniform(min, max)' picks any random decimal number between min and max.
                # '+=' means: take the current position and add the new number to it!
                obj.location.x += random.uniform(p.rt_pos_x_min, p.rt_pos_x_max)
                obj.location.y += random.uniform(p.rt_pos_y_min, p.rt_pos_y_max)
                obj.location.z += random.uniform(p.rt_pos_z_min, p.rt_pos_z_max)

            # CHECK 2: ROTATION (EULER ANGLES)
            # If rotation is ticked, spin the object around its axes:
            if p.rt_do_rot:
                obj.rotation_euler.x += random.uniform(p.rt_rot_x_min, p.rt_rot_x_max)
                obj.rotation_euler.y += random.uniform(p.rt_rot_y_min, p.rt_rot_y_max)
                obj.rotation_euler.z += random.uniform(p.rt_rot_z_min, p.rt_rot_z_max)

            # CHECK 3: SCALE (SIZE)
            if p.rt_do_scl:
                if p.rt_scl_uniform:
                    # Uniform Scale: Multiply X, Y, and Z by the exact same multiplier
                    # so the model scales evenly without squashing or stretching!
                    s = random.uniform(p.rt_scl_x_min, p.rt_scl_x_max)
                    obj.scale *= s
                else:
                    # Non-uniform: Jitter each axis independently (can stretch/squash):
                    obj.scale.x *= random.uniform(p.rt_scl_x_min, p.rt_scl_x_max)
                    obj.scale.y *= random.uniform(p.rt_scl_y_min, p.rt_scl_y_max)
                    obj.scale.z *= random.uniform(p.rt_scl_z_min, p.rt_scl_z_max)

        # Every operator must return {'FINISHED'} to tell Blender the job was successful!
        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_RandomTransform)

def unregister():
    bpy.utils.unregister_class(BT_OT_RandomTransform)