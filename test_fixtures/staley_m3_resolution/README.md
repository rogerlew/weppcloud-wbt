# Staley M3 paired resolution fixtures

Six user-supplied canonical WEPPcloud runs form the primary test panel for
WEPPpy work package `20260908_staley_m3_wbt_terrain`. Snapshots preserve source
bytes, grid extents, routing, and outlet records without resampling.

| Site | 30 m | 10 m |
| --- | --- | --- |
| Moscow Mountain | bass-elimination | desolate-yea |
| Topanga | untucked-hit | sorrowful-semicircle |
| AZ ponderosa (user label) | offshore-remake | full-crocodile |

Under each `<site>/<resolution>/`, source-relative paths contain:

- `dem/dem.tif` and `.meta`: original DEM and source/retrieval provenance.
- `dem/wbt/relief.tif` and `relief.diagnostics.json`: conditioned elevation
  DEM and conditioning diagnostics. This is **not** Staley vertical relief.
- `dem/wbt/flovec.tif`: existing WBT D8 pointer (WBT encoding).
- `dem/wbt/subwta.tif`: incremental subcatchment identifiers.
- `dem/wbt/netful.tif`: existing channel network.
- `dem/wbt/bound.tif`, `bound.geojson`, `bound.WGS.geojson`: raster boundary,
  projected vector boundary, and WGS84 vector boundary maps.
- `dem/wbt/outlet.geojson`: requested and snapped outlet metadata.

The user identifies each pair as the same watershed with identically specified
outlets through canonical 10 m/30 m workflows. Stored requested coordinates
nevertheless show small differences, and snapped cell centers differ. Preserve
and quantify both; do not assume exact coordinate equality. Extents differ.
Do not crop to the rectangle intersection and risk removing upstream terrain.
Use full contributing catchments, not incremental subcatchment labels alone.

`manifest.json` contains source URLs, local source paths, capture time, file
sizes/hashes, raster metadata, and outlet records. Source DEM sidecars identify
USGS datasets. No paper, GPL reference code, or project NoDb state is included.
This is a canonical-workflow comparison panel; controlled 10-to-30 m resampling
is a separate experiment. Scientific suitability and routing coverage still
require assessment. The AZ name is supplied by the user, not verified geography.

From the repository root:

```bash
python test_fixtures/staley_m3_resolution/verify.py
```

This verifies snapshot integrity only. Write generated study results outside
the fixture directories. TIFFs are ordinary Git files: GitHub rejected new
LFS objects on this public fork. The bounded bundle is about 52 MB, with no
individual raster larger than 12 MB.
