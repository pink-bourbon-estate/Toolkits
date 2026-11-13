import csv
import sys
from pathlib import Path

import numpy as np
import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))
import uk_tree_map as u  # noqa: E402


# ---- National Grid maths ---------------------------------------------------
@pytest.mark.parametrize("ref,expected", [
    ("SV", (0, 0, 100_000)),
    ("SU", (400_000, 100_000, 100_000)),
    ("TQ", (500_000, 100_000, 100_000)),
    ("TL", (500_000, 200_000, 100_000)),
    ("NT", (300_000, 600_000, 100_000)),
    ("HP", (400_000, 1_200_000, 100_000)),
    ("TL16", (510_000, 260_000, 10_000)),
    ("TL1861", (518_000, 261_000, 1_000)),
    ("TL 18500 61500", (518_500, 261_500, 1)),
])
def test_gridref_to_en(ref, expected):
    assert u.gridref_to_en(ref) == expected


@pytest.mark.parametrize("bad", ["", "T", "TL1", "TL186", "II1861", "1234", "TL18x"])
def test_bad_gridrefs(bad):
    with pytest.raises(ValueError):
        u.gridref_to_en(bad)


def test_tilename_roundtrip_every_real_100km_square():
    # the National Grid covers 7 x 13 squares of 100 km (SV in the SW to HP in the NE)
    seen = set()
    for n100 in range(13):
        for e100 in range(7):
            e, n = e100 * 100_000, n100 * 100_000
            for de, dn in ((0, 0), (37_000, 81_000), (99_000, 99_000)):
                name = u.en_to_tilename(e + de, n + dn)
                assert name[2:] == f"{de // 1000:02d}{dn // 1000:02d}"
                assert u.gridref_to_en(name)[:2] == (e + de, n + dn)
            seen.add(u.en_to_tilename(e, n)[:2])
    assert len(seen) == 91 and "I" not in "".join(seen)


def test_expand_ref_sizes():
    assert u.expand_ref("TL1861") == [(518_000, 261_000)]
    assert len(u.expand_ref("TL16")) == 100
    assert len(u.expand_ref("TL")) == 10_000
    assert u.expand_ref("TL 18500 61500") == [(518_000, 261_000)]  # sub-tile ref -> containing tile


def test_bbox_tiles_touch_the_box():
    t = u.tiles_in_bbox(518_500, 261_500, 520_500, 262_500)
    assert (518_000, 261_000) in t and (520_000, 262_000) in t
    assert len(t) == 3 * 2


def test_england_box():
    assert u.in_england_box(518_000, 261_000)
    assert not u.in_england_box(300_000, 700_000)  # Scotland


# ---- Detection on a synthetic canopy ---------------------------------------
def _canopy(n=1040, trees=()):
    yy, xx = np.mgrid[0:n, 0:n].astype(np.float32)
    chm = np.zeros((n, n), np.float32)
    for r, c, h, rad in trees:
        d2 = (yy - r) ** 2 + (xx - c) ** 2
        chm = np.maximum(chm, np.where(d2 < rad ** 2, h * (1 - 0.35 * d2 / rad ** 2), 0))
    chm[chm < 2.5] = 0
    return chm


def test_detects_separate_trees_with_height():
    p = u.Params()
    chm = _canopy(trees=[(200, 200, 12, 4), (200, 240, 18, 5), (600, 600, 9, 3.5)])
    found = u.detect_crowns(chm, p)
    assert len(found) == 3
    assert sorted(round(t["height_m"]) for t in found) == [9, 12, 18]
    assert not any(t["big"] for t in found)


def test_empty_canopy_gives_no_trees():
    assert u.detect_crowns(np.zeros((300, 300), np.float32), u.Params()) == []


def test_oversized_crown_is_split():
    p = u.Params()
    chm = np.zeros((400, 400), np.float32)
    chm[100:140, 100:300] = 14  # a 40 m x 200 m flat wall of canopy
    found = u.detect_crowns(chm, p)
    assert len(found) > 1
    assert all(t["diam_m"] <= p.max_crown * 1.6 for t in found)


# ---- One tile end to end, offline ------------------------------------------
def _write_bng_tiff(path, e0, n0, buf, chm):
    import rasterio
    from rasterio.transform import Affine
    tr = Affine(1, 0, e0 - buf, 0, -1, n0 + 1000 + buf)
    with rasterio.open(path, "w", driver="GTiff", height=chm.shape[0], width=chm.shape[1], count=1,
                       dtype="float32", crs=u.BNG, transform=tr, nodata=-9999) as ds:
        ds.write(chm, 1)


def test_process_tile_writes_bng_coordinates_and_resumes(tmp_path):
    e0, n0, buf = 518_000, 261_000, 20
    # one tree 100 m east, 200 m north of the tile's SW corner (pixel row counts down from the top)
    chm = _canopy(trees=[(buf + 1000 - 200, buf + 100, 15, 4)])
    tif = tmp_path / "vom.tif"
    _write_bng_tiff(tif, e0, n0, buf, chm)

    r = u.process_tile(e0, n0, str(tmp_path / "out"), u.Params(), vom_override=str(tif))
    assert r["status"] == "done" and r["trees"] == 1
    rows = list(csv.DictReader(open(tmp_path / "out" / "tiles" / "TL1861.csv")))
    assert rows[0]["tile"] == "TL1861"
    assert float(rows[0]["os_easting"]) == pytest.approx(e0 + 100.5, abs=1.5)
    assert float(rows[0]["os_northing"]) == pytest.approx(n0 + 200 - 0.5, abs=1.5)
    assert float(rows[0]["height_m"]) == pytest.approx(15, abs=0.1)

    again = u.process_tile(e0, n0, str(tmp_path / "out"), u.Params(), vom_override=str(tif))
    assert again["status"] == "skipped"


def test_trees_outside_the_tile_belong_to_the_neighbour(tmp_path):
    e0, n0, buf = 518_000, 261_000, 20
    chm = _canopy(trees=[(buf + 500, buf - 10, 14, 4)])  # 10 m west of the tile, inside the buffer
    tif = tmp_path / "vom.tif"
    _write_bng_tiff(tif, e0, n0, buf, chm)
    r = u.process_tile(e0, n0, str(tmp_path / "out"), u.Params(), vom_override=str(tif))
    assert r["trees"] == 0


def test_failed_tile_is_not_marked_done(tmp_path):
    r = u.process_tile(518_000, 261_000, str(tmp_path / "out"), u.Params(), vom_override=str(tmp_path / "missing.tif"))
    assert r["status"] == "failed"
    assert not (tmp_path / "out" / "tiles" / "TL1861.csv").exists()


def test_merge_writes_bng_geoparquet(tmp_path):
    gpd = pytest.importorskip("geopandas")
    e0, n0, buf = 518_000, 261_000, 20
    tif = tmp_path / "vom.tif"
    _write_bng_tiff(tif, e0, n0, buf, _canopy(trees=[(500, 500, 15, 4)]))
    out = tmp_path / "out"
    u.process_tile(e0, n0, str(out), u.Params(), vom_override=str(tif))
    assert u.main(["merge", "--out", str(out), "--wgs84"]) == 0
    bng = gpd.read_parquet(out / "trees" / "trees_TL_bng.parquet")
    assert bng.crs.to_epsg() == 27700 and len(bng) == 1
    wgs = gpd.read_parquet(out / "trees" / "trees_TL_wgs84.parquet")
    assert wgs.crs.to_epsg() == 4326
    assert 52 < wgs.geometry.y.iloc[0] < 53 and -1 < wgs.geometry.x.iloc[0] < 0.5
