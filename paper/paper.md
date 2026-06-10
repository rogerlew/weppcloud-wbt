---
title: 'weppcloud-wbt: TOPAZ-style watershed parameterization tools for WEPP workflows in WhiteboxTools'
tags:
  - hydrology
  - geomorphometry
  - watershed delineation
  - soil erosion
  - WhiteboxTools
  - WEPP
  - Rust
authors:
  - name: Roger Lew
    orcid: 0000-0002-4280-052X
    affiliation: 1
affiliations:
  - name: University of Idaho, United States
    index: 1
date: 9 June 2026
bibliography: paper.bib
---

# Summary

`weppcloud-wbt` (https://github.com/rogerlew/weppcloud-wbt) [@lew_2026_weppcloud_wbt] is a research software fork of WhiteboxTools that provides the watershed-topology and terrain-processing tools used by WEPPcloud, a published online interface for the Water Erosion Prediction Project (WEPP) model [@lew_etal_2022_weppcloud_part_i]. WEPP is a process-based soil erosion and sediment-delivery model used to simulate runoff, erosion, deposition, and sediment yield from hillslopes and small watersheds [@flanagan_nearing_1995_wepp]. WEPPcloud makes these simulations accessible by automatically assembling model inputs from public geospatial and climate datasets and by storing model runs remotely for later review, comparison, and reuse [@lew_etal_2022_weppcloud_part_i].

The software is distributed on PyPI as `weppcloud-wbt` with platform wheels for macOS, Windows, and Linux.

To run WEPP at watershed scale, a digital elevation model must be transformed into model-ready representations of channels, hillslopes, outlets, slopes, watershed masks, and channel-network metadata. `weppcloud-wbt` provides this preprocessing layer in a modern Rust geospatial framework while preserving the conceptual structure of the TOPAZ digital landscape analysis system [@garbrecht_martz_2004_topaz_overview_manual; @martz_garbrecht_1992_drainage_network_dem; @martz_garbrecht_1993_automated_extraction_dem]. The software adds TOPAZ-style hillslope and channel identifiers, outlet discovery, stream-network pruning, flow-vector slope calculation, terrain conditioning, GeoJSON pour-point support, and cloud-workflow adaptations including read-only VRT input. These tools are orchestrated through the `WhiteboxToolsTopazEmulator` adapter, which exposes the preprocessing pipeline to published WEPPcloud workflows.

# Statement of need

TOPAZ remains an important and durable conceptual model for hydrologic terrain analysis. It formalized automated drainage-network extraction, watershed segmentation, subcatchment parameterization, flat-area treatment, depression handling, and raster channel topology in ways that remain useful for erosion modeling and other watershed applications [@garbrecht_martz_1997_flat_surfaces; @garbrecht_martz_1997_channel_ordering; @martz_garbrecht_1998_flats_depressions]. However, the operational needs of current WEPPcloud workflows expose limitations in the legacy toolchain. Traditional TOPAZ workflows rely on ASCII-era file formats and flat-file serialization, assume the target watershed is completely contained inside the DEM boundary, and produce channel-network topologies with greater than 3 channel inflows that can conflict with WEPP's downstream consumer constraints. The legacy source is also fixed-form Fortran 77, which makes targeted modification, testing, instrumentation, and integration into modern cloud services difficult.

`weppcloud-wbt` addresses these constraints for researchers and operational modelers who need to run WEPP from contemporary geospatial inputs. Its target users are hydrologists, erosion-model developers, post-fire risk analysts, watershed scientists, and maintainers of WEPP-based decision-support systems. The software is not a new erosion model and does not replace WEPP. Instead, it provides the terrain and network preprocessing layer needed to convert raster elevation and vector outlet information into WEPP-compatible watershed structure.

# State of the field

Several mature geospatial software systems support watershed delineation and hydrologic terrain analysis. WhiteboxTools was a broad Rust-based command-line and Python-accessible geospatial analysis platform with hydrologic, terrain, raster, stream-network, and GIS tools [@lindsay_whitebox_tools]; the repository has since been marked as legacy by its author, with no updates since February 2025, as development moved to a separate commercial and open-source successor. TauDEM, GRASS GIS, ArcGIS hydrology tools, and other GIS packages can derive flow directions, contributing area, stream networks, watersheds, and terrain attributes. These tools are highly capable for general hydrologic analysis, but they do not directly emit the TOPAZ-style left/right/top hillslope identifiers, WEPP channel metadata tables, WEPP-compatible topology constraints, and operational diagnostics required by WEPPcloud.

The main design choice was therefore to build within WhiteboxTools rather than create a standalone preprocessing codebase. WhiteboxTools already provides a performant raster-processing framework, command-line interface, Python API (CLI wrapper), and a large collection of hydrologic tools. Extending that ecosystem avoids reimplementing general-purpose raster I/O, flow-direction handling, stream extraction, watershed delineation, and terrain analysis. The contribution of `weppcloud-wbt` is the WEPP/TOPAZ-specific layer: enforcing model-consumer topology requirements, emitting WEPPcloud sidecar tables and rasters, supporting interactive outlet selection, preserving reproducible diagnostics, and adapting the workflow to GeoTIFF, GeoJSON, VRT, and Python-driven automation. A fork was used rather than a patch-set because the required changes are architectural rather than additive: structured Rust error propagation to Python callers, deliberate removal of build-time environment variables from the compiled binary to prevent server-side stacktrace exposure, and WEPP-specific tool semantics that have no place in a general-purpose platform.

# Software design

`weppcloud-wbt` follows a tool-oriented design consistent with WhiteboxTools. Each operation is implemented as a command-line tool with explicit inputs and outputs, and Python bindings expose the same functionality to scripted workflows. The design emphasizes deterministic raster products, auditable intermediate files, and metadata sidecars rather than hidden in-memory state. This is important for WEPPcloud because watershed preparation is often run asynchronously, on user-provided DEMs, and under failure modes that must be diagnosable after the fact.

The central tool, `HillslopesTopaz`, implements TOPAZ-style stream and hillslope identifiers for a single watershed. It emits the rasters and channel metadata tables used by WEPPcloud, including left, right, and top hillslope classes and link-level attributes such as upstream area. Related tools provide outlet discovery (`FindOutlet`), stream-junction counting (`StreamJunctionIdentifier`), Strahler-order pruning (`PruneStrahlerStreamOrder`), iterative first-order-link pruning with local thresholds (`IterativeFirstOrderLinkPrune`), and enhanced short-stream pruning with a maximum-junction constraint. Together these tools make channel-network construction explicit enough to satisfy WEPP's consumer-side limits while retaining hydrologic traceability.

The legacy `Hillslopes` tool in the original WhiteboxTools repository has a known defect in its diagonal hillslope clumping phase. The tool assigns each non-stream cell to a hillslope unit using a lateral flood-fill that is supposed to prevent clumping across stream channels at diagonal connections. The guard condition for diagonal neighbors uses a logical OR where AND is required: it allows two cells to merge diagonally whenever at least one of the two adjacent cardinal cells is stream-free, rather than requiring that both are stream-free. The result is that cells on opposite sides of a stream channel can be incorrectly grouped into the same hillslope unit wherever the channel is diagonal. `HillslopesTopaz` avoids this class of error by replacing lateral flood-fill clumping with D8 flow-path tracing: each unlabeled cell follows the D8 pointer chain downstream until it reaches a cell that has already been assigned a TOPAZ hillslope identifier and inherits that identifier. Because no lateral neighbor comparisons are made, diagonal stream crossings cannot produce incorrect groupings.

![TOPAZ-style hillslope and channel delineation for an example watershed produced by `weppcloud-wbt`. Orange-bounded areas are numbered hillslope units assigned to their parent channel link; blue raster cells show the extracted channel network. Hillslope identifiers encode the parent link (e.g., identifiers 131, 132, and 133 are the left, right, and top hillslopes of link 13). Scale bar indicates 200 m.](figures/wbt-watershed_optimized.png)

Several tools handle terrain and infrastructure details that matter for erosion modeling but are awkward to express in generic GIS workflows. `FVSlope` computes slope in the D8 flow direction to match TOPAZ-style flow-vector slopes used by WEPP channel hydraulics where `Slope` produces biased estimates for channels. The fork's `FVSlope` implementation adds ratio units and records the selected unit in output metadata. `RaiseRoads` conditions DEMs for road embankments while guaranteeing that valid DEM cells are not lowered, supporting constant, profile-relative, and cross-section strategies with attribute-based GeoJSON overrides. `ClipRasterToRaster`, GeoJSON support in `Watershed`, basin hierarchy export in `UnnestBasins`, and read-only single-source VRT support reduce unnecessary format conversions and full-raster reads in cloud workflows.

The Python wrapper and runtime changes are part of the research software design rather than incidental plumbing. WEPPcloud executes preprocessing through the `WhiteboxToolsTopazEmulator` adapter in the companion WEPPpy codebase, which orchestrates WhiteboxTools calls, manages intermediate products, snaps outlets, and exposes build steps to the larger WEPPcloud application. Propagating Rust errors, supporting `raise_on_error`, preserving environment configuration, and returning richer diagnostics make failures reproducible and debuggable in automated modeling pipelines.

![WEPPcloud channel delineation panel exposing `weppcloud-wbt` preprocessing parameters. Users specify minimum channel length, critical source area, stream-pruning method (shown: `IterativeFirstOrderLinkPrune`), and DEM conditioning strategy. Submitting the form invokes the `WhiteboxToolsTopazEmulator` adapter, which orchestrates the tool pipeline and streams progress back to the browser.](figures/channel-delineation-screenshot.png)

# Research impact statement

`weppcloud-wbt` is the terrain-processing and watershed-topology engine used by WEPPcloud to prepare model-ready watershed inputs for WEPP simulations [@lew_etal_2022_weppcloud_part_i; @lew_2026_wepppy]. A companion Journal of Hydrology study used WEPPcloud to model 28 forested watersheds across the western United States and compared simulated streamflow, sediment, and phosphorus against observations with minimal or no calibration [@dobre_etal_2022_weppcloud_part_ii]. The associated open data publication archived full WEPPcloud model runs, including raw and processed input/output files, tables, shapefiles, and reproducible watershed configurations [@dobre_etal_2022_weppcloud_datasets].

`weppcloud-wbt` provides the reproducible terrain abstraction layer that translates user-selected outlets and contemporary geospatial rasters into the channel, hillslope, slope, watershed-boundary, and network-table products consumed by WEPP. These steps are scientifically consequential: errors in outlet placement, stream pruning, channel topology, hillslope assignment, or watershed clipping alter the spatial structure of the hydrologic model before any erosion equations are evaluated. The repository includes regression tests, synthetic fixtures, tool specifications, and reviewer-facing evidence matrices that allow independent evaluation of preprocessing behavior.

# AI usage disclosure

Generative AI tools, including ChatGPT, Claude, and Codex, were used to assist with software development, code review, documentation drafting, error-message cleanup, and preparation of this JOSS paper draft. AI assistance was used for scaffolding, refactoring suggestions, literature and citation organization, and prose drafting. The human author made the core design decisions, reviewed and edited AI-assisted outputs, tested behavior against WEPPcloud requirements, inspected generated code, and remains responsible for the accuracy, licensing, correctness, and maintainability of the submitted software and manuscript.

# Acknowledgements

This work builds on WhiteboxTools by John Lindsay and the TOPAZ digital landscape analysis system developed by Jurgen Garbrecht and Lawrence Martz. The author acknowledges the long-running WEPP, TOPAZ, and open-source geospatial software communities whose models, algorithms, and tools made this work possible.

# References
