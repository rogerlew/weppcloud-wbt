# TopazConditionDem fixtures

Synthetic cases are embedded in the Rust module. `dem.tif` is the exact
430-by-447 production-representative DEM used for the TOPAZ parity validation:

    SHA-256: b87f189bf3aa79b7f25542f0982378e193d11164fec55a68f7310e6256a8282a
    Size: 769856 bytes
    Resolution: 30 m
    CRS: EPSG:32610

Materialize the checksummed TOPAZ stage rasters with
`tools/prepare_topaz_condition_dem_fixture.py`, supplying `dem.tif` and the
preprocessing-only TOPAZ output directory. The generated manifest records
checksums and raster metadata so a cropped or resampled substitute cannot pass
unnoticed.
