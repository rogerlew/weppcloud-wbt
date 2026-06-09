# Task: UnnestBasins integration test suite

## Goal

Author `whitebox-tools-app/src/tools/hydro_analysis/unnest_basins_integration_tests.rs` and wire it into `unnest_basins.rs`. Run `cargo test` to confirm all new tests pass before reporting done.

## Context

`UnnestBasins` (`hydro_analysis/unnest_basins.rs`) delineates nested sub-watersheds for a set of pour points. It reads a D8 flow-pointer raster and a Shapefile of pour points, assigns each pour point a nesting order (innermost = 1, outermost = N), and writes one output raster per nesting level. The raster for nesting order `k` labels each cell with the ID of the order-k ancestor outlet it ultimately drains to.

**The fork's specific contribution** (claimed in the JOSS paper) is the basin hierarchy sidecar: `write_hierarchy_sidecar` writes `<output_stem>_hierarchy.csv` alongside the raster outputs, encoding outlet parent/child relationships and nesting metadata.

The existing `mod tests { }` block at the bottom of `unnest_basins.rs` contains unit tests for internal pure functions. **Do not modify or replace that block.** Wire the integration test module as a second `#[cfg(test)]` block using `#[path = ...]`.

### Output file naming

Given `--output /tmp/test_123.tif`:

- Raster for nesting order 1: `/tmp/test_123_1.tif`
- Raster for nesting order 2: `/tmp/test_123_2.tif`
- Hierarchy sidecar: `/tmp/test_123_hierarchy.csv`

The extension replacement is done via `output_file.replace(ext, &format!("_{}{}", order, ext))` — so a temp output path of `/tmp/unnest_basins_{PID}_{NANOS}.tif` produces `/tmp/unnest_basins_{PID}_{NANOS}_1.tif` etc.

### Input format constraint

`UnnestBasins` only accepts Shapefile (`.shp`) pour-point input via `Shapefile::read()`. No GeoJSON or raster fallback exists. No fixture directory currently contains a `.shp` file; integration tests must create a temporary Shapefile at runtime.

### Shapefile write API

Look at `whitebox-tools-app/src/tools/lidar_analysis/las_to_shapefile.rs` for the write pattern. It uses `Shapefile::new(&path, ShapeType::Point)`, `add_point_record(x, y)`, and `attributes.add_record(...)`. The `whitebox_vector` crate is already in `Cargo.toml`. UnnestBasins does not read attributes, so a minimal attribute table (or empty attribute records) is fine.

### Nesting order semantics

- All outlets start with `nesting_order = 0`.
- Processing iterates outlets in Shapefile record order. For each outlet, trace downstream; every downstream outlet encountered increments `cur_order` and may raise that outlet's stored nesting order.
- Result: a root outlet (no downstream outlet) ends up with the **highest** nesting order. An innermost outlet that flows through the most other outlets ends up with nesting order 1.
- For a **single outlet**: `nesting_order=1`, `max_nesting_order=1`, one output raster (`_1.tif`), one hierarchy CSV row: `outlet_id=1, parent_outlet_id=0, is_root=true, nesting_order=1, hierarchy_level=0`.
- For **two nested outlets** (downstream first in the Shapefile, upstream second):
  - outlet_id=1 (downstream): trace finds no other outlet → `nesting_order[1]=1`
  - outlet_id=2 (upstream): trace hits outlet_id=1 → `nesting_order[1]` raised to 2
  - Final: outlet_id=2 → `nesting_order=1` (innermost), outlet_id=1 → `nesting_order=2` (root/outermost)
  - `max_nesting_order=2`, two output rasters (`_1.tif`, `_2.tif`)
  - `_2.tif` covers the full watershed (root outlet), `_1.tif` covers the subcatchment of the upstream outlet

## Patterns to follow

Read before writing:

- `hydro_analysis/hillslopes_topaz_integration_tests.rs` — canonical pattern (`CARGO_MANIFEST_DIR`, `temp_output_path`, `cleanup_output_path`, `Raster::new` for validation)
- `hydro_analysis/watershed_integration_tests.rs` — example of writing a temporary file (GeoJSON) at runtime and cleaning it up

## Fixtures

### `test_fixtures/minimal_1pixel_stream/`

An 18×57 synthetic watershed. 144 cells drain to a single outlet. Relevant files:

- `flovec.tif` — D8 pointer input
- `bound.tif` — watershed boundary mask; positive values identify the 144 in-watershed cells

Outlet coordinate (projected, same CRS as `flovec.tif`):

```
x = 278362.5000017698
y = 4868384.500118681
```

### `test_fixtures/minimal_2pixel_stream/`

Same 18×57 watershed with a two-cell stream. Relevant files:

- `flovec.tif` — D8 pointer input
- `bound.tif` — watershed boundary mask

Two outlet coordinates (both projected, same CRS as `flovec.tif`):

| Role | Row | Col | x | y |
|------|-----|-----|---|---|
| Downstream (root) | 9 | 50 | 278362.5000017698 | 4868384.500118681 |
| Upstream (nested) | 10 | 49 | 278361.5000017698 | 4868383.500118681 |

Write the Shapefile with the **downstream outlet as record 0** (outlet_id=1) and the **upstream outlet as record 1** (outlet_id=2). The nesting order calculation is sensitive to Shapefile record order.

## Required tests

Write exactly three tests.

### 1. Single-outlet end-to-end (`single_outlet_end_to_end`)

Create a temporary Shapefile with one Point at the `minimal_1pixel_stream` outlet coordinate. Run `UnnestBasins` with `minimal_1pixel_stream/flovec.tif` and the temp Shapefile, with output at a temp path. Assert:

- The `_1.tif` output file exists and can be opened by `Raster::new`.
- No `_2.tif` output file exists (single outlet → single nesting order).
- The `_hierarchy.csv` sidecar file exists.
- Output `_1.tif` dimensions (rows, columns) match the D8 pointer raster.
- All cells with a positive value in `bound.tif` have output value `1.0` in `_1.tif` (single outlet, FID=1).
- At least one cell has value `1.0` (non-trivial delineation).

Clean up all temp files (`_1.tif`, `_hierarchy.csv`, temp Shapefile and its sidecar files `.shx`, `.dbf`).

### 2. Single-outlet hierarchy CSV content (`single_outlet_hierarchy_csv_fields`)

Repeat the single-outlet run (same fixture, same outlet coordinate, fresh temp paths). After the run, read and parse `_hierarchy.csv`. Assert:

- The header line matches exactly: `outlet_id,parent_outlet_id,child_count,child_ids,nesting_order,hierarchy_level,is_root,row,column`
- There is exactly one data row (header + 1 row total = 2 lines).
- Parse the data row by splitting on `,`. Assert field by field:
  - `outlet_id = 1`
  - `parent_outlet_id = 0`
  - `child_count = 0`
  - `child_ids` is empty (the field between the 4th and 5th comma is an empty string)
  - `nesting_order = 1`
  - `hierarchy_level = 0`
  - `is_root = true`

Clean up all temp files.

### 3. Two nested outlets produce two nesting-order rasters and a correct hierarchy (`two_nested_outlets_produce_nested_outputs`)

Create a temporary Shapefile with two Points — downstream outlet as record 0, upstream outlet as record 1 (coordinates from the table above). Run `UnnestBasins` with `minimal_2pixel_stream/flovec.tif` and the temp Shapefile. Assert:

- Both `_1.tif` and `_2.tif` output files exist.
- The hierarchy CSV exists.
- `_2.tif` (root outlet, full watershed) has at least as many labeled cells (non-nodata) as `_1.tif` (upstream subcatchment).
- Parse `_hierarchy.csv`. Assert:
  - Exactly 2 data rows.
  - Exactly one row has `is_root=true` and `parent_outlet_id=0`.
  - Exactly one row has `is_root=false` and `parent_outlet_id > 0` (the nested outlet has a parent).
  - The root row has `child_count=1` (it has one nested child).

Clean up all temp files (`_1.tif`, `_2.tif`, `_hierarchy.csv`, temp Shapefile files).

## Run requirement

After writing the tests, run:

```
cargo test -p whitebox-tools-app unnest_basins_integration
```

All three tests must pass. If any test fails, diagnose the root cause — do not paper over failures with `#[ignore]`. Do not report done until you have seen passing output from this command.

## Constraints

- Write output rasters and temp Shapefiles to temp paths using the standard helper pattern. Never write into `test_fixtures/`.
- Clean up all temp files after each test, including `.shx` and `.dbf` Shapefile sidecar files.
- No new dependencies. Use only crates already in `Cargo.toml`.
- Do not modify any fixture file.
- Do not modify the existing `mod tests { }` block in `unnest_basins.rs`.
- The `child_ids` field in the CSV is a comma-separated list of child IDs enclosed in quotes in some implementations, or a plain empty string if there are no children. Parse defensively — strip surrounding quotes if present before asserting emptiness for the no-children case.
