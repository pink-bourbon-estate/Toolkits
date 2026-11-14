"""Run inside Blender (Scripting tab > Open > Run Script).
Reads the *_trees.csv from find_trees.py and puts one Empty at every tree position.

Set the three options below. Units are metres (Blender default). Image centre = world origin;
+X east, +Y north. If your site model uses a different origin, move/rotate the parent empty
'Trees_Origin' that every tree is parented to - no need to redo anything.
"""
import csv
import bpy
from mathutils import Vector

CSV_PATH = "//site_trees.csv"        # // = relative to the .blend, or use a full path like "trees/site_trees.csv"
GROUND_OBJECT = ""                    # optional: name of your terrain/ground mesh. Trees drop onto it (Z). Empty = Z 0
EMPTY_DISPLAY = "SINGLE_ARROW"        # PLAIN_AXES, SINGLE_ARROW, CIRCLE, SPHERE ...
SCALE_BY_CROWN = False                # True = empty size follows crown diameter

path = bpy.path.abspath(CSV_PATH)
coll = bpy.data.collections.get("Trees") or bpy.data.collections.new("Trees")
if coll.name not in bpy.context.scene.collection.children:
    bpy.context.scene.collection.children.link(coll)

origin = bpy.data.objects.get("Trees_Origin")
if origin is None:
    origin = bpy.data.objects.new("Trees_Origin", None)
    origin.empty_display_type = "ARROWS"
    coll.objects.link(origin)

ground = bpy.data.objects.get(GROUND_OBJECT) if GROUND_OBJECT else None
dg = bpy.context.evaluated_depsgraph_get()

n = 0
with open(path, newline="") as f:
    for row in csv.DictReader(f):
        x, y = float(row["x_m"]), float(row["y_m"])
        z = 0.0
        if ground is not None:
            inv = ground.matrix_world.inverted()
            o = inv @ Vector((x, y, 1000.0))
            d = (inv.to_3x3() @ Vector((0, 0, -1))).normalized()
            hit, loc, nor, idx = ground.ray_cast(o, d, distance=5000.0, depsgraph=dg)
            if hit:
                z = (ground.matrix_world @ loc).z
        e = bpy.data.objects.new("Tree_%s" % row["id"], None)
        e.empty_display_type = EMPTY_DISPLAY
        e.location = (x, y, z)
        e.parent = origin
        d_m = float(row["crown_diam_m"])
        e["crown_diam_m"] = d_m
        e["flag"] = row.get("flag", "")
        e.empty_display_size = d_m * 0.5 if SCALE_BY_CROWN else 1.0
        coll.objects.link(e)
        n += 1
print("Placed %d tree empties from %s" % (n, path))
