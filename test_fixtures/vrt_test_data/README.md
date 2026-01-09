# VRT Support Test Data

Test fixtures for developing and validating VRT (Virtual Raster) support in weppcloud-wbt.
See `docs/vrt-support-spec.md` for the implementation specification.

## Directory Structure

```
vrt_test_data/
├── source/           # Source GeoTIFF files (crop targets)
├── vrt/              # Valid VRT files for testing
├── reference/        # Reference cropped TIFFs for value comparison
├── invalid_vrt/      # Invalid VRT files for error handling tests
└── README.md
```

## Source Files

All source files were created from a real DEM from the culvert-at-risk integration testing:
- Original: `/wc1/culverts/8121e6c0-50ff-4777-b61f-f743058f0fe4/topo/hydro-enforced-dem.tif`
- 8910x5510 pixels, 1m resolution, EPSG:32619 (UTM zone 19N), Float64

### source/dem_500x500.tif
- **Size**: 500x500 pixels
- **Type**: Float64
- **Layout**: Stripped (500x2 blocks)
- **Compression**: None
- **Creation**: `gdal_translate -srcwin 1000 1000 500 500`

### source/dem_500x500_lzw.tif
- Same as above with LZW compression
- **Creation**: `gdal_translate -co COMPRESS=LZW`

### source/dem_500x500_deflate.tif
- Same as above with DEFLATE compression
- **Creation**: `gdal_translate -co COMPRESS=DEFLATE`

### source/dem_500x500_tiled.tif
- Same data but with 64x64 tile layout
- **Creation**: `gdal_translate -co TILED=YES -co BLOCKXSIZE=64 -co BLOCKYSIZE=64`

### source/dem_500x500_tiled_lzw.tif
- Tiled (64x64) with LZW compression
- **Creation**: Combined TILED=YES and COMPRESS=LZW

### source/dem_100x100_int16.tif
- 100x100 crop, Int16 data type
- For testing different numeric types

### source/dem_100x100_int16_lzw_pred2.tif
- 100x100 crop, Int16 data type
- LZW compression with PREDICTOR=2 (horizontal differencing)

### source/dem_100x100_int16_packbits.tif
- 100x100 crop, Int16 data type
- PACKBITS compression

### source/dem_100x100_float32.tif
- 100x100 crop, Float32 data type
- For testing different numeric types

### source/dem_500x500_sparse.tif
- 500x500, Float64 data type
- Tiled (64x64), SPARSE_OK=YES, NoData=0
- All pixels set to 0 to exercise sparse-tile handling

## VRT Files (Valid)

Each VRT was created with `gdal_translate -of VRT -srcwin ...` and has a corresponding
reference TIF in `reference/` created with `gdal_translate -of GTiff -srcwin ...`.

| VRT File | Source | Window (xOff,yOff,xSize,ySize) | Test Purpose |
|----------|--------|-------------------------------|--------------|
| crop_center_100x100.vrt | dem_500x500.tif | 200,200,100,100 | Basic center crop |
| crop_topleft_150x150.vrt | dem_500x500.tif | 0,0,150,150 | Top-left corner |
| crop_bottomright_200x200.vrt | dem_500x500.tif | 300,300,200,200 | Bottom-right corner |
| crop_lzw_200x200.vrt | dem_500x500_lzw.tif | 100,100,200,200 | LZW compressed source |
| crop_deflate_300x300.vrt | dem_500x500_deflate.tif | 50,50,300,300 | DEFLATE compressed source |
| crop_tiled_nonaligned.vrt | dem_500x500_tiled.tif | 37,41,67,83 | Non-tile-aligned window |
| crop_tiled_lzw_150x150.vrt | dem_500x500_tiled_lzw.tif | 50,50,150,150 | Tiled + LZW, spans tiles |
| crop_relative_path.vrt | ../source/dem_500x500.tif | 200,200,100,100 | relativeToVRT="1" |
| crop_int16_50x50.vrt | dem_100x100_int16.tif | 25,25,50,50 | Int16 data type |
| crop_float32_50x50.vrt | dem_100x100_float32.tif | 25,25,50,50 | Float32 data type |
| crop_int16_pred2_50x50.vrt | dem_100x100_int16_lzw_pred2.tif | 25,25,50,50 | LZW + PREDICTOR=2 |
| crop_int16_packbits_50x50.vrt | dem_100x100_int16_packbits.tif | 25,25,50,50 | PACKBITS compression |
| crop_sparse_120x120.vrt | dem_500x500_sparse.tif | 0,0,120,120 | Sparse tiles (all nodata) |
| fullsize_no_rect.vrt | dem_100x100_int16.tif | full size (no SrcRect/DstRect) | Full-size view without rects |

### VRT Structure (MVP Subset)

All valid VRTs follow this structure:

```xml
<VRTDataset rasterXSize="W" rasterYSize="H">
  <SRS>...</SRS>
  <GeoTransform>...</GeoTransform>
  <VRTRasterBand dataType="..." band="1">
    <NoDataValue>0</NoDataValue>
    <SimpleSource>
      <SourceFilename relativeToVRT="0|1">path/to/source.tif</SourceFilename>
      <SourceBand>1</SourceBand>
      <SrcRect xOff="X" yOff="Y" xSize="W" ySize="H" />
      <DstRect xOff="0" yOff="0" xSize="W" ySize="H" />
    </SimpleSource>
  </VRTRasterBand>
</VRTDataset>
```
If SrcRect/DstRect are omitted, the VRT is treated as a full-size view and rasterXSize/rasterYSize must match the source dimensions.

## Invalid VRT Files

These files test error handling for unsupported or malformed VRTs:

| File | Error Condition |
|------|-----------------|
| multiple_sources.vrt | Multiple SimpleSource elements (mosaic) |
| nonzero_dstrect_offset.vrt | DstRect xOff/yOff != 0 |
| size_mismatch.vrt | SrcRect size != DstRect size (resampling) |
| window_out_of_bounds.vrt | SrcRect extends beyond source image |
| band_not_one.vrt | band attribute != 1 |
| complex_source.vrt | ComplexSource instead of SimpleSource |
| negative_offset.vrt | Negative xOff value |
| vrt_size_mismatch.vrt | VRT rasterXSize/YSize != SrcRect xSize/ySize |
| missing_srcrect.vrt | DstRect present but SrcRect missing |
| missing_dstrect.vrt | SrcRect present but DstRect missing |

## Validation Approach

For each valid VRT:

1. **Load VRT** via Raster::new (parser + windowed GeoTIFF read)
2. **Read pixel data** using the windowed GeoTIFF path
3. **Compare** against reference TIF loaded directly
4. **Verify**: All pixel values match exactly (byte-for-byte for integer types, within tolerance for floats)

Example validation (Python with GDAL for reference):

```python
from osgeo import gdal
import numpy as np

vrt_ds = gdal.Open('vrt/crop_center_100x100.vrt')
ref_ds = gdal.Open('reference/crop_center_100x100.tif')

vrt_data = vrt_ds.GetRasterBand(1).ReadAsArray()
ref_data = ref_ds.GetRasterBand(1).ReadAsArray()

assert np.allclose(vrt_data, ref_data), "Pixel mismatch!"
```

## Test Scenarios by Block Layout

### Stripped (Row-oriented)
- `dem_500x500.tif` has 500x2 strips
- Tests: crop_center, crop_topleft, crop_bottomright

### Tiled (Block-oriented)
- `dem_500x500_tiled.tif` has 64x64 tiles
- `crop_tiled_nonaligned.vrt` tests partial tile reads (window at 37,41 doesn't align to 64-pixel boundaries)
- `crop_tiled_lzw_150x150.vrt` spans multiple tiles (50+150 > 64, crosses 3 tiles in each dimension)

### Compression Variants
- None: dem_500x500.tif
- LZW: dem_500x500_lzw.tif, dem_500x500_tiled_lzw.tif
- DEFLATE: dem_500x500_deflate.tif
- PACKBITS: dem_100x100_int16_packbits.tif

### Predictor Variants
- PREDICTOR=2: dem_100x100_int16_lzw_pred2.tif

## Creation Commands

All files created 2026-01-07 using GDAL 3.x.

```bash
# Source DEM (uncompressed, stripped)
gdal_translate -of GTiff -srcwin 1000 1000 500 500 -co COMPRESS=NONE \
  /wc1/culverts/.../hydro-enforced-dem.tif source/dem_500x500.tif

# Compressed variants
gdal_translate -of GTiff -co COMPRESS=LZW source/dem_500x500.tif source/dem_500x500_lzw.tif
gdal_translate -of GTiff -co COMPRESS=DEFLATE source/dem_500x500.tif source/dem_500x500_deflate.tif

# Tiled variants
gdal_translate -of GTiff -co TILED=YES -co BLOCKXSIZE=64 -co BLOCKYSIZE=64 \
  source/dem_500x500.tif source/dem_500x500_tiled.tif

# LZW with predictor=2 (Int16)
gdal_translate -of GTiff -co COMPRESS=LZW -co PREDICTOR=2 \
  source/dem_100x100_int16.tif source/dem_100x100_int16_lzw_pred2.tif

# PACKBITS compression (Int16)
gdal_translate -of GTiff -co COMPRESS=PACKBITS \
  source/dem_100x100_int16.tif source/dem_100x100_int16_packbits.tif

# Sparse tiled dataset (all nodata = 0)
gdal_calc.py -A source/dem_500x500.tif --calc="A*0" --NoDataValue=0 --type Float64 \
  --format GTiff --creation-option TILED=YES --creation-option BLOCKXSIZE=64 \
  --creation-option BLOCKYSIZE=64 --creation-option SPARSE_OK=YES \
  --outfile source/dem_500x500_sparse.tif --overwrite

# VRT creation (example)
gdal_translate -of VRT -srcwin 200 200 100 100 \
  source/dem_500x500.tif vrt/crop_center_100x100.vrt

# Reference TIF creation (example)
gdal_translate -of GTiff -srcwin 200 200 100 100 \
  source/dem_500x500.tif reference/crop_center_100x100.tif
```

## Geospatial Metadata

All files share these properties (inherited from source):
- **CRS**: EPSG:32619 (WGS 84 / UTM zone 19N)
- **Pixel Size**: ~1.0m x 1.0m
- **NoData**: 0
- **Origin**: Varies by crop window (computed from source origin + offset * pixel_size)

## Notes

- The `crop_relative_path.vrt` was hand-edited to use `relativeToVRT="1"` for path resolution testing
- The `fullsize_no_rect.vrt` fixture omits SrcRect/DstRect to exercise full-size view handling
- Invalid VRTs were hand-crafted to test specific error conditions
- Reference TIFs should produce byte-identical reads when compared to VRT-based windowed reads
