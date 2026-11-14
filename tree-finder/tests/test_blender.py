import bpy, os, runpy
bpy.ops.wm.read_factory_settings(use_empty=True)
# a ground plane at z=2 to test the drop-onto-terrain option
bpy.ops.mesh.primitive_plane_add(size=400, location=(0, 0, 2.0)); bpy.context.active_object.name = "Ground"
src = open("place_empties.py").read()
src = src.replace('"//site_trees.csv"', repr(os.path.abspath("tests/out/hard_trees.csv"))).replace('GROUND_OBJECT = ""', 'GROUND_OBJECT = "Ground"')
exec(compile(src, "place_empties.py", "exec"))
em = [o for o in bpy.data.objects if o.name.startswith("Tree_")]
import csv
rows = list(csv.DictReader(open("tests/out/hard_trees.csv")))
assert len(em) == len(rows), (len(em), len(rows))
assert all(abs(o.location.z - 2.0) < 1e-3 for o in em), "not snapped to ground"
r0 = rows[0]; o0 = bpy.data.objects["Tree_1"]
assert abs(o0.location.x - float(r0["x_m"])) < 1e-4 and abs(o0.location.y - float(r0["y_m"])) < 1e-4
assert o0.parent and o0.parent.name == "Trees_Origin"
print("BLENDER OK:", len(em), "empties, snapped to z=2.0")
