# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Wire Color Random
# Section: wire
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.wire_color_random' does not match convention 'bt.wire_wire_color_random'

import bpy
import random
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 1. RANDOMISE WIRE COLOUR (VIEWPORT COLOUR CODING)
# -----------------------------------------------------------------------------
# What it does: Assigns random display colours to selected objects.
# It can colour-code by:
#   1. COLLECTION: All objects in the same folder share one colour.
#   2. MATERIAL: All objects with the same paint share one colour.
#   3. OBJECT: Every single object gets its own totally unique colour!
class BT_OT_WireColorRandom(Operator):
    bl_idname = "bt.wire_color_random"
    bl_label  = "Randomise Wire Colour"
    bl_description = "Assign random display colours to selected objects"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        # Dynamically import the core module to access the _rand_hsv_color function
        import sys
        if __package__:
            core_mod = sys.modules.get(__package__.split('.')[0] + ".blender_toolkit_core")
        else:
            core_mod = sys.modules.get("blender_toolkit_core")

        if not core_mod:
            self.report({'ERROR'}, "Core module not found. Cannot generate random HSV colors.")
            return {'CANCELLED'}

        p = ctx.scene.blender_toolkit
        random.seed(random.randint(0, 99999))
        mode = p.wire_mode

        # NUANCE: What is a Cache dictionary (col_cache)?
        # A 'cache' is a memory bank. If you have 20 chairs that all belong to the 
        # "Dining Room" collection, you don't want to pick 20 different colours!
        # The cache stores: { Folder_A: Bright_Green, Folder_B: Bright_Orange }.
        # That way, every object in the same group gets the exact same colour!
        col_cache: dict = {}

        # ── MODE 1: BY COLLECTION (OUTLINER FOLDER) ──────────────────────────
        if mode == 'COLLECTION':
            for obj in ctx.selected_objects:
                # Find the folder/collection this object lives in:
                coll = obj.users_collection[0] if obj.users_collection else None
                
                # NUANCE: What is 'id()'?
                # Every single item in your computer's memory has a unique number address.
                # 'id(coll)' gets that unique memory ID number to use as a dictionary key!
                key = id(coll)

                # If we haven't seen this folder yet, pick a new random colour for it:
                if key not in col_cache: 
                    col_cache[key] = core_mod._rand_hsv_color()
                
                # Grab the shared colour from our cache:
                r, g, b = col_cache[key]

                # Assign the display colour (Red, Green, Blue, Alpha) and turn on wireframe lines:
                obj.color = (r, g, b, 1.0)
                obj.show_wire = True

        # ── MODE 2: BY MATERIAL (SURFACE FINISH) ─────────────────────────────
        elif mode == 'MATERIAL':
            for obj in ctx.selected_objects:
                # Check what material is in the first slot:
                mat = obj.material_slots[0].material if obj.material_slots else None
                key = id(mat)

                # Pick one shared colour per material type:
                if key not in col_cache: 
                    col_cache[key] = core_mod._rand_hsv_color()
                
                r, g, b = col_cache[key]
                obj.color = (r, g, b, 1.0)
                obj.show_wire = True

        # ── MODE 3: INDIVIDUAL OBJECT (EVERYTHING UNIQUE) ────────────────────
        else:
            for obj in ctx.selected_objects:
                # Pick a brand new fresh random colour for every single item!
                r, g, b = core_mod._rand_hsv_color()
                obj.color = (r, g, b, 1.0)
                obj.show_wire = True

        return {'FINISHED'}

def register():
    bpy.utils.register_class(BT_OT_WireColorRandom)

def unregister():
    bpy.utils.unregister_class(BT_OT_WireColorRandom)