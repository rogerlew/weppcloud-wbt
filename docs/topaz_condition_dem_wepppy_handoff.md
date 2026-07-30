# TopazConditionDem WEPPpy handoff

Status: ready for WEPPpy integration. The production fixture has zero
cell-value mismatches against TOPAZ, and downstream D8 and watershed values
also match.

For general command-line and Python usage, output interpretation, limits, and
troubleshooting, see the
[TopazConditionDem end-user guide](topaz_condition_dem.ENDUSER.md).

## Interface

CLI:

    whitebox_tools -r=topaz_condition_dem \
      --dem=dem.tif \
      --output=conditioned.tif \
      --max_obstruction_width=2 \
      --delta=conditioned_delta.tif \
      --diagnostics=conditioned.json

Both Python bindings expose:

    wbt.topaz_condition_dem(
        dem,
        output,
        max_obstruction_width=2,
        delta=None,
        diagnostics=None,
        callback=None,
    )

The obstruction width must be 0, 1, or 2. Two is the historical WEPPpy TOPAZ
default. The primary output is the post-RELIEF DEM. The optional delta is
`conditioned - TOPAZ-rounded input`.

The tool rounds valid source elevations to TOPAZ's decimeter input precision.
NoData geometry and georeferencing are preserved. Output and delta rasters are
64-bit floating point because RELIEF increments may be `0.00001` elevation
unit.

Diagnostics JSON schema version 1 reports stage counts, fill/cut/relief
extrema, obstruction counts, and qualified volumes. Volume assumes compatible
projected horizontal and vertical units; WEPPpy should display that
qualification rather than silently labeling every result cubic meters.

## Integration placement

Add `topaz_condition` alongside the existing fill/breach conditioning choices.
Run it after DEM acquisition/cropping and before D8 pointer generation. Keep the
conditioned DEM, delta raster, and diagnostics JSON as run artifacts. Suggested
names:

    dem/topaz_conditioned.tif
    dem/topaz_condition_delta.tif
    dem/topaz_condition_diagnostics.json

Regression tests should assert argument construction, artifact persistence,
NoData preservation, and that D8 consumes the conditioned DEM. Do not infer
success from process exit alone; validate the output raster and diagnostics
schema.

## Production result

For the exact 430-by-447 DEM from `srivas42-reconciled-turf`, original TOPAZ
keeps the requested outlet at 533.0 m. FILDEP changes 581 cells, with maximum
fill 2.3 m and maximum cut 5.0 m. RELIEF changes 1,924 cells by at most
0.00148 m. This avoids the approximately 910.1 m fill produced by the existing
WBT fill workflow.

The Rust output also keeps the outlet at 533.0 m and matches all 192,210 valid
TOPAZ output cells.
