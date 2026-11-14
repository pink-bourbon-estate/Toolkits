"""Run inside Blender (Scripting tab > Open > Run Script).  UNTESTED in Blender - check the first run.
Companion to place_empties.py for find_trees_v0.12.py's hedge / shrub product.

  hedge_paths.csv    -> one 3D poly curve per hedge (bevelled so it has width and height)
  hedges_shrubs.csv  -> one Empty per shrub (custom props: width_m, height_m, area_m2)

Same conventions as place_empties.py: metres, +X east, +Y north, parented to 'Trees_Origin'.
"""
import csv
import bpy
from mathutils import Vector

HEDGE_PATHS_CSV = "//os_TQ2981_hedge_paths.csv"
SHRUBS_CSV = "//os_TQ2981_hedges_shrubs.csv"
GROUND_OBJECT = ""          # optional terrain mesh name; points drop onto it. Empty = Z 0
SHRUB_DISPLAY = "SPHERE"
HEDGE_BEVEL = True          # give curves a rectangular-ish cross-section from width_m / height_m

coll = bpy.data.collections.get("Hedges") or bpy.data.collections.new("Hedges")
if coll.name not in bpy.context.scene.collection.children:
    bpy.context.scene.collection.children.link(coll)
origin = bpy.data.objects.get("Trees_Origin")
if origin is None:
    origin = bpy.data.objects.new("Trees_Origin", None)
    origin.empty_display_type = "ARROWS"
    coll.objects.link(origin)

ground = bpy.data.objects.get(GROUND_OBJECT) if GROUND_OBJECT else None
dg = bpy.context.evaluated_depsgraph_get()


def ground_z(x, y):
    if ground is None:
        return 0.0
    inv = ground.matrix_world.inverted()
    o = inv @ Vector((x, y, 1000.0))
    d = (inv.to_3x3() @ Vector((0, 0, -1))).normalized()
    hit, loc, nor, idx = ground.ray_cast(o, d, distance=5000.0, depsgraph=dg)
    return (ground.matrix_world @ loc).z if hit else 0.0


# ---- hedges -------------------------------------------------------------
lines = {}
with open(bpy.path.abspath(HEDGE_PATHS_CSV), newline="") as f:
    for r in csv.DictReader(f):
        lines.setdefault(int(r["hedge_id"]), []).append(r)

nh = 0
for hid, pts in sorted(lines.items()):
    pts.sort(key=lambda r: int(r["seq"]))
    width = float(pts[0]["width_m"])
    height = float(pts[0]["height_m"])
    cu = bpy.data.curves.new("Hedge_%d" % hid, "CURVE")
    cu.dimensions = "3D"
    sp = cu.splines.new("POLY")
    sp.points.add(len(pts) - 1)
    for p, r in zip(sp.points, pts):
        x, y = float(r["x_m"]), float(r["y_m"])
        p.co = (x, y, ground_z(x, y) + height * 0.5, 1.0)
    if HEDGE_BEVEL:
        cu.bevel_depth = max(width, 0.3) * 0.5
        cu.extrude = max(height - width, 0.0) * 0.5
        cu.bevel_resolution = 1
    ob = bpy.data.objects.new("Hedge_%d" % hid, cu)
    ob["width_m"], ob["height_m"] = width, height
    ob.parent = origin
    coll.objects.link(ob)
    nh += 1

# ---- shrubs -------------------------------------------------------------
ns = 0
with open(bpy.path.abspath(SHRUBS_CSV), newline="") as f:
    for r in csv.DictReader(f):
        if r["type"] != "shrub":
            continue
        x, y = float(r["x_m"]), float(r["y_m"])
        e = bpy.data.objects.new("Shrub_%s" % r["id"], None)
        e.empty_display_type = SHRUB_DISPLAY
        e.location = (x, y, ground_z(x, y))
        e.empty_display_size = max(float(r["width_m"]), 0.5) * 0.5
        e["width_m"] = float(r["width_m"])
        e["height_m"] = float(r["height_m"])
        e["area_m2"] = float(r["area_m2"])
        e.parent = origin
        coll.objects.link(e)
        ns += 1
print("Placed %d hedge curves and %d shrub empties" % (nh, ns))
