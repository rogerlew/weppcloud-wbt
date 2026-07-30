# DEM Conditioning Diagnostics

`FillDepressions`, `BreachDepressions`, `BreachDepressionsLeastCost`, and
`TopazConditionDem` can write a JSON diagnostics sidecar:

```text
--diagnostics=relief.diagnostics.json
--diagnostics_id=0123456789abcdef0123456789abcdef
```

The identifier is supplied by the caller and must be 32 lowercase hexadecimal
characters. It lets a workflow reject a stale sidecar from an earlier attempt.
The file is written through a same-directory temporary file and atomic rename;
a successful tool exit means both the raster and requested diagnostics exist.

All four tools report source-to-output maximum terrain raise and cut, affected
cell counts and areas, and fill/cut volumes. Method-specific fields distinguish
depression filling, breach paths, least-cost search resolution and fallback,
TOPAZ narrow-obstruction adjustments, and synthetic flat relief.

TOPAZ stage statistics use its rounded working elevations, while the common
terrain-change block compares the final raster with the original input.

The sidecar assumes metre horizontal and elevation units. Callers must ensure
that input raster units satisfy that contract before interpreting areas and
volumes.
