# TopazConditionDem fixtures

This directory contains production, synthetic, and production-derived DEMs
used for TOPAZ parity validation:

| File | Rows × columns | Bytes | SHA-256 |
| --- | ---: | ---: | --- |
| `dem.tif` | 430 × 447 | 769,856 | `b87f189bf3aa79b7f25542f0982378e193d11164fec55a68f7310e6256a8282a` |
| `burned-out-harmonic_dem.tif` | 1,184 × 1,233 | 5,846,960 | `a2535e3564f8ebc488d3af18f05f0eaf80b25d305eac22bfd59b56c8cbe9757f` |
| `portland_BRnearMultnoma_HighSevS.202009.chn_cs200_dem.tif` | 1,233 × 1,269 | 6,268,981 | `d0e4c4f1cd32f03ba4cda6d092935a138102214c2199c4a61da5f58266626106` |
| `synthetic_irregular_nodata.tif` | 41 × 47 | 1,874 | `6a66f27ae94c9957977231ea00f25ba0f9b4ab255fb1fd1213be9daeb64e876d` |
| `burned-out-harmonic_nlcd-water-mask_dem.tif` | 1,184 × 1,233 | 5,009,207 | `4197c6689ca96f8dd349ea200b2776e27c7691ebd9a8e13ac7e7150e973ec78f` |

All are 30 m rasters in EPSG:32610. The synthetic fixture deliberately includes
an edge-connected stair-step corridor, irregular internal holes, an isolated
NoData cell, and a one-cell valid island. The derived burned-out-harmonic
fixture masks 25,541 cells where an exact-grid WMesque request for the
project-configured `nlcd/2019` alias returned class 11.

`parity_manifest.json` is the canonical golden contract. It records the input,
mask, FILDEP, and RELIEF hashes for seven TOPAZ cases, including obstruction
widths 0, 1, and 2. Validate a fresh release binary with:

    /usr/bin/python3 tools/validate_topaz_condition_dem_parity.py \
      --binary target/release/whitebox_tools \
      --manifest test_fixtures/topaz_condition_dem/parity_manifest.json \
      --all

The canonical stage hash is SHA-256 over valid cells in raster row-major order
as little-endian signed 32-bit TOPAZ internal units. This excludes TIFF
metadata and Fortran record framing. `additional_parity.json` retains the
earlier detailed evidence for the two larger all-valid production fixtures.

Materialize the checksummed TOPAZ stage rasters with
`tools/prepare_topaz_condition_dem_fixture.py`, supplying `dem.tif` and the
preprocessing-only TOPAZ output directory. The generated manifest records
checksums and raster metadata so a cropped or resampled substitute cannot pass
unnoticed.

Generate the synthetic input with
`tools/create_topaz_condition_dem_synthetic_nodata_fixture.py`. Recreate the
water-mask fixture with
`tools/create_topaz_condition_dem_nlcd_water_fixture.py` after retrieving the
documented exact-grid NLCD source. TOPAZ oracle outputs are generated under
`target/` with `tools/run_topaz_condition_dem_oracle.py`; they are not
committed.
