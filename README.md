# https://github.com/rogerlew/whitebox-tools

This is a fork of John Lindsay's WhiteBoxTools. 

This fork diverges from the upstream WhiteboxTools distribution in the following ways (all used operationally within WEPPcloud workflows):

- `HillslopesTopaz` (hydro_analysis/hillslopes_topaz.rs)
  - Implements Garbrecht & Martz TOPAZ-style stream and hillslope identifiers for a single watershed, emitting channel metadata tables (`netw.tsv`, `netw_props.tsv`) and left/right/top hillslope rasters needed by WEPPcloud.
  - Includes numerous performance optimizations (e.g., combined flood-fill phases, cached upstream areas) and additional output attributes such as `areaup` for each link.
- `FindOutlet` (hydro_analysis/find_outlet.rs)
  - Derives a single stream outlet pour point GeoJSON by tracing D8 flow from interior candidates, embedding diagnostics needed by downstream WEPPcloud steps.
  - Supports optional watershed masks and requested start locations (`--requested_outlet_lng_lat`, `--requested_outlet_row_col`) so interactive callers can walk downhill from arbitrary picks without bespoke Python search code.
- `StreamJunctionIdentifier` (stream_network_analysis/stream_junctions.rs)
  - Counts inflowing tributaries for every stream pixel, producing junction maps that WEPPcloud uses to locate confluences, outlets, and pseudo-gauges.
- `PruneStrahlerStreamOrder` (stream_network_analysis/prune_strahler_order.rs)
  - Drops first-order (Strahler order = 1) links from an existing order grid, subtracts one from downstream orders, and optionally preserves zero-valued background cells.
  - Exposed through new Python bindings (`whitebox_tools.py` and `WBT/whitebox_tools.py`).
- `ClipRasterToRaster` (gis_analysis/clip_raster_to_raster.rs) adds raster-on-raster clipping with a corresponding Python wrapper.
- `RemoveShortStreams` enhancement (stream_network_analysis/remove_short_streams.rs)
  - Adds `--max_junctions` pruning with iterative branch deletion so no junction retains more than the requested inflows; Python API updated with the new argument.
- `Slope` tool modification (terrain_analysis/slope.rs) introducing ratio units and recording the chosen unit in output metadata; banner text updated to reflect maintenance through 2025.
- `Watershed` tool update (hydro_analysis/watershed.rs)
  - Accepts GeoJSON pour-point inputs (Point/MultiPoint) in addition to shapefiles and rasters, pulling in the `geojson` crate and documenting the extended behaviour.
- CLI/runtime updates
  - Command-line entry point now propagates errors (`main.rs` returns `Result`), enabling backtraces from scripted environments.
  - Python wrapper enhancements provide optional `raise_on_error` semantics, custom exceptions, environment propagation, and richer error reporting for all tools.
- General code cleanup by CODEX ( gpt-5-codex high unprompted :| )
  - Tightened tool documentation, aligned specs/readmes with new diagnostics, and refreshed error messaging to keep automated workflows resilient.


Developers extending this fork can follow the guidelines in [DEVELOPING_TOOLS.md](DEVELOPING_TOOLS.md).



![](./img/WhiteboxToolsLogoBlue.png)


> Note: Compiled WhiteboxTools binaries for Windows, macOS, and Linux can be found at: https://www.whiteboxgeo.com/download-whiteboxtools/

*This page is related to the stand-alone command-line program and Python scripting API for geospatial analysis, **WhiteboxTools**.

The official WhiteboxTools User Manual can be found [at this link](https://whiteboxgeo.com/manual/wbt_book/preface.html).

**Contents**

1. [Description](#1-description)
2. [Getting Help](#2-getting-help)
3. [Downloads and Installation](#3-pre-compiled-binaries)
4. [Building From Source Code](#4-building-from-source-code)

## 1 Description

**WhiteboxTools** is an advanced geospatial data analysis platform developed by Prof. John Lindsay ([webpage](http://www.uoguelph.ca/~hydrogeo/index.html); [jblindsay](https://github.com/jblindsay)) at the [University of Guelph's](http://www.uoguelph.ca) [*Geomorphometry and Hydrogeomatics Research Group*](http://www.uoguelph.ca/~hydrogeo/index.html). *WhiteboxTools* can be used to perform common geographical information systems (GIS) analysis operations, such as cost-distance analysis, distance buffering, and raster reclassification. Remote sensing and image processing tasks include image enhancement (e.g. panchromatic sharpening, contrast adjustments), image mosaicing, numerous filtering operations, classification, and common image transformations. *WhiteboxTools* also contains advanced tooling for spatial hydrological analysis (e.g. flow-accumulation, watershed delineation, stream network analysis, sink removal), terrain analysis (e.g. common terrain indices such as slope, curvatures, wetness index, hillshading; hypsometric analysis; multi-scale topographic position analysis), and LiDAR data processing. LiDAR point clouds can be interrogated (LidarInfo, LidarHistogram), segmented, tiled and joined, analyized for outliers, interpolated to rasters (DEMs, intensity images), and ground-points can be classified or filtered. *WhiteboxTools* is not a cartographic or spatial data visualization package; instead it is meant to serve as an analytical backend for other data visualization software, mainly GIS.

## 2 Getting help

WhiteboxToos possesses extensive help documentation. Users are referred to the [User Manual](https://www.whiteboxgeo.com/manual/wbt_book/) located on www.whiteboxgeo.com.

## 3 Pre-compiled binaries

*WhiteboxTools* is a stand-alone executable command-line program with no actual installation. If you intend to use the Python programming interface for *WhiteboxTools* you will need to have Python 3 (or higher) installed. Pre-compiled binaries can be downloaded from the [*Whitebox Geospatial Inc. website*](https://www.whiteboxgeo.com/download-whiteboxtools/) with support for various operating systems.

## 4 Building from source code

It is likely that *WhiteboxTools* will work on a wider variety of operating systems and architectures than the distributed binary files. If you do not find your operating system/architecture in the list of available *WhiteboxTool* binaries, then compilation from source code will be necessary. WhiteboxTools can be compiled from the source code with the following steps:

1. Install the Rust compiler; Rustup is recommended for this purpose. Further instruction can be found at this [link](https://www.rust-lang.org/en-US/install.html).

2. Download the *WhiteboxTools* from this GitHub repo.
```

3. Decompress the zipped download file.

4. Open a terminal (command prompt) window and change the working directory to the `whitebox-tools` folder:

```
>> cd /path/to/folder/whitebox-tools/
```

5. Finally, use the Python build.py script to compile the code:

```
>> python build.py
```

Read the notes in the `build.py` file for detailed information about customizing the build. In particular, the `do_clean`,
`exclude_runner` and `zip` arguments can be used to add or remove functionality during the build process. Running the build
script requires a Python environment. (Note, WhiteboxTools itself is pure Rust code.)

Depending on your system, the compilation may take several minutes. Also depending on your system, it may be necessary to use the `python3` command instead. When completed, the script will have created a new `WBT` folder within `whitebox-tools`. This folder will contain all of the files needed to run the program, including the main Whitebox executable file (whitebox_tools.exe), the Whitebox Runner GUI application, and the various plugins.

This repository tracks the generated `WBT` build compiled on Linux (Ubuntu 24.04) so the deployment artifacts remain versioned alongside the code.

Be sure to follow the instructions for installing Rust carefully. In particular, if you are installing on MS Windows, you must have a linker installed prior to installing the Rust compiler (rustc). The Rust webpage recommends either the **MS Visual C++ 2015 Build Tools** or the GNU equivalent and offers details for each installation approach. You should also consider using **RustUp** to install the Rust compiler.
