# UK Tree Map

An open, free tree-crown map of England, built from the Environment Agency's open LiDAR. It finds individual tree crowns, 1 km tile by 1 km tile, and writes them out as points (position, crown width, crown area, height). Everything is in the **British National Grid (EPSG:27700)**: tiles, rasters and output coordinates are OS eastings and northings in metres.

It is not a replacement for commercial products made from aerial photography and survey-grade processing. It is a free, reproducible baseline anyone can run, inspect and improve.

## Status

Version 0.1. The grid maths, crown detection, resume logic and merge step are tested offline with synthetic canopy rasters (`pytest tests`). **It has not yet been run against the live Environment Agency service**: the first real run is the pilot below. Expect to tune `--prominence` and `--max-crown` against what you see.

## Install

```
python -m venv .venv
.venv\Scripts\activate          # Windows
source .venv/bin/activate       # Linux / macOS
pip install -r requirements.txt
```

## Use

Grid references can be any size: `TL` is a 100 km square, `TL16` a 10 km block (100 tiles), `TL1861` one 1 km tile.

```
python uk_tree_map.py run TL1861 --out data            # one tile: try this first
python uk_tree_map.py run TL16 --out data --limit 5    # time five tiles of a 10 km block
python uk_tree_map.py run TL16 --out data              # the whole block
python uk_tree_map.py run TL --out data --dry-run      # just count the tiles
python uk_tree_map.py run --bbox 515000 255000 525000 265000 --out data
python uk_tree_map.py merge --out data --wgs84         # GeoParquet per 100 km square
```

- **Resumable.** Each finished tile writes `data/tiles/<TILE>.csv`. Re-running the same command skips finished tiles and retries failed ones (listed in `data/failed.txt`). Stop and restart whenever you like.
- **Polite.** 4 tiles at a time by default (`--workers`). Don't turn this up for a national run; it is someone else's server.
- **Output.** `tiles/*.csv` per tile, then `trees/trees_<SQUARE>_bng.parquet` (EPSG:27700) from `merge`. `--wgs84` also writes an EPSG:4326 copy for web maps. `run.json` records the version, parameters and data licence of the run.

| Column | Meaning |
| --- | --- |
| `tree_id` | tile name and running number, e.g. `TL1861-37` |
| `os_easting`, `os_northing` | crown centre, British National Grid, metres |
| `crown_diam_m`, `crown_area_m2` | crown size from the canopy segment |
| `height_m` | tallest point of the crown |
| `flag` | `oversized` (wider than `--max-crown`) and/or `split` (broken up from a larger blob) |

## How it works

1. **Fetch** the Vegetation Object Model (canopy height at 1 m) for the tile plus a 20 m buffer.
2. **Treetops** are local maxima of the smoothed log-height surface. Prominence is relative to tree height, so tall woodland trees are not over-split.
3. **Crowns** are a watershed from those treetops. Tiny fragments merge into a neighbour; crowns wider than `--max-crown` are split again.
4. Only trees whose centre falls inside the tile are written, so neighbouring tiles never double count.

## Known limits

- **England only.** The Environment Agency LiDAR does not cover Scotland, Wales or Northern Ireland. Those need their own sources (Scotland: the Scottish Remote Sensing Portal) and are future work.
- **LiDAR only.** An earlier single-tile script also used satellite imagery to rescue trees the vegetation model missed and to find hedges. That imagery (Esri World Imagery) is not licensed for bulk download or redistribution of derived data, so it is left out. Garden trees the model classed as buildings or ground will be missed.
- **Crown boundaries in dense woodland are approximate.** Individual trees in closed canopy are the hard case for any LiDAR method.
- **Date.** The model is a 2022 composite; the real survey date varies by area.

## Licence and credit

Code: MIT (see the repository's `LICENSE-MIT`).

Data: *Contains public sector information licensed under the Open Government Licence v3.0. Source: Environment Agency, Vegetation Object Model (LiDAR).* Anything published from this tool should carry that line.

## Tests

```
pip install pytest
python -m pytest tests
```
