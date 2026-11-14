# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Random Face Remove
# Section: mesh_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.random_face_remove' does not match convention 'bt.mesh_tools_random_face_remove'

import bpy
import bmesh
import random
from bpy.props import FloatProperty
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 1. RANDOM FACE REMOVE (THE CHEESE-GRATER / DAMAGE TOOL)
# -----------------------------------------------------------------------------
# What it does: Randomly punches holes in a 3D mesh by deleting a percentage
# of its faces! Great for making ruined buildings, torn cloth, or weathered sci-fi panels.
class BT_OT_RandomFaceRemove(Operator):
    bl_idname = "bt.random_face_remove"
    bl_label  = "Random Face Remove"
    bl_description = "Randomly delete faces from selected mesh objects"
    bl_options = {'REGISTER', 'UNDO'}

    # Slider: What percentage of faces should be vaporised? (defaults to 20%)
    pct: FloatProperty(name="Remove %", default=20.0, min=0.0, max=100.0)

    def execute(self, ctx):
        # Pick a fresh random starting number for the dice roll:
        random.seed(random.randint(0, 99999))

        for obj in ctx.selected_objects:
            if obj.type != 'MESH':
                continue

            # NUANCE: The BMESH Scratchpad Pattern!
            # 1. 'bmesh.new()' creates a blank digital workbench.
            # 2. 'bm.from_mesh(obj.data)' loads a copy of the model onto the workbench.
            bm = bmesh.new()
            bm.from_mesh(obj.data)

            # NUANCE: List Comprehension with Dice Rolls!
            # Loop through all faces ('bm.faces'). If a random roll (0 to 100) is below our
            # percentage, add that face to the 'to_del' (to delete) hit-list:
            to_del = [f for f in bm.faces if random.random() * 100 < self.pct]

            # Tell bmesh to delete only the faces, keeping surrounding vertices intact if needed:
            bmesh.ops.delete(bm, geom=to_del, context='FACES')

            # NUANCE: Saving and Freeing Memory!
            # 1. 'bm.to_mesh(obj.data)' writes the altered mesh back into Blender.
            # 2. 'bm.free()' deletes the temporary workbench to release RAM and avoid memory leaks!
            bm.to_mesh(obj.data)
            bm.free()

        return {'FINISHED'}

    def invoke(self, ctx, event):
        # Opens a dialogue box so you can adjust the percentage before running:
        return ctx.window_manager.invoke_props_dialog(self)

def register():
    bpy.utils.register_class(BT_OT_RandomFaceRemove)

def unregister():
    bpy.utils.unregister_class(BT_OT_RandomFaceRemove)