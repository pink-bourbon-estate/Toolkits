# Tree finder

Finds tree crowns, hedges and shrubs for one 1 km Ordnance Survey grid tile from open
Environment Agency LiDAR plus satellite imagery, and writes CSVs that the Blender scripts
turn into empties and curves.

The example area is central London (Soho), tile **TQ2981**:

    python find_trees_v0.12.py TQ2981 --out out

You can also give a Google Maps URL instead of a grid reference. `python find_trees_v0.12.py -h`
lists every option (detection tuning, hedges, caching).

Blender: open `place_empties.py` or `place_hedges.py` in the Scripting tab, point the CSV path
at the files written to `out/`, and run.

Needs: Python 3.8+, `pip install numpy scipy scikit-image pillow requests`.

## Honest limits
- Written and tuned on a different area. I have not re-checked the data endpoints or the
  results for central London, so treat the Soho default as an untested starting point.
- Dense city blocks have few trees, so expect small counts and some false positives on green roofs.
- Satellite imagery is not perfectly orthorectified; tall trees lean, so positions can be off by a few metres.
- Needs network access to the public data services, which can change.
- Data downloaded into `_cache/` is not part of this repo.
