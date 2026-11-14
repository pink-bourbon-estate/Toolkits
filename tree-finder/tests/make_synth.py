import numpy as np, json, sys
from PIL import Image
from scipy import ndimage as ndi
rng = np.random.default_rng(7)
W = H = 600; mpp = 0.3
img = np.zeros((H, W, 3), np.float32)
# lawn: bright mid green with noise
img[:] = (0.38, 0.55, 0.25)
img += rng.normal(0, 0.02, img.shape)
# paving/road grey band, roofs, gravel
img[250:290, :] = (0.55, 0.55, 0.55)
img[40:140, 380:520] = (0.62, 0.30, 0.25)   # red roof
img[420:520, 40:160] = (0.70, 0.70, 0.68)   # light roof
img[300:380, 300:420] = (0.60, 0.52, 0.40)  # gravel
img += rng.normal(0, 0.015, img.shape)
# trees
truth = []
yy, xx = np.mgrid[0:H, 0:W]
def tree(cx, cy, r):
    d = np.hypot(xx - cx, yy - cy)
    m = d < r * (1 + 0.12 * np.sin(np.arctan2(yy - cy, xx - cx) * 5 + cx))
    tex = ndi.gaussian_filter(rng.normal(0, 1, (H, W)), 1.5) * 0.06
    col = np.array([0.12, 0.28, 0.10])
    img[m] = col + tex[m, None] + (0.03 * (1 - d[m] / r))[:, None]
    # shadow to the SE
    sh = (np.hypot(xx - cx - r * 0.5, yy - cy - r * 0.5) < r) & ~m
    img[sh] *= 0.7
    truth.append((cx, cy, 2 * r * mpp))
pts = []
while len(pts) < 55:
    cx, cy = rng.integers(20, W - 20, 2); r = rng.uniform(6, 16)
    if 240 < cy < 300: continue
    if any(np.hypot(cx - a, cy - b) < (r + c) * 0.95 for a, b, c in pts): continue
    if 30 < cy < 150 and 370 < cx < 530: continue
    pts.append((cx, cy, r))
for cx, cy, r in pts: tree(cx, cy, r)
# a touching pair + a row of 5
for k in range(5): tree(100 + k * 20, 200, 10)
tree(450, 450, 14); tree(472, 450, 12)
img = np.clip(img, 0, 1)
Image.fromarray((img * 255).astype(np.uint8)).save("tests/synth.png")
json.dump([[float(a) for a in t] for t in truth], open("tests/truth.json", "w"))
print(len(truth), "trees")
