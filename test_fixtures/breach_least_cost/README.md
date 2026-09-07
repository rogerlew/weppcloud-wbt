# Least-cost breaching fixtures

`valid-tabletop/dem.tif` is the unchanged project DEM copied from
`wepp1:/geodata/wc1/runs/va/valid-tabletop/dem/dem.tif` on 2026-09-07 UTC.
Grid: 2,015 columns by 2,053 rows, 5 m cells. SHA-256:
`6d49e9e92daef8a9b5709db6833975cebfafda92ac97d6c4d6d77eeb1f05ee60`.

Reproduce the original conditioning settings with `--dist=600 --min_dist
--fail_on_unresolved`, without `--fill`. The 600 cells correspond to 3,000 m.
The production RQ job 6b9be406-92df-4706-bd80-e99e62aff37c timed out after 600 s.
A separate manual benchmark was stopped after 794.319 s at the user's request;
that is an interrupted lower bound, not completed runtime.

Existing smaller real and synthetic DEMs under ../topaz_condition_dem are reused
for parity. See ../../docs/work-packages/20260907_breach_least_cost_optimization/package.md
for acceptance requirements. Raw benchmark outputs and binaries belong in target/.

## Synthetic cases

`tools/create_breach_synthetic_fixtures.py` deterministically generates
`tied_pits.tif` (interacting equal-height pits and internal NoData),
`flat_edge.tif` (uniform flat to outer edges), and `flat_edge_perturbed.tif`
(raised, lowered, and NoData cells interrupting the flat). These exercise exact
translation reuse and rejection, ordered conflicts, and filling. The parity
runner also varies maximum cost, distance, minimum-distance mode, and flat
increment. Input hashes are retained in the package's parity artifact.
