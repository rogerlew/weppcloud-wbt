# TopazConditionDem

Use this tool to condition a digital elevation model (DEM) with
TOPAZ-compatible depression filling, narrow-obstruction adjustment, and flat
resolution before deriving flow directions.

## What This Is For

`TopazConditionDem` is a source-level Rust translation of the FILDEP and RELIEF
terrain-conditioning methods in USDA-ARS TOPAZ DEDNM 3.10. It is intended for
WEPP and other workflows that need TOPAZ-compatible conditioned elevations
rather than the result of a generic depression-filling algorithm.

The tool performs two stages:

1. **FILDEP** fills depressions and can lower narrow spill obstructions one or
   two cells wide.
2. **RELIEF** adds very small, deterministic elevation increments across flats
   so that downstream flow-direction tools can establish drainage paths.

The primary output is the post-RELIEF conditioned DEM. The tool does not
calculate a D8 pointer, contributing area, channels, or watersheds.

## When to Use It

Run `TopazConditionDem` after acquiring, projecting, and cropping the DEM and
before calculating D8 flow direction or any derivative that depends on flow
direction.

Use this tool when TOPAZ compatibility matters. Do not treat it as
interchangeable with `FillDepressions`, `BreachDepressions`, or other
conditioning tools: their spill selection, obstruction handling, flat
resolution, and numerical precision differ.

## Before You Begin

Required inputs:

- `--dem` (or `-i`) — input DEM raster
- `--output` (or `-o`) — post-RELIEF conditioned DEM

Optional outputs:

- `--fildep` — intermediate DEM after FILDEP and before RELIEF
- `--delta` — signed difference between the final conditioned DEM and the
  TOPAZ-rounded input
- `--diagnostics` — JSON summary of conditioning counts, extrema, and volumes

The input must contain finite elevations in valid cells. NoData geometry and
the input raster's georeferencing are preserved.

## Key Terms and Settings

| Setting | Meaning | Default |
|---------|---------|---------|
| `--max_obstruction_width=0` | Fill depressions without cutting through narrow spill obstructions | — |
| `--max_obstruction_width=1` | Permit adjustment through a one-cell obstruction | — |
| `--max_obstruction_width=2` | Permit adjustment through one- or two-cell obstructions; historical WEPPpy TOPAZ behavior | `2` |
| `--fildep` | Write the post-FILDEP, pre-RELIEF DEM | not written |
| `--delta` | Write `conditioned - TOPAZ-rounded input`; negative values identify cuts | not written |
| `--diagnostics` | Write diagnostics JSON, schema version 1 | not written |

`--max_obstruction_width` accepts only `0`, `1`, or `2`. This setting is a
narrow-obstruction adjustment mode; it is not a general breach-length setting.

## Command-Line Example

```bash
whitebox_tools -r=TopazConditionDem \
  --dem=dem.tif \
  --output=topaz_conditioned.tif \
  --max_obstruction_width=2 \
  --fildep=topaz_fildep.tif \
  --delta=topaz_condition_delta.tif \
  --diagnostics=topaz_condition_diagnostics.json
```

Relative paths can be combined with the standard WhiteboxTools working
directory option:

```bash
whitebox_tools -r=TopazConditionDem \
  --wd="/path/to/run/dem" \
  --dem=dem.tif \
  --output=topaz_conditioned.tif
```

## Python Example

Both bundled Python wrappers expose the same method:

```python
from whitebox_tools import WhiteboxTools

wbt = WhiteboxTools()
status = wbt.topaz_condition_dem(
    "dem.tif",
    "topaz_conditioned.tif",
    max_obstruction_width=2,
    fildep="topaz_fildep.tif",
    delta="topaz_condition_delta.tif",
    diagnostics="topaz_condition_diagnostics.json",
    timeout=900,
)
if status != 0:
    raise RuntimeError("TopazConditionDem failed")
```

The `timeout` argument is a Python-wrapper process timeout in seconds; it is
not a native `TopazConditionDem` CLI parameter.

## Interpreting the Outputs

### Conditioned DEM

The primary output has the same extent, dimensions, coordinate reference
system, and NoData layout as the input. It uses 64-bit floating-point storage.

Before conditioning, valid elevations are rounded using TOPAZ's single-precision
decimeter input convention. RELIEF may then add increments as small as
`0.00001` elevation unit. Therefore, unchanged-looking cells can differ from
the original source solely because of TOPAZ input quantization.

### FILDEP stage

The optional `--fildep` raster isolates depression filling and obstruction
adjustment from the subsequent RELIEF step. Compare it with the final output
when diagnosing whether a change came from FILDEP or flat resolution.

### Delta raster

The optional delta is:

```text
final conditioned elevation - TOPAZ-rounded input elevation
```

Positive values are fills or synthetic relief. Negative values are cuts made
by narrow-obstruction adjustment. It is not a subtraction from the original
full-precision raster.

### Diagnostics JSON

The optional schema-version-1 JSON reports:

- depression and flat counts;
- filled, lowered, and synthetic-relief cell counts;
- one- and two-cell obstruction-adjustment counts;
- maximum fill, cut, and synthetic relief in elevation units;
- fill and cut volumes; and
- input/output identity, raster dimensions, parameters, and TOPAZ source
  revision.

Volume values use projected raster cell area. Interpret them only after
confirming that horizontal and vertical units are compatible. The tool does
not automatically convert, for example, horizontal meters and vertical feet
into a common unit.

## Assumptions and Limits

- Obstruction adjustment can lower cells. The conditioned DEM is not
  guaranteed to be greater than or equal to the input everywhere.
- NoData is treated as an open lower boundary for adjacent valid terrain,
  matching TOPAZ behavior.
- Raster-edge cells participate as outlets and neighbors but are not RELIEF
  candidate seeds.
- TOPAZ source emits a warning when a flat's row or column span exceeds 2,500
  cells. This Rust implementation dynamically stores flat membership and
  currently neither limits processing nor emits that warning.
- The implementation targets TOPAZ numerical compatibility. It intentionally
  preserves TOPAZ rounding, scan order, and tie-breaking rather than applying
  alternative hydrologic corrections.
- Runtime can increase substantially for large or extensive flat regions.

## Recommended Quality Checks

After a run:

1. Confirm that the conditioned DEM and, when requested, diagnostics JSON
   exist and can be opened.
2. Verify that extent, dimensions, coordinate reference system, and NoData
   layout match the input.
3. Inspect the delta raster for the location and magnitude of fills and cuts.
4. Review diagnostics extrema and volumes for physically implausible changes.
5. Derive the D8 pointer from the conditioned DEM, not from the original DEM.
6. Confirm the expected outlet and drainage route in the downstream workflow.

## Troubleshooting

- **`--max_obstruction_width must be 0, 1, or 2`** — use one of the three
  supported TOPAZ modes.
- **Valid DEM cells must contain finite elevations** — replace or mark `NaN`
  and infinite values as NoData before running.
- **Unexpected negative delta values** — obstruction modes `1` and `2` may
  lower narrow spill cells. Re-run with mode `0` if cuts are not desired.
- **Small changes across broad flats** — these are expected RELIEF increments,
  not conventional fill depth.
- **Unexpected changes of up to about 0.05 elevation unit** — check the
  TOPAZ decimeter input rounding before attributing the difference to FILDEP or
  RELIEF.
- **Unexpected drainage beside a NoData mask** — NoData acts as an open lower
  boundary. Fill or otherwise preprocess unintended holes before conditioning
  if they should not drain the surrounding terrain.
- **Python call times out** — increase `timeout` or omit it after confirming
  the input does not contain an unexpectedly large flat.

## Related Documentation

- [Algorithm and numerical contract](topaz_condition_dem_algorithm.md)
- [WEPPpy integration handoff](topaz_condition_dem_wepppy_handoff.md)
- [Parity validation](../prompts/artifacts/topaz_condition_dem_validation.md)
