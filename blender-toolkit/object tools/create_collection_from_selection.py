# ─────────────────────────────────────────────────────────────────────────────
# Blender Toolkit — Create Collection From Selection
# Section: object_tools
# ─────────────────────────────────────────────────────────────────────────────
# FLAG: bl_idname 'bt.create_collection_from_selection' does not match convention 'bt.object_tools_create_collection_from_selection'

import bpy
from bpy.types import Operator

# -----------------------------------------------------------------------------
# 3. CREATE COLLECTION FROM SELECTION (GROUP INTO A NEW FOLDER)
# -----------------------------------------------------------------------------
# What it does: In Blender, "Collections" are the folders in your Outliner tree.
# This operator grabs all selected objects, builds a brand new collection folder,
# and moves the objects into it so your scene tree stays tidy!
class BT_OT_CreateCollectionFromSelection(Operator):
    bl_idname = "bt.create_collection_from_selection"
    bl_label  = "Create Collection from Selection"
    bl_description = "Create a new collection and move selected objects into it"
    bl_options = {'REGISTER', 'UNDO'}

    def execute(self, ctx):
        p = ctx.scene.blender_toolkit

        # NUANCE: Fallback Names with 'or'!
        # Python evaluates expressions left to right.
        # If 'p.new_coll_name' is empty, it uses the active object's name.
        # If that's missing too, it falls back to the default text "BT_Collection"!
        name = p.new_coll_name or (ctx.active_object.name if ctx.active_object else "BT_Collection")

        # 1. Create the new collection container in Blender's memory:
        coll = bpy.data.collections.new(name)

        # 2. Attach the new folder into the master scene tree so it becomes visible:
        ctx.scene.collection.children.link(coll)

        # 3. Move each selected object across:
        for obj in list(ctx.selected_objects):
            # Unlink the object from its old collections so it doesn't appear in two folders at once:
            for old_coll in obj.users_collection:
                old_coll.objects.unlink(obj)

            # Link the object into our brand new collection:
            coll.objects.link(obj)

        return {'FINISHED'}

    # 'invoke' runs before 'execute' to pop open a small name-entry dialogue box:
    def invoke(self, ctx, event):
        if ctx.active_object:
            ctx.scene.blender_toolkit.new_coll_name = ctx.active_object.name
        return ctx.window_manager.invoke_props_dialog(self)

    # 'draw' tells Blender what text box to show inside the popup dialogue window:
    def draw(self, ctx):
        self.layout.prop(ctx.scene.blender_toolkit, 'new_coll_name')

def register():
    bpy.utils.register_class(BT_OT_CreateCollectionFromSelection)

def unregister():
    bpy.utils.unregister_class(BT_OT_CreateCollectionFromSelection)