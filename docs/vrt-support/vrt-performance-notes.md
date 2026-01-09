# VRT Performance Notes (Release)

## Test Date
2026-01-07

## Environment
- Host: Linux 6.8.0-90-generic
- Build: `cargo build -p whitebox_tools --release`
- Tool: `RasterSummaryStats` (reads full raster into memory)

## Dataset
- Source: `/wc1/culverts/8121e6c0-50ff-4777-b61f-f743058f0fe4/topo/hydro-enforced-dem.tif`
- Dimensions: 8910 x 5510 (from `PrintGeoTiffTags`)
- VRT window: xOff=2000, yOff=2000, xSize=1000, ySize=1000
- VRT file: `/tmp/weppcloud-wbt-perf/hydro_enforced_1000x1000.vrt`

VRT contents used:
```xml
<VRTDataset rasterXSize="1000" rasterYSize="1000">
  <VRTRasterBand band="1">
    <SimpleSource>
      <SourceFilename relativeToVRT="0">/wc1/culverts/8121e6c0-50ff-4777-b61f-f743058f0fe4/topo/hydro-enforced-dem.tif</SourceFilename>
      <SourceBand>1</SourceBand>
      <SrcRect xOff="2000" yOff="2000" xSize="1000" ySize="1000" />
      <DstRect xOff="0" yOff="0" xSize="1000" ySize="1000" />
    </SimpleSource>
  </VRTRasterBand>
</VRTDataset>
```

## Commands
```bash
mkdir -p /tmp/weppcloud-wbt-perf

# Optional: verify dimensions
target/release/whitebox_tools -r=PrintGeoTiffTags -v --wd="/tmp/weppcloud-wbt-perf" \
  --input="/wc1/culverts/8121e6c0-50ff-4777-b61f-f743058f0fe4/topo/hydro-enforced-dem.tif"

# Full GeoTIFF
/usr/bin/time -v target/release/whitebox_tools -r=RasterSummaryStats -v \
  --wd="/tmp/weppcloud-wbt-perf" \
  -i="/wc1/culverts/8121e6c0-50ff-4777-b61f-f743058f0fe4/topo/hydro-enforced-dem.tif"

# VRT crop (1000x1000 at offset 2000,2000)
/usr/bin/time -v target/release/whitebox_tools -r=RasterSummaryStats -v \
  --wd="/tmp/weppcloud-wbt-perf" \
  -i="/tmp/weppcloud-wbt-perf/hydro_enforced_1000x1000.vrt"
```

## Results (single run, warm cache)

| Input | Elapsed (s) | Max RSS (KB) |
|------|-------------|--------------|
| Full GeoTIFF | 1.75 | 774528 |
| VRT 1000x1000 | 0.10 | 23040 |

Notes:
- `/usr/bin/time -v` reported `File system inputs: 0` for both runs, indicating OS cache was warm.
- Max RSS dropped from ~757 MB to ~23 MB for the windowed VRT read.
- The VRT used a minimal XML (no SRS/GeoTransform) to avoid copying large WKT strings.
