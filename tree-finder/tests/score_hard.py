import csv, json, sys, numpy as np
truth = json.load(open("tests/truth_hard.json"))
det = [(float(r["x_px"]), float(r["y_px"])) for r in csv.DictReader(open("tests/out/hard_trees.csv"))]
used = set(); hit = 0
for tx, ty, d in truth:
    r = d / 0.3 / 2
    best = None
    for i, (x, y) in enumerate(det):
        if i in used: continue
        dd = np.hypot(x - tx, y - ty)
        if dd < r * 0.8 and (best is None or dd < best[0]): best = (dd, i)
    if best: used.add(best[1]); hit += 1
print("truth %d  detected %d  matched %d  missed %d  false %d" % (len(truth), len(det), hit, len(truth) - hit, len(det) - len(used)))
