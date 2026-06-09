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

`weppcloud-wbt` (<https://github.com/rogerlew/weppcloud-wbt>) is a research software fork of WhiteboxTools that adds watershed-topology and terrain-processing tools needed by the Water Erosion Prediction Project (WEPP) and WEPPcloud workflows. WEPP is a process-based soil erosion and sediment-delivery model used to simulate runoff, erosion, deposition, and sediment yield from hillslopes and small watersheds [@flanagan_nearing_1995_wepp]. To run WEPP at watershed scale, a digital elevation model must be transformed into model-ready representations of channels, hillslopes, outlets, slopes, watershed masks, and channel-network metadata. `weppcloud-wbt` provides these preprocessing steps in a modern Rust geospatial analysis framework while preserving the useful conceptual structure of the TOPAZ digital landscape analysis system [@garbrecht_martz_2004_topaz_overview_manual; @martz_garbrecht_1992_drainage_network_dem; @martz_garbrecht_1993_automated_extraction_dem].

The software adds TOPAZ-style hillslope and channel identifiers, outlet discovery, stream-junction diagnostics, stream-network pruning, flow-vector slope calculation, raster clipping, GeoJSON pour-point watershed support, road-embankment conditioning, basin hierarchy export, improved runtime error propagation, Python wrapper enhancements, and limited read-only VRT support. These tools are used through WEPPcloud and its Python adapter, `WhiteboxToolsTopazEmulator`, which exposes the WhiteboxTools-based preprocessing pipeline to the larger WEPPcloud application. The result is a reproducible bridge between legacy hydrologic terrain parameterization and contemporary raster, GeoJSON, and Python-driven cloud workflows.

# Statement of need

TOPAZ remains an important and durable conceptual model for hydrologic terrain analysis. It formalized automated drainage-network extraction, watershed segmentation, subcatchment parameterization, flat-area treatment, depression handling, and raster channel topology in ways that remain useful for erosion modeling and other watershed applications [@garbrecht_martz_1997_flat_surfaces; @garbrecht_martz_1997_channel_ordering; @martz_garbrecht_1998_flats_depressions]. However, the operational needs of current WEPPcloud workflows expose limitations in the legacy toolchain. Traditional TOPAZ workflows rely on ASCII-era file formats and flat-file serialization, assume the target watershed is completely contained inside the DEM boundary, and produce channel-network topologies with greater than 3 channel inflows that can conflict with WEPP's downstream consumer constraints. The legacy source is also fixed-form Fortran 77, which makes targeted modification, testing, instrumentation, and integration into modern cloud services difficult.

`weppcloud-wbt` addresses these constraints for researchers and operational modelers who need to run WEPP from contemporary geospatial inputs. Its target users are hydrologists, erosion-model developers, post-fire risk analysts, watershed scientists, and maintainers of WEPP-based decision-support systems. The software is not a new erosion model and does not replace WEPP. Instead, it provides the terrain and network preprocessing layer needed to convert raster elevation and vector outlet information into WEPP-compatible watershed structure.

# State of the field

Several mature geospatial software systems support watershed delineation and hydrologic terrain analysis. WhiteboxTools was a broad Rust-based command-line and Python-accessible geospatial analysis platform with hydrologic, terrain, raster, stream-network, and GIS tools [@lindsay_whitebox_tools]; the repository has since been marked as legacy by its author, with no updates since February 2025, as development moved to a separate commercial and open-source successor. TauDEM, GRASS GIS, ArcGIS hydrology tools, and other GIS packages can derive flow directions, contributing area, stream networks, watersheds, and terrain attributes. These tools are highly capable for general hydrologic analysis, but they do not directly emit the TOPAZ-style left/right/top hillslope identifiers, WEPP channel metadata tables, WEPP-compatible topology constraints, and operational diagnostics required by WEPPcloud.

The main design choice was therefore to build within WhiteboxTools rather than create a standalone preprocessing codebase. WhiteboxTools already provides a performant raster-processing framework, command-line interface, Python API (CLI wrapper), and a large collection of hydrologic tools. Extending that ecosystem avoids reimplementing general-purpose raster I/O, flow-direction handling, stream extraction, watershed delineation, and terrain analysis. The contribution of `weppcloud-wbt` is the WEPP/TOPAZ-specific layer: enforcing model-consumer topology requirements, emitting WEPPcloud sidecar tables and rasters, supporting interactive outlet selection, preserving reproducible diagnostics, and adapting the workflow to GeoTIFF, GeoJSON, VRT, and Python-driven automation. A fork was used rather than a patch-set because the required changes are architectural rather than additive: structured Rust error propagation to Python callers, deliberate removal of build-time environment variables from the runtime binary to prevent server-side stacktrace exposure in production cloud contexts, and WEPP-specific tool semantics that have no place in a general-purpose platform. Upstreaming is not a viable path: the original WhiteboxTools repository has been marked as legacy by its author, who has transitioned development to a new commercial and open-source product, and has received no commits since February 2025. `weppcloud-wbt` is therefore the actively maintained branch of the WhiteboxTools codebase for WEPP and WEPPcloud workflows.

# Software design

`weppcloud-wbt` follows a tool-oriented design consistent with WhiteboxTools. Each operation is implemented as a command-line tool with explicit inputs and outputs, and Python bindings expose the same functionality to scripted workflows. The design emphasizes deterministic raster products, auditable intermediate files, and metadata sidecars rather than hidden in-memory state. This is important for WEPPcloud because watershed preparation is often run asynchronously, on user-provided DEMs, and under failure modes that must be diagnosable after the fact.

The central tool, `HillslopesTopaz`, implements TOPAZ-style stream and hillslope identifiers for a single watershed. It emits the rasters and channel metadata tables used by WEPPcloud, including left, right, and top hillslope classes and link-level attributes such as upstream area. Related tools provide outlet discovery (`FindOutlet`), stream-junction counting (`StreamJunctionIdentifier`), Strahler-order pruning (`PruneStrahlerStreamOrder`), iterative first-order-link pruning with local thresholds (`IterativeFirstOrderLinkPrune`), and enhanced short-stream pruning with a maximum-junction constraint. Together these tools make channel-network construction explicit enough to satisfy WEPP's consumer-side limits while retaining hydrologic traceability.

Several tools handle terrain and infrastructure details that matter for erosion modeling but are awkward to express in generic GIS workflows. `FVSlope` computes slope in the D8 flow direction to match TOPAZ-style flow-vector slopes used by WEPP channel hydraulics where `Slope` produces biased estimates for channels. The modified `FVSlope` tool adds ratio units and records the selected unit in output metadata. `RaiseRoads` conditions DEMs for road embankments while guaranteeing that valid DEM cells are not lowered, supporting constant, profile-relative, and cross-section strategies with attribute-based GeoJSON overrides. `ClipRasterToRaster`, GeoJSON support in `Watershed`, basin hierarchy export in `UnnestBasins`, and read-only single-source VRT support reduce unnecessary format conversions and full-raster reads in cloud workflows.

The Python wrapper and runtime changes are part of the research software design rather than incidental plumbing. WEPPcloud executes preprocessing through the `WhiteboxToolsTopazEmulator` adapter, which orchestrates WhiteboxTools calls, manages intermediate products, snaps outlets, and exposes build steps to the larger WEPPcloud application. Propagating Rust errors, supporting `raise_on_error`, preserving environment configuration, and returning richer diagnostics make failures reproducible and debuggable in automated modeling pipelines.

# Research impact statement

`weppcloud-wbt` is used operationally within WEPPcloud [@lew_etal_2022_weppcloud_part_i; @lew_2026_wepppy] to prepare watershed inputs for WEPP simulations. WEPPcloud allows users to run WEPP-based erosion and watershed analyses without manually assembling all model input files. The tools described here provide the terrain-processing layer that converts DEMs, requested outlets, stream thresholds, road layers, and watershed masks into the hillslope/channel structures consumed by WEPP. This integration is a concrete research impact because it enables reproducible WEPP watershed simulations from contemporary geospatial data rather than requiring manual TOPAZ-era preprocessing.

The software also has credible near-term significance beyond one deployment. It preserves the practical value of TOPAZ-style parameterization while making the workflow inspectable, testable, and extensible in Rust and Python. The repository includes test fixtures, regression tests for selected mapping behavior, documented specifications for pruning logic, and deployment notes for WEPPcloud cutovers. These materials support independent review of the terrain-processing algorithms and provide a path for other WEPP, post-fire hydrology, and watershed-modeling groups to reproduce or adapt the workflow.

# AI usage disclosure

Generative AI tools, including ChatGPT, Claude, and Codex, were used to assist with software development, code review, documentation drafting, error-message cleanup, and preparation of this JOSS paper draft. AI assistance was used for scaffolding, refactoring suggestions, literature and citation organization, and prose drafting. The human author made the core design decisions, reviewed and edited AI-assisted outputs, tested behavior against WEPPcloud requirements, inspected generated code, and remains responsible for the accuracy, licensing, correctness, and maintainability of the submitted software and manuscript.

# Acknowledgements

This work builds on WhiteboxTools by John Lindsay and the TOPAZ digital landscape analysis system developed by Jurgen Garbrecht and Lawrence Martz. The author acknowledges the long-running WEPP, TOPAZ, and open-source geospatial software communities whose models, algorithms, and tools made this work possible.

# References

