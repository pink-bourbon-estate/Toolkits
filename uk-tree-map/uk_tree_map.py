#!/usr/bin/env python3
"""
uk_tree_map.py - open tree-crown map of England from free LiDAR, in British National Grid
=========================================================================================

Works on 1 km Ordnance Survey National Grid tiles. For each tile it downloads the Environment
Agency's Vegetation Object Model (VOM, 1 m canopy height, Open Government Licence) and detects
individual tree crowns:

  treetops = h-maxima on a log-height surface (prominence is RELATIVE to tree height, so tall
             woodland trees are not over-split), crowns = watershed, tiny fragments merged into
             a neighbour, crowns wider than --max-crown split again.

Everything is in the British National Grid (EPSG:27700): tiles, rasters and the output
coordinates are OS eastings/northings in metres. Nothing is reprojected unless you ask for
--wgs84 when merging.

Commands
  run     fetch + detect for tiles (any grid reference size, or a bounding box). Resumable:
          tiles that already have an output file are skipped, so you can stop and restart.
  merge   combine the per-tile CSVs into GeoParquet files (one per 100 km square, EPSG:27700).

Examples
  python uk_tree_map.py run TL1861 --out data               one 1 km tile
  python uk_tree_map.py run TL16   --out data               a 10 km x 10 km block (100 tiles)
  python uk_tree_map.py run TL     --out data --dry-run      count the tiles in a 100 km square
  python uk_tree_map.py run --bbox 515000 255000 525000 265000 --out data
  python uk_tree_map.py merge --out data
"""
from __future__ import annotations

import argparse
import csv
import json
import math
import os
import re
import sys
import time
import warnings
from collections import defaultdict
from concurrent.futures import ProcessPoolExecutor, as_completed
from dataclasses import asdict, dataclass
from pathlib import Path

warnings.filterwarnings("ignore")

import numpy as np

__version__ = "0.1.0"

BNG = "EPSG:27700"
BASE = "https://environment.data.gov.uk/spatialdata"
VOM_URL = f"{BASE}/vegetation-object-model/wcs"
VOM_COVERAGE = "ecae3bef-1e1d-4051-887b-9dc613c928ec__Vegetation_Object_Model_Elevation_2022"
HEADERS = {"User-Agent": f"uk-tree-map/{__version__} (open tree dataset; github.com/pink-bourbon-estate/toolkits)"}
DATA_SOURCE = ("Contains public sector information licensed under the Open Government Licence v3.0. "
               "Source: Environment Agency, Vegetation Object Model (LiDAR), 2022.")

# Rough British National Grid box around England (metres). Tiles outside it are skipped in batch runs.
ENGLAND_E = (80_000, 660_000)
ENGLAND_N = (0, 660_000)

CSV_FIELDS = ["tree_id", "tile", "os_easting", "os_northing", "crown_diam_m", "crown_area_m2", "height_m", "flag"]


# --------------------------------------------------------------------------
# OS National Grid helpers
# --------------------------------------------------------------------------
def gridref_to_en(ref: str) -> tuple[int, int, int]:
    """'TL', 'TL16', 'TL1861' or 'TL 18500 61500' -> (easting, northing, size_m) of the SW corner.

    size_m is how big the square the reference names is: 100 km for letters only, 10 km for
    2 digits, 1 km for 4 digits, and so on. Letter pair follows the standard OS 100 km grid
    (the letter I is not used).
    """
    ref = re.sub(r"[^A-Za-z0-9]", "", ref).upper()
    m = re.fullmatch(r"([A-HJ-Z])([A-HJ-Z])(\d*)", ref)
    if not m or len(m.group(3)) % 2:
        raise ValueError(f"Not a valid OS grid reference: {ref!r}")
    l1, l2 = ord(m.group(1)) - 65, ord(m.group(2)) - 65
    l1 -= l1 > 7
    l2 -= l2 > 7
    e100 = ((l1 - 2) % 5) * 5 + (l2 % 5)
    n100 = (19 - (l1 // 5) * 5) - (l2 // 5)
    digits = m.group(3)
    h = len(digits) // 2
    e = int(digits[:h]) * 10 ** (5 - h) if h else 0
    n = int(digits[h:]) * 10 ** (5 - h) if h else 0
    return e100 * 100_000 + e, n100 * 100_000 + n, 10 ** (5 - h)


def en_to_tilename(e: int, n: int) -> str:
    """Easting/northing (anywhere in the tile) -> 1 km tile name such as 'TL1861'."""
    e100, n100 = int(e // 100_000), int(n // 100_000)
    l1 = (19 - n100) - (19 - n100) % 5 + (e100 + 10) // 5
    l2 = ((19 - n100) % 5) * 5 + e100 % 5
    l1 += l1 > 7
    l2 += l2 > 7
    return f"{chr(l1 + 65)}{chr(l2 + 65)}{int(e % 100_000) // 1000:02d}{int(n % 100_000) // 1000:02d}"


def tiles_in_bbox(e0: float, n0: float, e1: float, n1: float) -> list[tuple[int, int]]:
    """SW corners of every 1 km tile touching the box [e0,e1) x [n0,n1)."""
    ea, na = int(e0 // 1000) * 1000, int(n0 // 1000) * 1000
    eb, nb = int(math.ceil(e1 / 1000)) * 1000, int(math.ceil(n1 / 1000)) * 1000
    return [(e, n) for n in range(na, nb, 1000) for e in range(ea, eb, 1000)]


def expand_ref(ref: str) -> list[tuple[int, int]]:
    """A grid reference of any size -> the 1 km tiles it covers."""
    e, n, size = gridref_to_en(ref)
    if size <= 1000:
        return [(e // 1000 * 1000, n // 1000 * 1000)]
    return tiles_in_bbox(e, n, e + size, n + size)


def in_england_box(e: int, n: int) -> bool:
    return ENGLAND_E[0] <= e < ENGLAND_E[1] and ENGLAND_N[0] <= n < ENGLAND_N[1]


# --------------------------------------------------------------------------
# Parameters
# --------------------------------------------------------------------------
@dataclass
class Params:
    min_height: float = 2.5      # m, canopy below this is not a tree (the VOM also stops here)
    min_crown: float = 2.5       # m, smallest crown diameter kept
    max_crown: float = 14.0      # m, wider crowns are split
    smooth_m: float = 1.0        # m, treetop smoothing
    prominence: float = 0.15     # treetop must rise this FRACTION of its height above the saddle
    merge_area: float = 25.0     # m2, touching fragments smaller than this are merged
    buffer: int = 20             # m fetched around the tile so edge crowns are not cut
    res: float = 1.0             # m, VOM resolution

    @property
    def min_area_px(self) -> int:
        return max(4, int(round(math.pi * (self.min_crown / 2) ** 2 * 0.5 / self.res ** 2)))

    @property
    def merge_area_px(self) -> int:
        return max(self.min_area_px, int(round(self.merge_area / self.res ** 2)))


class NoDataError(RuntimeError):
    """The service has no LiDAR coverage for this tile (sea, outside England)."""


# --------------------------------------------------------------------------
# LiDAR download
# --------------------------------------------------------------------------
class Grid:
    """The 1 m grid of one tile plus its buffer, in British National Grid."""

    def __init__(self, e0: int, n0: int, buf: int, res: float = 1.0):
        from rasterio.transform import Affine
        self.e0, self.n0, self.buf, self.res = e0, n0, buf, res
        self.n = int(round((1000 + 2 * buf) / res))
        self.tr = Affine(res, 0, e0 - buf, 0, -res, n0 + 1000 + buf)
        self.bounds = (e0 - buf, n0 - buf, e0 + 1000 + buf, n0 + 1000 + buf)


def read_raster(data_or_path) -> tuple[np.ndarray, object]:
    """-> (float32 array with NaN for no-data, affine transform)."""
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
    arr[(arr < -50) | (arr > 150)] = np.nan  # unflagged nodata guard
    return arr, tr


def align_to_grid(arr: np.ndarray, src_tr, grid: Grid) -> np.ndarray:
    """Place a BNG raster onto the tile's master grid (nearest neighbour; any offset or size)."""
    from rasterio.warp import Resampling, reproject
    dst = np.full((grid.n, grid.n), np.nan, np.float32)
    reproject(arr, dst, src_transform=src_tr, src_crs=BNG, dst_transform=grid.tr, dst_crs=BNG,
              resampling=Resampling.nearest, src_nodata=np.nan, dst_nodata=np.nan)
    return dst


_TIFF_MAGIC = (b"II*\x00", b"MM\x00*", b"II+\x00", b"MM\x00+")


def fetch_vom_tiff(bounds, session=None, attempts: int = 4) -> bytes:
    """Download the VOM for a BNG box from the Environment Agency WCS. Retries with back-off."""
    import requests
    session = session or requests.Session()
    e0, n0, e1, n1 = bounds
    params = [("service", "WCS"), ("version", "2.0.1"), ("request", "GetCoverage"),
              ("coverageId", VOM_COVERAGE), ("format", "image/tiff"),
              ("subset", f"E({e0},{e1})"), ("subset", f"N({n0},{n1})")]
    last = "unknown error"
    for attempt in range(attempts):
        try:
            r = session.get(VOM_URL, params=params, headers=HEADERS, timeout=180)
            body = r.content
            if r.status_code == 200 and body[:4] in _TIFF_MAGIC:
                return body
            text = body[:600].decode("utf-8", "replace")
            last = f"WCS {r.status_code}: {text}"
            # An OGC exception about the subset means there is no coverage here: do not retry.
            if r.status_code in (400, 404) and re.search(r"subset|outside|extent|intersect", text, re.I):
                raise NoDataError(last)
        except NoDataError:
            raise
        except requests.RequestException as ex:
            last = f"Could not reach the Environment Agency WCS: {str(ex)[:160]}"
        time.sleep(2 ** attempt)
    raise RuntimeError(last)


def get_vom(grid: Grid, tile: str, cache_dir: Path | None, override: str | None) -> np.ndarray:
    """Canopy height (m) on the tile's master grid; NaN where there is no data."""
    if override:
        arr, tr = read_raster(override)
        return align_to_grid(arr, tr, grid)
    path = cache_dir / f"{tile}_vom_b{grid.buf}.tif" if cache_dir else None
    if path and path.exists():
        arr, tr = read_raster(str(path))
        return align_to_grid(arr, tr, grid)
    body = fetch_vom_tiff(grid.bounds)
    if path:
        cache_dir.mkdir(parents=True, exist_ok=True)
        path.write_bytes(body)
    arr, tr = read_raster(body)
    return align_to_grid(arr, tr, grid)


# --------------------------------------------------------------------------
# Crown detection
# --------------------------------------------------------------------------
def _log_int(sm: np.ndarray) -> np.ndarray:
    return np.round(np.log(np.maximum(sm, 1.0)) * 1000).astype(np.int32)


def _smooth_masked(chm, mask, sigma, floor):
    """Gaussian smoothing that does not bleed the zero background into crown edges."""
    from scipy import ndimage as ndi
    num = ndi.gaussian_filter(np.where(mask, np.maximum(chm, floor), 0).astype(np.float32), sigma)
    den = np.maximum(ndi.gaussian_filter(mask.astype(np.float32), sigma), 1e-3)
    return np.where(mask, num / den, 0).astype(np.float32)


def _remove_small(mask, min_px):
    """Drop connected blobs smaller than min_px pixels (works across scikit-image versions)."""
    from skimage import measure
    lab = measure.label(mask, connectivity=1)
    cnt = np.bincount(lab.ravel())
    keep = cnt >= min_px
    keep[0] = False
    return keep[lab]


def _merge_small(lab, sm, mask, min_area_px):
    """Absorb watershed fragments smaller than min_area_px into the neighbour they touch."""
    from skimage import segmentation
    cnt = np.bincount(lab.ravel())
    small = np.nonzero((cnt < min_area_px) & (np.arange(len(cnt)) > 0) & (cnt > 0))[0]
    if len(small) == 0:
        return lab
    keep = np.where(np.isin(lab, small), 0, lab)
    out = segmentation.watershed(-sm, keep, mask=mask)
    return np.where((out == 0) & (lab > 0), lab, out)


def _split_big(lab, sm_fine, p: Params):
    """Split crowns wider than max_crown: finer watershed first, k-means as a fallback."""
    from scipy.cluster.vq import kmeans2
    from skimage import measure, morphology, segmentation
    mpp = p.res
    max_area = math.pi * (p.max_crown / 2) ** 2 / mpp ** 2
    tgt_area = math.pi * (0.6 * p.max_crown / 2) ** 2 / mpp ** 2
    out = lab.copy()
    nxt = int(lab.max()) + 1
    was_split: set[int] = set()
    hf = max(1, int(round(p.prominence * 1000 / 3)))
    for reg in measure.regionprops(lab):
        if reg.area <= max_area:
            continue
        r0, c0, r1, c1 = reg.bbox
        m = reg.image
        fine = sm_fine[r0:r1, c0:c1]
        parts = np.where(m, 1, 0)
        mk = measure.label(morphology.h_maxima(np.where(m, _log_int(fine), 0), hf).astype(np.uint8) & m)
        if mk.max() >= 2:
            ws = _merge_small(segmentation.watershed(-fine, mk, mask=m), fine, m, p.merge_area_px)
            if len(np.unique(ws[m])) >= 2:
                parts = ws
        for pid in np.unique(parts[m]):
            ys, xs = np.nonzero(parts == pid)
            if len(ys) > max_area:
                k = max(2, int(round(len(ys) / tgt_area)))
                _, cl = kmeans2(np.column_stack([ys, xs]).astype(float), k, minit="++", seed=0, missing="warn")
                for ci in range(k):
                    sel = cl == ci
                    if sel.any():
                        out[r0 + ys[sel], c0 + xs[sel]] = nxt
                        was_split.add(nxt)
                        nxt += 1
            else:
                out[r0 + ys, c0 + xs] = nxt
                was_split.add(nxt)
                nxt += 1
    return out, was_split


def detect_crowns(chm: np.ndarray, p: Params) -> list[dict]:
    """Canopy height model (m, 0 = no canopy) -> one dict per crown, in pixel coordinates."""
    from scipy import ndimage as ndi
    from skimage import measure, morphology, segmentation
    mpp = p.res
    mask = chm >= p.min_height
    mask = ndi.binary_closing(mask, np.ones((3, 3), bool))
    mask = ndi.binary_fill_holes(mask)
    mask = _remove_small(mask, p.min_area_px)
    if not mask.any():
        return []
    sm = _smooth_masked(chm, mask, max(0.5, p.smooth_m / mpp), p.min_height)
    h = max(1, int(round(p.prominence * 1000)))  # relative prominence on log-height
    markers = measure.label(morphology.h_maxima(_log_int(sm), h).astype(np.uint8) & mask)
    lab = segmentation.watershed(-sm, markers, mask=mask)
    lab = _merge_small(lab, sm, mask, p.merge_area_px)
    sm_fine = _smooth_masked(chm, mask, max(0.5, 0.7 / mpp), p.min_height)
    lab, was_split = _split_big(lab, sm_fine, p)

    max_r_px = (p.max_crown / mpp) / 2
    min_r_px = (p.min_crown / mpp) / 2
    out = []
    for reg in measure.regionprops(lab, intensity_image=chm):
        diam_px = 2 * math.sqrt(reg.area / math.pi)
        if diam_px / 2 < min_r_px * 0.7:
            continue
        cy, cx = reg.centroid
        out.append({"row": float(cy), "col": float(cx), "area_m2": reg.area * mpp * mpp,
                    "diam_m": diam_px * mpp, "height_m": float(reg.intensity_max),
                    "split": reg.label in was_split, "big": diam_px / 2 > max_r_px})
    return out


# --------------------------------------------------------------------------
# One tile
# --------------------------------------------------------------------------
def tile_csv_path(out_dir: Path, tile: str) -> Path:
    return out_dir / "tiles" / f"{tile}.csv"


def _write_tile_csv(path: Path, rows: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_suffix(".tmp")
    with open(tmp, "w", newline="") as fh:
        wr = csv.DictWriter(fh, CSV_FIELDS)
        wr.writeheader()
        wr.writerows(rows)
    os.replace(tmp, path)  # atomic: a half-written tile never looks finished


def process_tile(e0: int, n0: int, out_dir: str, p: Params, cache_dir: str | None = None,
                 vom_override: str | None = None) -> dict:
    """Fetch the VOM for one 1 km tile, detect crowns, write tiles/<TILE>.csv. Never raises."""
    tile = en_to_tilename(e0, n0)
    out = Path(out_dir)
    final = tile_csv_path(out, tile)
    if final.exists():
        return {"tile": tile, "status": "skipped", "trees": None}
    t0 = time.time()
    try:
        grid = Grid(e0, n0, p.buffer, p.res)
        vom = get_vom(grid, tile, Path(cache_dir) if cache_dir else None, vom_override)
        if not np.isfinite(vom).any() or np.nanmax(vom) <= 0:
            _write_tile_csv(final, [])
            return {"tile": tile, "status": "empty", "trees": 0, "secs": time.time() - t0}
        chm = np.where(np.nan_to_num(vom, nan=0.0) >= p.min_height, np.nan_to_num(vom, nan=0.0), 0.0)
        rows = []
        for t in detect_crowns(chm.astype(np.float32), p):
            x, y = grid.tr @ (t["col"] + 0.5, t["row"] + 0.5)
            if not (e0 <= x < e0 + 1000 and n0 <= y < n0 + 1000):
                continue  # belongs to the neighbouring tile
            flags = [s for s, on in (("oversized", t["big"]), ("split", t["split"])) if on]
            rows.append({"tree_id": f"{tile}-{len(rows) + 1}", "tile": tile,
                         "os_easting": f"{x:.2f}", "os_northing": f"{y:.2f}",
                         "crown_diam_m": f"{t['diam_m']:.2f}", "crown_area_m2": f"{t['area_m2']:.1f}",
                         "height_m": f"{t['height_m']:.1f}", "flag": ";".join(flags)})
        _write_tile_csv(final, rows)
        return {"tile": tile, "status": "done", "trees": len(rows), "secs": time.time() - t0}
    except NoDataError as ex:
        _write_tile_csv(final, [])  # remember it, so a re-run does not ask again
        return {"tile": tile, "status": "nodata", "trees": 0, "error": str(ex)[:200]}
    except Exception as ex:  # noqa: BLE001 - one bad tile must not stop a national run
        return {"tile": tile, "status": "failed", "trees": None, "error": f"{type(ex).__name__}: {str(ex)[:300]}"}


# --------------------------------------------------------------------------
# Commands
# --------------------------------------------------------------------------
def collect_tiles(a) -> list[tuple[int, int]]:
    seen: dict[tuple[int, int], None] = {}
    for ref in a.refs:
        for t in expand_ref(ref):
            seen[t] = None
    if a.bbox:
        for t in tiles_in_bbox(*a.bbox):
            seen[t] = None
    tiles = [t for t in seen if in_england_box(*t)]
    skipped = len(seen) - len(tiles)
    if skipped:
        print(f"Skipping {skipped} tiles outside the England grid box (the Environment Agency LiDAR covers England only).")
    return tiles


def cmd_run(a) -> int:
    if not a.refs and not a.bbox:
        sys.exit("Give at least one grid reference (e.g. TL1861, TL16, TL) or --bbox E0 N0 E1 N1.")
    out = Path(a.out)
    tiles = collect_tiles(a)
    if not tiles:
        sys.exit("No tiles to process.")
    done = sum(1 for e, n in tiles if tile_csv_path(out, en_to_tilename(e, n)).exists())
    todo = [t for t in tiles if not tile_csv_path(out, en_to_tilename(*t)).exists()]
    if a.limit:
        todo = todo[:a.limit]
    print(f"{len(tiles)} tiles requested, {done} already done, {len(todo)} to process this run.")
    if a.dry_run or not todo:
        return 0

    p = Params(min_height=a.min_height, min_crown=a.min_crown, max_crown=a.max_crown,
               smooth_m=a.smooth, prominence=a.prominence, merge_area=a.merge_area, buffer=a.buffer)
    out.mkdir(parents=True, exist_ok=True)
    (out / "run.json").write_text(json.dumps({
        "tool": "uk_tree_map.py", "version": __version__, "crs": BNG, "params": asdict(p),
        "coverage": VOM_COVERAGE, "licence": DATA_SOURCE}, indent=2))
    failed_path = out / "failed.txt"
    cache = a.cache_dir

    counts: dict[str, int] = defaultdict(int)
    n_trees = 0
    failures: list[str] = []
    t_start = time.time()
    with ProcessPoolExecutor(max_workers=a.workers) as ex:
        futs = [ex.submit(process_tile, e, n, str(out), p, cache, a.vom_tif) for e, n in todo]
        for i, fut in enumerate(as_completed(futs), 1):
            r = fut.result()
            counts[r["status"]] += 1
            n_trees += r.get("trees") or 0
            if r["status"] == "failed":
                failures.append(f"{r['tile']}\t{r['error']}")
            rate = (time.time() - t_start) / i
            msg = f"[{i}/{len(todo)}] {r['tile']} {r['status']}"
            if r.get("trees") is not None:
                msg += f" {r['trees']} trees"
            if r.get("error"):
                msg += f"  ({r['error'][:120]})"
            print(f"{msg}  ~{rate * (len(todo) - i) / 60:.0f} min left", flush=True)
    if failures:
        failed_path.write_text("\n".join(failures) + "\n")
    elif failed_path.exists():
        failed_path.unlink()
    print(f"\nFinished: {dict(counts)}; {n_trees:,} trees this run.")
    if failures:
        print(f"{len(failures)} tiles failed (see {failed_path}). Re-run the same command to retry just those.")
    return 1 if failures else 0


def cmd_merge(a) -> int:
    import geopandas as gpd
    import pandas as pd
    out = Path(a.out)
    files = sorted((out / "tiles").glob("*.csv"))
    if not files:
        sys.exit(f"No tile files in {out / 'tiles'}. Run the 'run' command first.")
    squares: dict[str, list[Path]] = defaultdict(list)
    for f in files:
        squares[f.stem[:2]].append(f)
    dest = out / "trees"
    dest.mkdir(parents=True, exist_ok=True)
    total = 0
    for sq, fs in sorted(squares.items()):
        frames = [pd.read_csv(f, dtype={"flag": str}, keep_default_na=False) for f in fs]
        df = pd.concat([x for x in frames if len(x)], ignore_index=True) if any(len(x) for x in frames) else None
        if df is None:
            continue
        gdf = gpd.GeoDataFrame(df, geometry=gpd.points_from_xy(df.os_easting, df.os_northing), crs=BNG)
        gdf.to_parquet(dest / f"trees_{sq}_bng.parquet")
        if a.wgs84:
            gdf.to_crs("EPSG:4326").to_parquet(dest / f"trees_{sq}_wgs84.parquet")
        total += len(gdf)
        print(f"{sq}: {len(gdf):,} trees from {len(fs)} tiles")
    print(f"Wrote {total:,} trees to {dest}")
    return 0


def build_parser() -> argparse.ArgumentParser:
    ap = argparse.ArgumentParser(description="Open tree-crown map of England from free LiDAR (British National Grid).")
    sub = ap.add_subparsers(dest="cmd", required=True)

    r = sub.add_parser("run", help="fetch LiDAR and detect tree crowns (resumable)")
    r.add_argument("refs", nargs="*", help="OS grid references: TL (100 km), TL16 (10 km), TL1861 (1 km)")
    r.add_argument("--bbox", nargs=4, type=float, metavar=("E0", "N0", "E1", "N1"), help="box in OS eastings/northings (m)")
    r.add_argument("--out", default="data", help="output folder (default: data)")
    r.add_argument("--workers", type=int, default=4, help="parallel tiles (be kind to the server; default 4)")
    r.add_argument("--limit", type=int, default=0, help="only do the first N pending tiles (for timing a pilot)")
    r.add_argument("--dry-run", action="store_true", help="count the tiles and stop")
    r.add_argument("--cache-dir", default=None, help="keep downloaded VOM GeoTIFFs here (about 4 MB a tile; off by default)")
    r.add_argument("--vom-tif", default=None, help="use a local VOM GeoTIFF (BNG) instead of downloading")
    g = r.add_argument_group("detection tuning")
    g.add_argument("--min-height", type=float, default=2.5)
    g.add_argument("--min-crown", type=float, default=2.5)
    g.add_argument("--max-crown", type=float, default=14.0, help="wider crowns are split")
    g.add_argument("--smooth", type=float, default=1.0, help="treetop smoothing, metres")
    g.add_argument("--prominence", type=float, default=0.15, help="fraction of height a treetop must rise above its saddle")
    g.add_argument("--merge-area", type=float, default=25.0, help="touching fragments smaller than this (m2) are merged")
    g.add_argument("--buffer", type=int, default=20, help="metres fetched around the tile")
    r.set_defaults(func=cmd_run)

    m = sub.add_parser("merge", help="combine tile CSVs into GeoParquet (one file per 100 km square)")
    m.add_argument("--out", default="data")
    m.add_argument("--wgs84", action="store_true", help="also write a WGS84 (EPSG:4326) copy for web maps")
    m.set_defaults(func=cmd_merge)
    return ap


def main(argv=None) -> int:
    a = build_parser().parse_args(argv)
    return a.func(a)


if __name__ == "__main__":
    sys.exit(main())
