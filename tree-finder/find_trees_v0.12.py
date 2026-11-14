#!/usr/bin/env python3
"""
find_trees_v0.12.py  -  LiDAR + satellite tree crown detection for a 1 km OS tile
=================================================================================

Pipeline
  1. LiDAR   Fetch Environment Agency WCS rasters for the tile (+ buffer), 1 m:
               * VOM      Vegetation Object Model  (canopy height, vegetation only, >2.5 m)
               * DSM, DTM first-return surface model and bare-earth model (used to
                          rescue trees the VOM classified out, e.g. garden trees)
  2. Imagery Fetch Esri World Imagery tiles and RE-PROJECT them onto the OS grid
             (v0.08/0.09 cut a true-north box; the OS grid is rotated slightly from true
             north (the angle depends on location), which skews a 1 km tile by tens of metres).  The image is then
             registered to the LiDAR by cross-correlation, which removes the datum
             shift / ortho-rectification offset.
  3. Fusion  LiDAR gives geometry (height, crown extent); imagery gives classification:
               * trees VOM already found are kept
               * extra objects (DSM-DTM >= 2.5 m, not in VOM) are accepted ONLY if they
                 are green in the imagery and have a rough, non-planar surface
                 (rejects roofs, cars, green-painted flat surfaces)
  4. Crowns  Treetops = h-maxima on a log-height surface (prominence is RELATIVE to tree
             height, so tall woodland trees are not over-split); crowns = watershed;
             tiny fragments merged into neighbours; any crown wider than --max-crown
             is split (finer watershed first, k-means as a last resort).

  5. Hedges  Green vegetation 1-6 m tall that is NOT a tree crown is split by shape:
             long + narrow -> hedge (polyline through its skeleton), small + compact -> shrub
             (point). Large soft patches (reed beds, scrub, meadow) are ignored. This is a
             separate product: trees stay trees.

Outputs (in --out):
  os_<TILE>_lidar_trees.csv            tree point calculation (OS coordinates, crown, height)
  os_<TILE>_satellite.tif              satellite imagery on the OS grid (georeferenced, EPSG:27700)
  os_<TILE>_trees_on_satellite.jpg     trees overlaid on that imagery
  os_<TILE>_trees_on_satellite.tif     same overlay, georeferenced (only with --overlay-tif)
  os_<TILE>_hedges_shrubs.csv          one row per hedge / shrub (length, width, height ...)
  os_<TILE>_hedge_paths.csv            hedge polyline vertices (for Blender curves)
  os_<TILE>_hedges_on_satellite.jpg    trees (thin yellow) + hedges (orange) + shrubs (magenta)
  _cache/                              downloaded rasters, so re-runs while tuning are instant
"""
import argparse
import csv
import math
import os
import re
import sys
import warnings
from collections import deque
from concurrent.futures import ThreadPoolExecutor
from io import BytesIO

warnings.filterwarnings("ignore")

import numpy as np
import requests
from PIL import Image, ImageDraw
from scipy import ndimage as ndi
from scipy.cluster.vq import kmeans2
from skimage import measure, morphology, segmentation

# --------------------------------------------------------------------------
# Constants
# --------------------------------------------------------------------------
BASE = "https://environment.data.gov.uk/spatialdata"
SOURCES = {   # coverage IDs verified against each service's GetCapabilities
    "vom": (f"{BASE}/vegetation-object-model/wcs",
            "ecae3bef-1e1d-4051-887b-9dc613c928ec__Vegetation_Object_Model_Elevation_2022"),
    "dsm": (f"{BASE}/lidar-composite-digital-surface-model-first-return-dsm-1m/wcs",
            "df4e3ec3-315e-48aa-aaaf-b5ae74d7b2bb__Lidar_Composite_Elevation_FZ_DSM_1m"),
    "dtm": (f"{BASE}/lidar-composite-digital-terrain-model-dtm-1m/wcs",
            "13787b9a-26a4-4775-8523-806d13af58fc__Lidar_Composite_Elevation_DTM_1m"),
}
ESRI_URL = "https://server.arcgisonline.com/ArcGIS/rest/services/World_Imagery/MapServer/tile/{z}/{y}/{x}"
HEADERS = {"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) tree-mapper/0.12"}
BNG = "EPSG:27700"
MERC = "EPSG:3857"
ORIGIN = 20037508.342789244          # half the Web-Mercator world width, metres


# --------------------------------------------------------------------------
# OS grid helpers
# --------------------------------------------------------------------------
def gridref_to_en(ref):
    """'TQ2981' or 'TQ 29500 81500' -> (easting, northing) in metres (SW corner)."""
    ref = re.sub(r"[^A-Za-z0-9]", "", ref).upper()
    m = re.fullmatch(r"([A-HJ-Z])([A-HJ-Z])(\d*)", ref)
    if not m or len(m.group(3)) % 2:
        raise ValueError(f"Not a valid OS grid reference: {ref!r}")
    l1, l2 = ord(m.group(1)) - 65, ord(m.group(2)) - 65
    l1 -= l1 > 7
    l2 -= l2 > 7
    e100 = ((l1 - 2) % 5) * 5 + (l2 % 5)
    n100 = (19 - l1 // 5 * 5) - (l2 // 5)
    digits = m.group(3)
    h = len(digits) // 2
    e = int(digits[:h]) * 10 ** (5 - h) if h else 0
    n = int(digits[h:]) * 10 ** (5 - h) if h else 0
    return e100 * 100000 + e, n100 * 100000 + n


def en_to_tilename(e, n):
    e100, n100 = int(e // 100000), int(n // 100000)
    l1 = (19 - n100) - (19 - n100) % 5 + (e100 + 10) // 5
    l2 = (19 - n100) * 5 % 25 + e100 % 5
    l1 += l1 > 7
    l2 += l2 > 7
    return f"{chr(l1 + 65)}{chr(l2 + 65)}{int(e % 100000) // 1000:02d}{int(n % 100000) // 1000:02d}"


def parse_google_url(url):
    m = re.search(r"@(-?\d+\.\d+),(-?\d+\.\d+)", url)
    if not m:
        raise ValueError("Could not extract coordinates from URL. Ensure it contains '@lat,lon'.")
    return float(m.group(1)), float(m.group(2))


def resolve_tile(user_input):
    """(easting, northing) of the 1 km tile's south-west corner."""
    if user_input.startswith("http"):
        from rasterio.warp import transform
        lat, lon = parse_google_url(user_input)
        x, y = transform("EPSG:4326", BNG, [lon], [lat])
        return int(x[0] // 1000) * 1000, int(y[0] // 1000) * 1000
    e, n = gridref_to_en(user_input)
    return int(e // 1000) * 1000, int(n // 1000) * 1000


# --------------------------------------------------------------------------
# Grid + raster helpers
# --------------------------------------------------------------------------
class Grid:
    """The master 1 m grid (tile + buffer) every raster is aligned to."""
    def __init__(self, e0, n0, buf, res=1.0):
        from rasterio.transform import Affine
        self.e0, self.n0, self.buf, self.res = e0, n0, buf, res
        self.n = int(round((1000 + 2 * buf) / res))
        self.tr = Affine(res, 0, e0 - buf, 0, -res, n0 + 1000 + buf)
        self.bounds = (e0 - buf, n0 - buf, e0 + 1000 + buf, n0 + 1000 + buf)


def read_raster(data_or_path):
    """-> (float32 array, NaN = nodata), Affine transform."""
    import rasterio
    from rasterio.io import MemoryFile
    if isinstance(data_or_path, (bytes, bytearray)):
        with MemoryFile(data_or_path) as mf, mf.open() as ds:
            arr, nod, tr = ds.read(1).astype(np.float32), ds.nodata, ds.transform
    else:
        with rasterio.open(data_or_path) as ds:
            arr, nod, tr = ds.read(1).astype(np.float32), ds.nodata, ds.transform
    if nod is not None and not np.isnan(nod):
        arr[arr == nod] = np.nan
    arr[(arr < -50) | (arr > 150)] = np.nan          # unflagged nodata guard
    return arr, tr


def align_to_grid(arr, src_tr, grid):
    """Resample a BNG raster onto the master grid (nearest; handles any offset/size)."""
    from rasterio.warp import reproject, Resampling
    dst = np.full((grid.n, grid.n), np.nan, np.float32)
    reproject(arr, dst, src_transform=src_tr, src_crs=BNG, dst_transform=grid.tr, dst_crs=BNG,
              resampling=Resampling.nearest, src_nodata=np.nan, dst_nodata=np.nan)
    return dst


def wcs_get_tiff(source_key, bounds):
    url, cov = SOURCES[source_key]
    e0, n0, e1, n1 = bounds
    params = [("service", "WCS"), ("version", "2.0.1"), ("request", "GetCoverage"),
              ("coverageId", cov), ("format", "image/tiff"),
              ("subset", f"E({e0},{e1})"), ("subset", f"N({n0},{n1})")]
    last = None
    for attempt in range(3):
        try:
            r = requests.get(url, params=params, headers=HEADERS, timeout=180)
            body = r.content
            if r.status_code == 200 and body[:4] in (b"II*\x00", b"MM\x00*", b"II+\x00", b"MM\x00+"):
                return body
            last = f"WCS request failed ({r.status_code}) for {source_key}:\n{body[:600].decode('utf-8', 'replace')}\nURL: {r.url}"
        except requests.RequestException as ex:
            last = f"Could not reach the Defra WCS service for {source_key}: {str(ex)[:160]}"
    raise RuntimeError(last)


def get_lidar_layer(key, grid, tile, cache_dir, refresh, override=None):
    """Return the layer on the master grid (NaN = no data)."""
    if override:
        arr, tr = read_raster(override)
        return align_to_grid(arr, tr, grid)
    path = os.path.join(cache_dir, f"os_{tile}_{key}_b{grid.buf}.tif") if cache_dir else None
    if path and os.path.exists(path) and not refresh:
        arr, tr = read_raster(path)
        return align_to_grid(arr, tr, grid)
    body = wcs_get_tiff(key, grid.bounds)
    if path:
        os.makedirs(cache_dir, exist_ok=True)
        with open(path, "wb") as f:
            f.write(body)
    arr, tr = read_raster(body)
    return align_to_grid(arr, tr, grid)


# --------------------------------------------------------------------------
# Satellite imagery: Esri tiles -> OS grid
# --------------------------------------------------------------------------
def esri_mosaic(bounds, zoom, workers=12):
    """Download the Esri tiles covering a BNG box. Returns RGBA uint8 mosaic + Web-Mercator transform."""
    from rasterio.transform import Affine
    from rasterio.warp import transform
    e0, n0, e1, n1 = bounds
    t = np.linspace(0, 1, 21)
    xs = np.concatenate([e0 + (e1 - e0) * t, e0 + (e1 - e0) * t, np.full(21, e0), np.full(21, e1)])
    ys = np.concatenate([np.full(21, n0), np.full(21, n1), n0 + (n1 - n0) * t, n0 + (n1 - n0) * t])
    mx, my = transform(BNG, MERC, xs.tolist(), ys.tolist())
    res = 2 * ORIGIN / (256 * 2 ** zoom)
    px = (np.array(mx) + ORIGIN) / res
    py = (ORIGIN - np.array(my)) / res
    min_tx, max_tx = int(px.min() // 256), int(px.max() // 256)
    min_ty, max_ty = int(py.min() // 256), int(py.max() // 256)
    ntx, nty = max_tx - min_tx + 1, max_ty - min_ty + 1
    if ntx * nty > 3000:
        raise RuntimeError(f"Imagery request too large ({ntx * nty} tiles); lower --zoom.")
    canvas = np.zeros((nty * 256, ntx * 256, 4), np.uint8)
    sess = requests.Session()
    sess.headers.update(HEADERS)

    def fetch(job):
        tx, ty = job
        for _ in range(3):
            try:
                r = sess.get(ESRI_URL.format(z=zoom, y=ty, x=tx), timeout=30)
                if r.status_code == 200:
                    return job, np.asarray(Image.open(BytesIO(r.content)).convert("RGB"))
            except Exception:
                pass
        return job, None

    jobs = [(tx, ty) for ty in range(min_ty, max_ty + 1) for tx in range(min_tx, max_tx + 1)]
    missing = 0
    with ThreadPoolExecutor(workers) as ex:
        for (tx, ty), tile in ex.map(fetch, jobs):
            if tile is None or tile.shape[:2] != (256, 256):
                missing += 1
                continue
            y0, x0 = (ty - min_ty) * 256, (tx - min_tx) * 256
            canvas[y0:y0 + 256, x0:x0 + 256, :3] = tile
            canvas[y0:y0 + 256, x0:x0 + 256, 3] = 255
    if missing > 0.2 * len(jobs):
        raise RuntimeError(f"{missing}/{len(jobs)} imagery tiles failed to download.")
    if missing:
        print(f"  warning: {missing} imagery tiles missing (left blank)")
    tr = Affine(res, 0, -ORIGIN + min_tx * 256 * res, 0, -res, ORIGIN - min_ty * 256 * res)
    return canvas, tr


def mosaic_to_bng(canvas, src_tr, grid, img_res):
    """Re-project an RGBA Web-Mercator mosaic onto the OS grid at img_res m/px. Returns (H,W,4) uint8."""
    from rasterio.transform import Affine
    from rasterio.warp import reproject, Resampling
    n = int(round((1000 + 2 * grid.buf) / img_res))
    dst_tr = Affine(img_res, 0, grid.e0 - grid.buf, 0, -img_res, grid.n0 + 1000 + grid.buf)
    src = np.ascontiguousarray(np.moveaxis(canvas, 2, 0))
    dst = np.zeros((4, n, n), np.uint8)
    reproject(src, dst, src_transform=src_tr, src_crs=MERC, dst_transform=dst_tr, dst_crs=BNG,
              resampling=Resampling.bilinear)
    return np.ascontiguousarray(np.moveaxis(dst, 0, 2))


def save_rgb_tif(path, rgb, transform, jpeg=True):
    import rasterio
    base = dict(driver="GTiff", height=rgb.shape[0], width=rgb.shape[1], count=3, dtype="uint8",
                crs=BNG, transform=transform, tiled=True, blockxsize=256, blockysize=256)
    attempts = [dict(compress="jpeg", photometric="ycbcr", jpeg_quality=92), dict(compress="deflate")]
    for opts in (attempts if jpeg else attempts[1:]):
        try:
            with rasterio.open(path, "w", **base, **opts) as ds:
                ds.write(np.moveaxis(rgb, 2, 0))
            return
        except Exception:
            continue
    raise RuntimeError(f"Could not write {path}")


def load_image_tif(path, grid, img_res):
    """Place any local RGB GeoTIFF (BNG) onto the buffered imagery grid. Returns (rgb, valid)."""
    import rasterio
    n = int(round((1000 + 2 * grid.buf) / img_res))
    rgb = np.zeros((n, n, 3), np.uint8)
    with rasterio.open(path) as ds:
        if abs(ds.transform.a - img_res) > 1e-6:
            raise RuntimeError(f"--image-tif is {ds.transform.a} m/px but --img-res is {img_res}")
        data = np.moveaxis(ds.read([1, 2, 3]), 0, 2)
        c0 = int(round((ds.transform.c - (grid.e0 - grid.buf)) / img_res))
        r0 = int(round(((grid.n0 + 1000 + grid.buf) - ds.transform.f) / img_res))
    ys, xs = max(r0, 0), max(c0, 0)
    ye, xe = min(r0 + data.shape[0], n), min(c0 + data.shape[1], n)
    rgb[ys:ye, xs:xe] = data[ys - r0:ye - r0, xs - c0:xe - c0]
    return rgb, rgb.max(axis=2) > 8


def get_imagery(grid, tile, cache_dir, refresh, zoom, img_res, workers):
    """Buffered RGB (H,W,3) + valid mask on the OS grid at img_res (cached as a JPEG GeoTIFF)."""
    path = os.path.join(cache_dir, f"os_{tile}_imagery_b{grid.buf}_z{zoom}_r{img_res:g}.tif") if cache_dir else None
    if path and os.path.exists(path) and not refresh:
        import rasterio
        with rasterio.open(path) as ds:
            rgb = np.moveaxis(ds.read(), 0, 2)
        return rgb, rgb.max(axis=2) > 8
    canvas, tr = esri_mosaic(grid.bounds, zoom, workers)
    out = mosaic_to_bng(canvas, tr, grid, img_res)
    rgb, valid = out[..., :3].copy(), out[..., 3] > 127
    rgb[~valid] = 0
    if path:
        from rasterio.transform import Affine
        os.makedirs(cache_dir, exist_ok=True)
        save_rgb_tif(path, rgb, Affine(img_res, 0, grid.e0 - grid.buf, 0, -img_res, grid.n0 + 1000 + grid.buf))
    return rgb, valid


# --------------------------------------------------------------------------
# Imagery features + registration
# --------------------------------------------------------------------------
def exg_index(rgb):
    """Excess-green index (2g - r - b)/(r+g+b), per pixel. Foliage is high; roofs, tarmac and
    shadow stay low even though this imagery has an overall green cast."""
    f = rgb.astype(np.float32) / 255.0
    tot = f.sum(axis=2) + 1e-6
    return (2 * f[..., 1] - f[..., 0] - f[..., 2]) / tot


def green_mask(rgb, thr):
    return (exg_index(rgb) >= thr) & (rgb.astype(np.float32).sum(axis=2) > 15)


def block_mean(a, f):
    h, w = (a.shape[0] // f) * f, (a.shape[1] // f) * f
    return a[:h, :w].reshape(h // f, f, w // f, f).mean(axis=(1, 3))


def tree_proxy(rgb, valid, thr):
    """Per-pixel 'tree-like' score used for registration: strongly green AND darker than lawn."""
    f = rgb.astype(np.float32) / 255.0
    lum = 0.299 * f[..., 0] + 0.587 * f[..., 1] + 0.114 * f[..., 2]
    return (green_mask(rgb, thr + 0.1) & valid & (lum < 0.25)).astype(np.float32)


def estimate_shift(ref, mov, max_shift):
    """FFT cross-correlation. Returns (dy, dx) in pixels such that ndi.shift(mov, (dy, dx)) ~ ref."""
    h, w = ref.shape
    win = np.outer(np.hanning(h), np.hanning(w)).astype(np.float32)
    a = (ref - ref.mean()) * win
    b = (mov - mov.mean()) * win
    cc = np.fft.ifft2(np.fft.fft2(a) * np.conj(np.fft.fft2(b))).real
    cc = np.fft.fftshift(cc)
    cy, cx = h // 2, w // 2
    m = int(math.ceil(max_shift))
    sub = cc[cy - m - 1:cy + m + 2, cx - m - 1:cx + m + 2]
    yy, xx = np.mgrid[-m - 1:m + 2, -m - 1:m + 2]
    sub = np.where((yy ** 2 + xx ** 2) <= (m + 0.5) ** 2, sub, -np.inf)
    iy, ix = np.unravel_index(np.argmax(sub), sub.shape)
    iy = int(np.clip(iy, 1, sub.shape[0] - 2)); ix = int(np.clip(ix, 1, sub.shape[1] - 2))

    def parab(l, c, r):
        d = l - 2 * c + r
        return 0.0 if d == 0 else 0.5 * (l - r) / d
    dy = (iy - m - 1) + parab(sub[iy - 1, ix], sub[iy, ix], sub[iy + 1, ix])
    dx = (ix - m - 1) + parab(sub[iy, ix - 1], sub[iy, ix], sub[iy, ix + 1])
    return float(dy), float(dx)


def corr(a, b):
    a = a - a.mean(); b = b - b.mean()
    d = math.sqrt(float((a * a).sum() * (b * b).sum())) + 1e-9
    return float((a * b).sum() / d)


def register_imagery(rgb, valid, lidar_veg, f, max_shift, thr):
    """Find the translation (metres) that best aligns the imagery to the LiDAR vegetation mask.
    Returns (rgb_shifted, valid_shifted, (dE_m, dN_m), info_str)."""
    prox = block_mean(tree_proxy(rgb, valid, thr), f)
    ref = ndi.gaussian_filter(lidar_veg.astype(np.float32), 1.0)
    mov = ndi.gaussian_filter(prox, 1.0)
    before = corr(ref, mov)
    dy, dx = estimate_shift(ref, mov, max_shift)
    moved = ndi.shift(mov, (dy, dx), order=1, mode="constant")
    after = corr(ref, moved)
    msg = f"correlation {before:.3f} -> {after:.3f}"
    if after < before * 1.03 + 0.005 or math.hypot(dy, dx) < 0.3:
        return rgb, valid, (0.0, 0.0), msg + " (kept as-is)"
    sh = (dy * f, dx * f, 0)
    rgb2 = ndi.shift(rgb, sh, order=1, mode="constant", cval=0).astype(np.uint8)
    valid2 = ndi.shift(valid.astype(np.float32), sh[:2], order=1, mode="constant") > 0.5
    # +dx px = image moved east, +dy px = image moved south (rows go down = northing decreases)
    return rgb2, valid2, (dx, -dy), msg + f" (shifted imagery {dx:+.1f} m E, {-dy:+.1f} m N)"


# --------------------------------------------------------------------------
# Fusion: LiDAR + imagery -> vegetation height surface
# --------------------------------------------------------------------------
def local_roughness(h):
    m = ndi.uniform_filter(h, 3)
    m2 = ndi.uniform_filter(h * h, 3)
    return ndi.uniform_filter(np.sqrt(np.clip(m2 - m * m, 0, None)), 3)


def fuse(vom, dsmdtm, green1m, p):
    """Return (veg_height, rescued_mask)."""
    veg = np.where(vom >= p.min_height, vom, 0.0).astype(np.float32)
    rescued = np.zeros(veg.shape, bool)
    if dsmdtm is not None and green1m is not None:
        cand = (veg <= 0) & (dsmdtm >= p.min_height) & (dsmdtm <= p.max_tree_height)
        cand &= green1m >= p.green_min
        cand &= local_roughness(dsmdtm) >= p.rough_min
        cand = ndi.binary_opening(cand, np.ones((3, 3), bool))
        cand = morphology.remove_small_objects(cand, p.min_area_px)
        rescued = cand
        veg = np.where(rescued, dsmdtm, veg)
    return veg, rescued


# --------------------------------------------------------------------------
# Crown detection
# --------------------------------------------------------------------------
def log_int(sm):
    return np.round(np.log(np.maximum(sm, 1.0)) * 1000).astype(np.int32)


def merge_small(lab, sm, mask, min_area_px):
    """Absorb watershed fragments smaller than min_area_px into the neighbour they touch.
    Small crowns that touch nothing (isolated small trees) keep their own label."""
    cnt = np.bincount(lab.ravel())
    small = np.nonzero((cnt < min_area_px) & (np.arange(len(cnt)) > 0) & (cnt > 0))[0]
    if len(small) == 0:
        return lab
    keep = np.where(np.isin(lab, small), 0, lab)
    out = segmentation.watershed(-sm, keep, mask=mask)
    return np.where((out == 0) & (lab > 0), lab, out)


def smooth_masked(chm, mask, sigma, floor):
    """Gaussian smoothing that does not bleed the zero background into crown edges."""
    num = ndi.gaussian_filter(np.where(mask, np.maximum(chm, floor), 0).astype(np.float32), sigma)
    den = np.maximum(ndi.gaussian_filter(mask.astype(np.float32), sigma), 1e-3)
    return np.where(mask, num / den, 0).astype(np.float32)


def split_big(lab, sm_fine, p, mpp):
    """Split crowns wider than max_crown: finer watershed first, k-means as a fallback."""
    max_area = math.pi * (p.max_crown / 2) ** 2 / mpp ** 2
    tgt_area = math.pi * (0.6 * p.max_crown / 2) ** 2 / mpp ** 2
    out = lab.copy()
    nxt = int(lab.max()) + 1
    was_split = set()
    hf = max(1, int(round(p.prominence * 1000 / 3)))
    for reg in measure.regionprops(lab):
        if reg.area <= max_area:
            continue
        r0, c0, r1, c1 = reg.bbox
        m = reg.image
        fine = sm_fine[r0:r1, c0:c1]
        parts = np.where(m, 1, 0)
        mk = measure.label(morphology.h_maxima(np.where(m, log_int(fine), 0), hf).astype(np.uint8) & m)
        if mk.max() >= 2:
            ws = merge_small(segmentation.watershed(-fine, mk, mask=m), fine, m, p.merge_area_px)
            if len(np.unique(ws[m])) >= 2:
                parts = ws
        for pid in np.unique(parts[m]):
            pm = parts == pid
            ys, xs = np.nonzero(pm)
            if len(ys) > max_area:
                k = max(2, int(round(len(ys) / tgt_area)))
                _, cl = kmeans2(np.column_stack([ys, xs]).astype(float), k, minit="++", seed=0, missing="warn")
                for ci in range(k):
                    sel = cl == ci
                    if sel.any():
                        out[r0 + ys[sel], c0 + xs[sel]] = nxt
                        was_split.add(nxt); nxt += 1
            else:
                out[r0 + ys, c0 + xs] = nxt
                was_split.add(nxt); nxt += 1
    return out, was_split


def detect(veg, rescued, green1m, mpp, p):
    mask = veg >= p.min_height
    mask = ndi.binary_closing(mask, np.ones((3, 3), bool))
    mask = ndi.binary_fill_holes(mask)
    mask = morphology.remove_small_objects(mask, p.min_area_px)
    sm = smooth_masked(veg, mask, max(0.5, p.smooth_m / mpp), p.min_height)
    h = max(1, int(round(p.prominence * 1000)))          # relative prominence on log-height
    markers = measure.label(morphology.h_maxima(log_int(sm), h).astype(np.uint8) & mask)
    lab = segmentation.watershed(-sm, markers, mask=mask)
    lab = merge_small(lab, sm, mask, p.merge_area_px)

    sm_fine = smooth_masked(veg, mask, max(0.5, 0.7 / mpp), p.min_height)
    lab, was_split = split_big(lab, sm_fine, p, mpp)

    max_r_px = (p.max_crown / mpp) / 2
    min_r_px = (p.min_crown / mpp) / 2
    trees = []
    for reg in measure.regionprops(lab, intensity_image=veg):
        diam_px = 2 * math.sqrt(reg.area / math.pi)
        if diam_px / 2 < min_r_px * 0.7:
            continue
        sel = lab[reg.slice] == reg.label
        cy, cx = reg.centroid
        trees.append({
            "row": float(cy), "col": float(cx),
            "area_m2": reg.area * mpp * mpp, "diam_m": diam_px * mpp,
            "height_m": float(reg.intensity_max),
            "green": float(green1m[reg.slice][sel].mean()) if green1m is not None else float("nan"),
            "rescued": float(rescued[reg.slice][sel].mean()),
            "split": reg.label in was_split,
            "big": diam_px / 2 > max_r_px,
        })
    return trees, lab


# --------------------------------------------------------------------------
# Hedges and shrubs (everything green and 1-6 m tall that is not a tree crown)
# --------------------------------------------------------------------------
_NB = [(-1, -1), (-1, 0), (-1, 1), (0, -1), (0, 1), (1, -1), (1, 0), (1, 1)]


def _bfs(start, allowed):
    dist, par, far, q = {start: 0.0}, {start: None}, start, deque([start])
    while q:
        u = q.popleft()
        for dr, dc in _NB:
            v = (u[0] + dr, u[1] + dc)
            if v in allowed and v not in dist:
                dist[v] = dist[u] + (1.414 if dr and dc else 1.0)
                par[v] = u
                q.append(v)
                if dist[v] > dist[far]:
                    far = v
    return far, dist, par


def skeleton_paths(sel, min_len):
    """Longest skeleton path of a binary mask, then its branches. Returns [(pixels, length_px)]."""
    remaining = set(zip(*np.nonzero(morphology.skeletonize(sel))))
    out = []
    while remaining:
        a, _, _ = _bfs(next(iter(remaining)), remaining)
        b, dist, par = _bfs(a, remaining)
        path, u = [], b
        while u is not None:
            path.append(u)
            u = par[u]
        if dist[b] >= min_len:
            out.append((path, dist[b]))
        drop = set()
        for (r, c) in path:
            drop.add((r, c))
            for dr, dc in _NB:
                drop.add((r + dr, c + dc))
        remaining -= drop
    return out


def find_hedges_shrubs(h, green1m, tree_lab, grid, p):
    """h = DSM-DTM height on the master grid. Returns (features, n_ignored_patches)."""
    mpp = grid.res
    hs = ndi.rank_filter(h, rank=p.hedge_rank, size=3)      # speckle filter (4 = median, 6 = keeps small shrubs)
    cand = (hs >= p.hedge_min_height) & (hs <= p.hedge_max_height) & (green1m >= p.hedge_green)
    cand &= ~ndi.binary_dilation(tree_lab > 0, iterations=1)
    if p.hedge_open:      # old behaviour: also erases hedges one cell wide
        cand = ndi.binary_opening(cand, ndi.generate_binary_structure(2, 1))
    else:                 # keep thin lines, drop isolated specks (a cell needs >= 2 neighbours)
        cand &= ndi.convolve(cand.astype(np.uint8), np.ones((3, 3), np.uint8), mode="constant") >= 3
    cand = morphology.remove_small_objects(cand, max(3, int(p.min_shrub_area / mpp ** 2)))
    lab = measure.label(cand, connectivity=2)
    feats, patches = [], 0
    min_len = p.hedge_min_length / mpp
    for reg in measure.regionprops(lab):
        area = reg.area * mpp ** 2
        r0, c0, r1, c1 = reg.bbox
        sel = reg.image
        hh = h[r0:r1, c0:c1]
        gg = green1m[r0:r1, c0:c1]
        height = float(np.percentile(hh[sel], 90))
        green = float(gg[sel].mean())
        if height >= p.flat_roof_height and float(hh[sel].std()) < p.flat_roof_std:
            patches += 1                                    # tall AND dead flat = green roof / wall, not a plant
            continue
        made_hedge = False
        if max(r1 - r0, c1 - c0) >= min_len:
            edt = ndi.distance_transform_edt(np.pad(sel, 1))[1:-1, 1:-1]
            for path, plen in skeleton_paths(sel, min_len):
                width = float(2 * np.mean([edt[r, c] for r, c in path]) * mpp)
                if width > p.hedge_max_width or plen * mpp < p.hedge_min_length:
                    continue
                pr = np.array(path)
                simp = measure.approximate_polygon(pr.astype(float), tolerance=0.8)
                verts = [tuple(grid.tr * (c0 + c + 0.5, r0 + r + 0.5)) for r, c in simp]
                mid = pr[len(pr) // 2]
                ph = [hh[max(r - 1, 0):r + 2, max(c - 1, 0):c + 2].max() for r, c in path]
                e, n = grid.tr * (c0 + mid[1] + 0.5, r0 + mid[0] + 0.5)
                feats.append({"type": "hedge", "easting": e, "northing": n, "length_m": plen * mpp,
                              "width_m": width, "height_m": float(np.percentile(ph, 75)),
                              "area_m2": float(len(pr) * width * mpp), "green": green, "verts": verts})
                made_hedge = True
        if made_hedge:
            continue
        if area <= p.shrub_max_area and max(r1 - r0, c1 - c0) * mpp <= 14:
            cy, cx = reg.centroid
            e, n = grid.tr * (cx + 0.5, cy + 0.5)
            feats.append({"type": "shrub", "easting": e, "northing": n, "length_m": float(reg.major_axis_length * mpp),
                          "width_m": 2 * math.sqrt(area / math.pi), "height_m": height, "area_m2": area,
                          "green": green, "verts": []})
        else:
            patches += 1
    return feats, patches


# --------------------------------------------------------------------------
# Output
# --------------------------------------------------------------------------
def draw_overlay(base_rgb, rows, grid, img_res, off_px, scale):
    """Draw crowns on a (tile-cropped) image. rows carry easting/northing in OS metres."""
    ov = Image.fromarray(base_rgb)
    d = ImageDraw.Draw(ov)
    w = max(1, int(round(2 * scale)))
    for t in rows:
        cx = (t["easting"] - (grid.e0 - grid.buf)) / img_res - off_px
        cy = ((grid.n0 + 1000 + grid.buf) - t["northing"]) / img_res - off_px
        r = t["diam_m"] / img_res / 2
        col = (255, 60, 60) if t["big"] else ((0, 255, 255) if t["rescued"] > 0.5 else (255, 255, 0))
        d.ellipse([cx - r, cy - r, cx + r, cy + r], outline=col, width=w)
        d.rectangle([cx - 1.5 * w, cy - 1.5 * w, cx + 1.5 * w, cy + 1.5 * w], fill=col)
    d.rectangle([10, 10, 420, 112], fill=(0, 0, 0))
    for i, (c, s) in enumerate([((255, 255, 0), "LiDAR tree (VOM)"),
                                ((0, 255, 255), "rescued by satellite + LiDAR"),
                                ((255, 60, 60), "crown wider than --max-crown")]):
        d.rectangle([20, 22 + i * 28, 40, 38 + i * 28], fill=c)
        d.text((52, 24 + i * 28), s, fill=(255, 255, 255))
    return ov


def draw_hedge_overlay(base_rgb, tree_rows, feats, grid, img_res, off_px):
    ov = Image.fromarray(base_rgb)
    d = ImageDraw.Draw(ov)

    def px(e, n):
        return ((e - (grid.e0 - grid.buf)) / img_res - off_px, ((grid.n0 + 1000 + grid.buf) - n) / img_res - off_px)
    for t in tree_rows:
        cx, cy = px(t["easting"], t["northing"])
        r = t["diam_m"] / img_res / 2
        d.ellipse([cx - r, cy - r, cx + r, cy + r], outline=(255, 255, 0), width=1)
    for f in feats:
        if f["type"] == "hedge":
            pts = [px(e, n) for e, n in f["verts"]]
            d.line(pts, fill=(255, 140, 0), width=max(4, int(f["width_m"] / img_res)))
        else:
            cx, cy = px(f["easting"], f["northing"])
            r = max(f["width_m"] / img_res / 2, 6)
            d.ellipse([cx - r, cy - r, cx + r, cy + r], outline=(255, 0, 255), width=3)
    d.rectangle([10, 10, 470, 112], fill=(0, 0, 0))
    for i, (c, t) in enumerate([((255, 255, 0), "tree crown"), ((255, 140, 0), "hedge (line, width = hedge width)"),
                                ((255, 0, 255), "shrub")]):
        d.rectangle([20, 22 + i * 28, 40, 38 + i * 28], fill=c)
        d.text((52, 24 + i * 28), t, fill=(255, 255, 255))
    return ov


def chm_background(veg, grid):
    g = np.clip(veg / 25.0, 0, 1)
    return np.stack([g * 120, 40 + g * 215, g * 90], -1).astype(np.uint8)


# --------------------------------------------------------------------------
# Main
# --------------------------------------------------------------------------
def build_parser():
    ap = argparse.ArgumentParser(description="LiDAR + satellite tree crown detection for a 1 km OS tile")
    ap.add_argument("input_val", nargs="?", help="OS grid reference (e.g. TQ2981) or Google Maps URL")
    ap.add_argument("--out", default=".", help="Output directory")
    ap.add_argument("--buffer", type=int, default=20, help="Metres fetched around the tile (edge effects)")
    ap.add_argument("--zoom", type=int, default=19, help="Esri imagery zoom (19 ~ 0.18 m/px)")
    ap.add_argument("--img-res", type=float, default=0.25, help="Satellite GeoTIFF resolution, m/px")
    ap.add_argument("--workers", type=int, default=12, help="Parallel imagery downloads")
    ap.add_argument("--no-imagery", action="store_true", help="LiDAR only (no satellite, no rescue)")
    ap.add_argument("--no-rescue", action="store_true", help="Do not add DSM-DTM trees missed by the VOM")
    ap.add_argument("--no-register", action="store_true", help="Skip imagery-to-LiDAR registration")
    ap.add_argument("--no-cache", action="store_true", help="Do not keep downloaded rasters")
    ap.add_argument("--refresh", action="store_true", help="Re-download even if cached")
    ap.add_argument("--overlay-tif", action="store_true", help="Also write a georeferenced overlay GeoTIFF")
    ap.add_argument("--vom-tif"); ap.add_argument("--dsm-tif"); ap.add_argument("--dtm-tif")
    ap.add_argument("--image-tif", help="Use a local satellite GeoTIFF already on the OS grid (skip download)")
    g = ap.add_argument_group("detection tuning")
    g.add_argument("--min-height", type=float, default=2.5)
    g.add_argument("--max-tree-height", type=float, default=45.0)
    g.add_argument("--min-crown", type=float, default=2.5)
    g.add_argument("--max-crown", type=float, default=14.0, help="Wider crowns are split")
    g.add_argument("--smooth", dest="smooth_m", type=float, default=1.0, help="Treetop smoothing, metres")
    g.add_argument("--prominence", type=float, default=0.15,
                   help="Treetop must rise this FRACTION of its height above the saddle to a neighbour "
                        "(raise = fewer, bigger crowns in woodland; lower = more, smaller)")
    g.add_argument("--merge-area", type=float, default=25.0,
                   help="Crown fragments smaller than this (m2) that touch a neighbour are merged into it (woodland over-splitting)")
    g.add_argument("--green-exg", type=float, default=0.0, help="Foliage threshold on excess-green (0 = auto-calibrate per tile)")
    g.add_argument("--green-min", type=float, default=0.5, help="Rescue: min fraction of green pixels per 1 m cell")
    g.add_argument("--rough-min", type=float, default=0.15, help="Rescue: min local height roughness (m); rejects roofs")
    hg = ap.add_argument_group("hedges and shrubs")
    hg.add_argument("--no-hedges", action="store_true", help="Skip the hedge / shrub pass")
    hg.add_argument("--hedge-min-height", type=float, default=1.0)
    hg.add_argument("--hedge-max-height", type=float, default=6.0)
    hg.add_argument("--hedge-green", type=float, default=0.4, help="Min green fraction per 1 m cell for hedges/shrubs")
    hg.add_argument("--hedge-rank", type=int, default=5, choices=range(0, 9),
                    help="Speckle filter rank on 3x3 (4 = median; 5-6 keeps smaller shrubs but also more noise)")
    hg.add_argument("--flat-roof-height", type=float, default=3.5, help="Objects at least this tall and flatter than --flat-roof-std are rejected")
    hg.add_argument("--flat-roof-std", type=float, default=0.25)
    hg.add_argument("--hedge-open", action="store_true", help="(old) morphological opening; erases 1-cell-wide hedges")
    hg.add_argument("--hedge-min-length", type=float, default=4.0, help="Shortest hedge line, m")
    hg.add_argument("--hedge-max-width", type=float, default=5.0, help="Wider than this is scrub, not a hedge")
    hg.add_argument("--min-shrub-area", type=float, default=3.0, help="Smallest shrub, m2")
    hg.add_argument("--shrub-max-area", type=float, default=60.0, help="Bigger soft patches are ignored")
    g.add_argument("--max-shift", type=float, default=30.0, help="Max imagery registration shift searched, m")
    return ap


def main(argv=None):
    a = build_parser().parse_args(argv)
    if not a.input_val:
        print("\n--- 1km OS Tile Tree Mapper (LiDAR + satellite) ---")
        a.input_val = input("Enter a Google Maps URL OR an OS Grid reference (e.g. TQ2981): ").strip()
    os.makedirs(a.out, exist_ok=True)
    cache = None if a.no_cache else os.path.join(a.out, "_cache")

    e0, n0 = resolve_tile(a.input_val)
    if not (134000 <= e0 <= 655000 and 11000 <= n0 <= 657000):
        raise ValueError(f"E {e0}, N {n0} is outside the Environment Agency LiDAR coverage (England). Check the grid reference.")
    tile = en_to_tilename(e0, n0)
    grid = Grid(e0, n0, a.buffer)
    mpp = grid.res
    a.min_area_px = max(4, int(round(math.pi * (a.min_crown / 2) ** 2 * 0.5 / mpp ** 2)))
    a.merge_area_px = max(a.min_area_px, int(round(a.merge_area / mpp ** 2)))
    print(f"OS tile {tile}: E {e0}-{e0 + 1000}, N {n0}-{n0 + 1000}")

    # 1. LiDAR ---------------------------------------------------------------
    print("LiDAR: fetching vegetation model (VOM)...")
    vom = get_lidar_layer("vom", grid, tile, cache, a.refresh, a.vom_tif)
    if not np.isfinite(vom).any() or np.nanmax(vom) <= 0:
        raise RuntimeError("The VOM has no vegetation for this tile (outside England, or no LiDAR coverage?).")
    vom = np.nan_to_num(vom, nan=0.0)
    dsmdtm = None
    if not a.no_imagery and not (a.no_rescue and a.no_hedges):
        try:
            print("LiDAR: fetching DSM + DTM (rescue missed trees, find hedges and shrubs)...")
            dsm = get_lidar_layer("dsm", grid, tile, cache, a.refresh, a.dsm_tif)
            dtm = get_lidar_layer("dtm", grid, tile, cache, a.refresh, a.dtm_tif)
            dsmdtm = np.nan_to_num(np.clip(dsm - dtm, 0, None), nan=0.0)
        except RuntimeError as ex:
            print(f"  warning: DSM/DTM unavailable, continuing with VOM only.\n  {str(ex).splitlines()[0]}")

    # 2. Imagery -------------------------------------------------------------
    rgb = valid = None
    f = int(round(1.0 / a.img_res)) if a.img_res else 4
    if not a.no_imagery:
        print("Imagery: " + ("loading local file..." if a.image_tif else f"Esri zoom {a.zoom}, re-projecting onto the OS grid at {a.img_res} m/px..."))
        if a.image_tif:
            rgb, valid = load_image_tif(a.image_tif, grid, a.img_res)
        else:
            rgb, valid = get_imagery(grid, tile, cache, a.refresh, a.zoom, a.img_res, a.workers)
        if abs(1.0 / a.img_res - f) > 1e-6:
            raise RuntimeError("--img-res must divide 1 m evenly (0.25, 0.5, 0.2, 0.125 ...)")
        if not a.no_register:
            rgb, valid, (dE, dN), info = register_imagery(rgb, valid, vom > 0, f, a.max_shift, 0.40)
            print(f"  registration: {info}")
            if math.hypot(dE, dN) > a.max_shift * 0.9:
                print("  warning: shift is at the search limit - check the overlay alignment")

    # 3. Fusion --------------------------------------------------------------
    green1m = None
    if rgb is not None:
        thr = a.green_exg
        if thr <= 0:      # calibrate from this tile's own LiDAR-confirmed trees
            inside = np.kron(vom >= a.min_height, np.ones((f, f), bool))[:rgb.shape[0], :rgb.shape[1]] & valid
            thr = float(np.clip(np.percentile(exg_index(rgb)[inside], 10), 0.30, 0.55)) if inside.sum() > 1000 else 0.40
        print(f"Imagery: foliage threshold ExG >= {thr:.2f}")
        green1m = block_mean(green_mask(rgb, thr) & valid, f).astype(np.float32)
    veg, rescued = fuse(vom, None if a.no_rescue else dsmdtm, green1m, a)
    print(f"Fusion: {int((vom >= a.min_height).sum()):,} VOM canopy cells + {int(rescued.sum()):,} rescued cells")

    # 4. Crowns --------------------------------------------------------------
    trees, tree_lab = detect(veg, rescued, green1m, mpp, a)
    rows = []
    for t in trees:
        x, y = grid.tr * (t["col"] + 0.5, t["row"] + 0.5)
        if not (e0 <= x < e0 + 1000 and n0 <= y < n0 + 1000):
            continue
        t.update(easting=x, northing=y, lx=x - e0, ly=y - n0)
        rows.append(t)

    img_res = a.img_res
    csv_path = os.path.join(a.out, f"os_{tile}_lidar_trees.csv")
    with open(csv_path, "w", newline="") as fh:
        wr = csv.writer(fh)
        wr.writerow(["id", "parent_location", "x_px", "y_px", "x_m", "y_m", "os_easting", "os_northing",
                     "crown_diam_m", "crown_area_m2", "height_m", "green_frac", "source", "flag"])
        for i, t in enumerate(rows, 1):
            flags = [s for s, on in (("oversized", t["big"]), ("split", t["split"])) if on]
            wr.writerow([i, tile, "%.1f" % (t["lx"] / img_res), "%.1f" % ((1000 - t["ly"]) / img_res),
                         "%.2f" % t["lx"], "%.2f" % t["ly"], "%.2f" % t["easting"], "%.2f" % t["northing"],
                         "%.2f" % t["diam_m"], "%.1f" % t["area_m2"], "%.1f" % t["height_m"],
                         "" if math.isnan(t["green"]) else "%.2f" % t["green"],
                         "satellite+lidar" if t["rescued"] > 0.5 else "vom", ";".join(flags)])

    # Overlay + satellite tile -------------------------------------------------
    from rasterio.transform import Affine
    if rgb is not None:
        off = int(round(a.buffer / img_res))
        n_px = int(round(1000 / img_res))
        tile_rgb = np.ascontiguousarray(rgb[off:off + n_px, off:off + n_px])
        sat_path = os.path.join(a.out, f"os_{tile}_satellite.tif")
        tile_tr = Affine(img_res, 0, e0, 0, -img_res, n0 + 1000)
        save_rgb_tif(sat_path, tile_rgb, tile_tr)
        ov = draw_overlay(tile_rgb, rows, grid, img_res, off, 1.0)
    else:
        sat_path = None
        view = veg[int(a.buffer):int(a.buffer) + 1000, int(a.buffer):int(a.buffer) + 1000]
        base = Image.fromarray(chm_background(view, grid)).resize((3000, 3000), Image.NEAREST)
        ov = draw_overlay(np.asarray(base), rows, grid, 1.0 / 3.0, int(a.buffer) * 3, 1.0)
    ov_path = os.path.join(a.out, f"os_{tile}_trees_on_{'satellite' if rgb is not None else 'chm'}.jpg")
    ov.save(ov_path, "JPEG", quality=85)
    if a.overlay_tif and rgb is not None:
        save_rgb_tif(os.path.join(a.out, f"os_{tile}_trees_on_satellite.tif"), np.asarray(ov), tile_tr)

    # 5. Hedges and shrubs ------------------------------------------------------
    feats, n_patch = [], 0
    if not a.no_hedges:
        if dsmdtm is None or green1m is None:
            print("Hedges/shrubs: skipped (needs the DSM/DTM and the satellite imagery)")
        else:
            allf, n_patch = find_hedges_shrubs(dsmdtm, green1m, tree_lab, grid, a)
            feats = [f for f in allf if e0 <= f["easting"] < e0 + 1000 and n0 <= f["northing"] < n0 + 1000]
            hs_path = os.path.join(a.out, f"os_{tile}_hedges_shrubs.csv")
            hp_path = os.path.join(a.out, f"os_{tile}_hedge_paths.csv")
            with open(hs_path, "w", newline="") as fh, open(hp_path, "w", newline="") as fp:
                w1, w2 = csv.writer(fh), csv.writer(fp)
                w1.writerow(["id", "type", "x_m", "y_m", "os_easting", "os_northing", "length_m", "width_m",
                             "height_m", "area_m2", "green_frac"])
                w2.writerow(["hedge_id", "seq", "x_m", "y_m", "os_easting", "os_northing", "width_m", "height_m"])
                for i, f in enumerate(feats, 1):
                    f["id"] = i
                    w1.writerow([i, f["type"], "%.2f" % (f["easting"] - e0), "%.2f" % (f["northing"] - n0),
                                 "%.2f" % f["easting"], "%.2f" % f["northing"], "%.1f" % f["length_m"],
                                 "%.1f" % f["width_m"], "%.1f" % f["height_m"], "%.1f" % f["area_m2"],
                                 "%.2f" % f["green"]])
                    for k, (e, n) in enumerate(f["verts"]):
                        w2.writerow([i, k, "%.2f" % (e - e0), "%.2f" % (n - n0), "%.2f" % e, "%.2f" % n,
                                     "%.1f" % f["width_m"], "%.1f" % f["height_m"]])
            if rgb is not None:
                hov = draw_hedge_overlay(tile_rgb, rows, feats, grid, img_res, off)
                hov.save(os.path.join(a.out, f"os_{tile}_hedges_on_satellite.jpg"), "JPEG", quality=85)
    n_hedge = sum(1 for f in feats if f["type"] == "hedge")
    n_shrub = len(feats) - n_hedge
    if not a.no_hedges and feats:
        tot_len = sum(f["length_m"] for f in feats if f["type"] == "hedge")
        print(f"Hedges/shrubs: {n_hedge} hedge lines ({tot_len:.0f} m in total), {n_shrub} shrubs, "
              f"{n_patch} larger soft patches ignored")

    n_res = sum(1 for t in rows if t["rescued"] > 0.5)
    n_big = sum(1 for t in rows if t["big"])
    print(f"\nComplete! {len(rows)} trees ({n_res} rescued via satellite, {n_big} oversized)")
    print(f"  {csv_path}")
    if sat_path:
        print(f"  {sat_path}")
    print(f"  {ov_path}")


if __name__ == "__main__":
    try:
        main()
    except (RuntimeError, ValueError) as e:
        sys.exit(f"\nERROR: {e}")
