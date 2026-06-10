# weppcloud-wbt

**weppcloud-wbt** is an actively maintained Rust geospatial toolkit for WEPP watershed preprocessing. It provides the terrain parameterization, channel-network construction, outlet discovery, and DEM conditioning tools used operationally by [WEPPcloud](https://github.com/rogerlew/wepppy) — a published online interface for the Water Erosion Prediction Project (WEPP) model.

The software is built on the foundation of John Lindsay's [WhiteboxTools](https://github.com/jblindsay/whitebox-tools). We are grateful for the substantial body of work Lindsay contributed to open-source geospatial analysis. That repository is [officially marked as legacy](https://github.com/jblindsay/whitebox-tools) and has received no commits since February 2025, as development moved to a separate commercial and open-source successor. **weppcloud-wbt is the actively maintained branch of the WhiteboxTools codebase for WEPP and WEPPcloud workflows.**

---

## Tools added in this fork

All tools below are used operationally within WEPPcloud. Python bindings are included in `whitebox_tools.py` and `WBT/whitebox_tools.py` unless noted.

### Hydrology / terrain

- **`HillslopesTopaz`** (`hydro_analysis/hillslopes_topaz.rs`)
  Implements Garbrecht & Martz TOPAZ-style stream and hillslope identifiers for a single watershed. Emits channel metadata tables (`netw.tsv`, `netw_props.tsv`) and left/right/top hillslope rasters consumed by WEPPcloud. Includes combined flood-fill phases, cached upstream areas, and per-link `areaup` attributes.

- **`FindOutlet`** (`hydro_analysis/find_outlet.rs`)
  Derives a single-stream outlet pour-point GeoJSON by tracing D8 flow from interior candidates. Supports optional watershed masks and requested start locations (`--requested_outlet_lng_lat`, `--requested_outlet_row_col`) for interactive callers.

- **`FVSlope`** (`hydro_analysis/fvslope.rs`)
  Computes slope in the D8 flow direction with ESRI pointer support, z-factor, and unit controls (ratio, degrees, percent, radians). Mirrors TOPAZ-style flow-vector slopes used by WEPP channel hydraulics. Output unit is recorded in raster metadata.

- **`RaiseRoads`** (`hydro_analysis/raise_roads.rs`)
  Conditions DEMs for road embankments using `constant`, `profile_relative`, or `cross_section` strategies while enforcing a no-lowering guarantee (`output ≥ input` on all valid cells). Supports GeoJSON attribute overrides for cross-section parameters, width/parameter fallback hierarchy, conservative unpaved-road behavior, and automatic reprojection of road vectors to DEM CRS.

- **`Watershed`** update (`hydro_analysis/watershed.rs`)
  Extended to accept GeoJSON pour-point inputs (Point and MultiPoint features) in addition to shapefiles and rasters.

- **`UnnestBasins`** update (`hydro_analysis/unnest_basins.rs`)
  Writes a `<output_stem>_hierarchy.csv` sidecar encoding parent/child outlet relationships, nesting order, hierarchy level, and outlet grid coordinates. Replaces per-order full-grid flowpath retracing with a one-pass outlet assignment plus per-order ancestor remapping.

### Stream network

- **`StreamJunctionIdentifier`** (`stream_network_analysis/stream_junctions.rs`)
  Counts inflowing tributaries for every stream pixel, producing junction maps WEPPcloud uses to locate confluences, outlets, and pseudo-gauges.

- **`PruneStrahlerStreamOrder`** (`stream_network_analysis/prune_strahler_order.rs`)
  Drops first-order (Strahler order = 1) links from an existing order grid, subtracts one from remaining orders, and optionally preserves zero-valued background cells or collapses retained links to a binary mask.

- **`IterativeFirstOrderLinkPrune`** (`stream_network_analysis/iterative_first_order_link_prune.rs`)
  Two-stage stream qualification and pruning with local threshold support (`--threshold_code_raster` + `--threshold_table`) and deterministic first-order-link pruning behavior for TOPAZ-parity workflows. Specification: [docs/iterative-first-order-link-prune/specification.md](docs/iterative-first-order-link-prune/specification.md).

- **`RemoveShortStreams`** enhancement (`stream_network_analysis/remove_short_streams.rs`)
  Adds `--max_junctions` pruning with iterative branch deletion so no junction retains more than the requested number of inflows.

### GIS

- **`ClipRasterToRaster`** (`gis_analysis/clip_raster_to_raster.rs`)
  Cell-wise raster masking: passes input values where the mask is valid and non-zero, writes nodata elsewhere. Reduces full-raster reads in cloud preprocessing steps.

### Raster I/O

- **VRT support** (read-only, single `SimpleSource`)
  Adds `.vrt` detection, a minimal VRT XML parser, and a windowed GeoTIFF read path to avoid full in-memory loads for cropped inputs. Supports full-size VRTs without `SrcRect`/`DstRect` when dimensions match the source.

### Runtime and Python API

- **CLI error propagation** — `main.rs` returns `Result`, enabling backtraces from scripted environments.
- **Python wrapper enhancements** — `raise_on_error` semantics, custom exceptions, environment propagation, and richer error reporting across all tools.
- **`Slope` unit extension** — ratio units added; chosen unit recorded in output metadata.

---

## Building from source

This fork targets Linux (Ubuntu 24.04) for WEPPcloud deployment. The compiled `WBT/` directory is tracked in the repository so deployment artifacts stay versioned alongside the code.

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone this repository
git clone https://github.com/rogerlew/weppcloud-wbt.git
cd weppcloud-wbt

# Release build
cargo build --release -p whitebox-tools-app

# Run tests
cargo test -p whitebox_raster --tests
cargo test -p whitebox-tools-app
```

Full release build and WEPPcloud deployment procedure: [docs/release-build-install.md](docs/release-build-install.md).

---

## Developer guide

[DEVELOPING_TOOLS.md](DEVELOPING_TOOLS.md) covers tool structure, test fixture conventions, Python binding patterns, and the integration test requirements expected for new tools.

---

## Acknowledgements

This project is built on [WhiteboxTools](https://github.com/jblindsay/whitebox-tools) by John Lindsay at the University of Guelph. Lindsay's work established the Rust geospatial analysis framework, hydrologic toolchain, raster I/O layer, and Python API that make this fork possible. The TOPAZ algorithms implemented here draw on the foundational work of Jurgen Garbrecht and Lawrence Martz. We are grateful to both for their contributions to open-source hydrologic terrain analysis.
