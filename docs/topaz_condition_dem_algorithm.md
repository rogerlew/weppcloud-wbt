# TopazConditionDem algorithm

`TopazConditionDem` is a source-level Rust translation of the FILDEP and RELIEF
terrain-conditioning methods in USDA-ARS TOPAZ DEDNM 3.10. The normative source
is `/workdir/topaz/src/dednm.f90` at revision
`116607fc1185800ca78e387454ef1ccd3ffd73b4`. Original authors J. Garbrecht and
L. Martz must be acknowledged when the numerical methods are used or described.

The tool conditions elevations only. TOPAZ FLOVEC, FLOPAT, UPAREA, BOUND,
channel extraction, and watershed delineation are not ported. Run an existing
WhiteboxTools pointer tool on the conditioned output.

## Numeric contract

TOPAZ reads elevation as a 32-bit REAL, multiplies it by ten, applies nearest
integer rounding, and represents the resulting decimetre value internally on a
`1/100000` elevation-unit scale. Consequently, every initial elevation is a
multiple of 10,000 internal units. RELIEF later adds integer increments on the
same scale: two internal units in its first pass and one in its second pass.

The Rust implementation deliberately performs the multiplication in `f32`.
Using `f64` changes half-decimetre cases present in the production DEM.

NoData cells remain NoData and are excluded from conditioning. Non-finite valid
cells and internal-scale overflow are errors.

## FILDEP

FILDEP visits interior cells in row-major order. A candidate has no lower
eight-neighbour and at least one higher neighbour. From the candidate it grows
the connected, monotonically non-decreasing depression region. The original
uses search windows that expand in five-cell steps; expansion is an
implementation optimization and does not change the intended region.

The spill is the lowest region cell on the raster edge or adjacent to a lower
cell outside the region. With obstruction adjustment disabled, cells at or
below the spill are raised to it.

For one-cell adjustment, FILDEP looks for the greatest defensible cut through a
single spill obstruction. For two-cell adjustment, it also examines a second
cell on the depression side. It selects greatest cut first and shortest
eight-neighbour path second. In the final equal-distance replacement case,
TOPAZ compares the outside drop with the selected cut, and the candidate must
remain connected to the seed without crossing cells resolved by an earlier
depression. These details determine which member of an otherwise equivalent
two-cell obstruction is lowered. FILDEP then fills the depression to the
adjusted spill. The public modes are 0, 1, and 2; the default is 2, matching
WEPPpy's historical TOPAZ control.

## RELIEF

RELIEF identifies cells without a lower neighbour and groups equal-elevation
eight-connected flats. It assigns distance-like increments from surrounding
higher terrain, then performs two iterative propagation passes. The first uses
two internal units and includes the synthetic relief field; the second uses one
internal unit. Iteration and scan order are deterministic.

The source warns when a flat's row or column span exceeds 2,500 cells. The
executable source uses 2,500; older prose referring to a different threshold is
not normative.

Raster edge cells are not candidate seeds but participate as outlets and
neighbours. This preserves TOPAZ's open-edge drainage behavior.

## Current parity status

On the 430-by-447 production DEM, all 192,210 valid final cells match TOPAZ
exactly. Input quantization, fill/cut counts and extrema, RELIEF modifications,
derived D8 pointer values, and the outlet watershed values also match. See
`prompts/artifacts/topaz_condition_dem_validation.md`.
