/*
This tool is part of the WhiteboxTools geospatial analysis library.
Author: Dr. Roger Lew
Created: 09/09/2025
License: MIT
*/

use crate::tools::*;
use geojson::{Feature, FeatureCollection, GeoJson, Geometry, Value as GeoValue};
use serde_json::{json, Map as JsonMap, Value as JsonValue};
use std::collections::{HashSet, VecDeque};
use std::env;
use std::f64;
use std::fs::File;
use std::io::{Error, ErrorKind, Write};
use std::path;
use std::time::Instant;
use whitebox_common::structures::Array2D;
use whitebox_common::utils::get_formatted_elapsed_time;
use whitebox_raster::*;

pub struct FindOutlet {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl FindOutlet {
    pub fn new() -> FindOutlet {
        let name = "FindOutlet".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Identifies the primary outlet for a watershed mask using D8 flow directions and stream network.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input D8 Pointer File".to_owned(),
            flags: vec!["--d8_pntr".to_owned()],
            description: "Input raster D8 pointer file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Streams File".to_owned(),
            flags: vec!["--streams".to_owned()],
            description: "Input raster streams file (1=stream, 0=non-stream).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input Watershed Mask File".to_owned(),
            flags: vec!["--watershed".to_owned()],
            description:
                "Optional watershed mask raster file (1=inside, 0=outside). Required unless a requested outlet location is supplied.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Requested Outlet Longitude/Latitude".to_owned(),
            flags: vec!["--requested_outlet_lng_lat".to_owned()],
            description: "Optional requested outlet location specified as 'lon,lat' (WGS84)."
                .to_owned(),
            parameter_type: ParameterType::String,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Requested Outlet Row/Column".to_owned(),
            flags: vec!["--requested_outlet_row_col".to_owned()],
            description: "Optional requested outlet specified as 'row,col' in raster coordinates."
                .to_owned(),
            parameter_type: ParameterType::String,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output Pour Point GeoJSON File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output GeoJSON file containing the identified outlet point.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Vector(
                VectorGeometryType::Point,
            )),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Pointer Uses ESRI Style".to_owned(),
            flags: vec!["--esri_pntr".to_owned()],
            description: "Specify if the input D8 pointer uses the ESRI style scheme.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_string()),
            optional: true,
        });

        let sep: String = path::MAIN_SEPARATOR.to_string();
        let e = format!("{}", env::current_exe().unwrap().display());
        let mut parent = env::current_exe().unwrap();
        parent.pop();
        let p = format!("{}", parent.display());
        let mut short_exe = e
            .replace(&p, "")
            .replace(".exe", "")
            .replace(".", "")
            .replace(&sep, "");
        if e.contains(".exe") {
            short_exe += ".exe";
        }
        let usage = format!(
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --d8_pntr='d8pntr.tif' --streams='streams.tif' --watershed='ws.tif' --requested_outlet_lng_lat='-120.5,42.1' --output='outlet.geojson'",
            short_exe, name
        )
        .replace("*", &sep);

        FindOutlet {
            name,
            description,
            toolbox,
            parameters,
            example_usage: usage,
        }
    }
}

#[derive(Copy, Clone)]
enum TraceStartMode {
    Requested,
    WatershedCandidate,
}

impl TraceStartMode {
    fn as_str(&self) -> &'static str {
        match self {
            TraceStartMode::Requested => "requested",
            TraceStartMode::WatershedCandidate => "watershed",
        }
    }
}

struct TraceContext<'a> {
    pntr: &'a Raster,
    streams: &'a Raster,
    mask: Option<&'a Array2D<u8>>,
    junction_counts: &'a Array2D<i16>,
    pntr_nodata: f64,
    streams_nodata: f64,
    pntr_matches: &'a [i8; 129],
    dx: &'a [isize; 8],
    dy: &'a [isize; 8],
    rows: isize,
    columns: isize,
    max_steps: usize,
}

struct TraceParams<'a> {
    label: &'a str,
    mode: TraceStartMode,
}

#[derive(Copy, Clone)]
struct TraceSuccessData {
    outlet_row: isize,
    outlet_col: isize,
    steps_taken: usize,
    steps_beyond_mask: usize,
    outlet_downstream: bool,
    outlet_junction_count: i16,
}

struct TraceFailureData {
    reason: String,
    last_junction: Option<(isize, isize, i16)>,
}

struct SelectedTrace {
    success: TraceSuccessData,
    start_row: isize,
    start_col: isize,
    start_mode: TraceStartMode,
    distance_to_boundary: i32,
    candidate_rank: Option<usize>,
    start_offset_cells: usize,
}

fn trace_flow_path(
    mut row: isize,
    mut col: isize,
    ctx: &TraceContext,
    params: &TraceParams,
) -> Result<TraceSuccessData, TraceFailureData> {
    if row < 0 || row >= ctx.rows || col < 0 || col >= ctx.columns {
        return Err(TraceFailureData {
            reason: format!(
                "{}: start cell ({}, {}) lies outside raster bounds.",
                params.label, row, col
            ),
            last_junction: None,
        });
    }

    let mut visited: HashSet<(isize, isize)> = HashSet::new();
    let mut steps: usize = 0;
    let mut has_left_mask = false;
    let mut steps_beyond_mask: usize = 0;
    let mut last_junction_mismatch: Option<(isize, isize, i16)> = None;

    loop {
        if !visited.insert((row, col)) {
            return Err(TraceFailureData {
                reason: format!(
                    "{}: flow path loops near row {}, col {}.",
                    params.label, row, col
                ),
                last_junction: last_junction_mismatch,
            });
        }

        if let Some(mask) = ctx.mask {
            if mask.get_value(row, col) == 0u8 {
                has_left_mask = true;
            }
        }

        let stream_val = ctx.streams[(row, col)];
        let (is_stream, junction_count) = if stream_val != ctx.streams_nodata && stream_val > 0f64 {
            let junction = ctx.junction_counts.get_value(row, col);
            (true, junction)
        } else {
            (false, -1i16)
        };

        let outlet_downstream_now = ctx
            .mask
            .map(|mask| mask.get_value(row, col) == 0u8)
            .unwrap_or(false);

        let accept_current = match params.mode {
            TraceStartMode::Requested => is_stream && junction_count == 1,
            TraceStartMode::WatershedCandidate => has_left_mask && is_stream && junction_count == 1,
        };

        if accept_current {
            return Ok(TraceSuccessData {
                outlet_row: row,
                outlet_col: col,
                steps_taken: steps,
                steps_beyond_mask,
                outlet_downstream: outlet_downstream_now,
                outlet_junction_count: junction_count,
            });
        } else if is_stream && junction_count != 1 {
            last_junction_mismatch = Some((row, col, junction_count));
        }

        let pointer = ctx.pntr[(row, col)];
        if pointer == ctx.pntr_nodata || pointer <= 0f64 {
            let reason = if ctx.mask.is_some() && has_left_mask {
                format!(
                    "{}: downstream pointer becomes invalid ({}) near row {}, col {}.",
                    params.label, pointer, row, col
                )
            } else {
                format!(
                    "{}: encountered invalid D8 pointer ({}) at row {}, col {}.",
                    params.label, pointer, row, col
                )
            };
            return Err(TraceFailureData {
                reason,
                last_junction: last_junction_mismatch,
            });
        }

        let pointer_index = pointer.round() as usize;
        if pointer_index >= ctx.pntr_matches.len() {
            let reason = if ctx.mask.is_some() && has_left_mask {
                format!(
                    "{}: downstream pointer value {} out of range near row {}, col {}.",
                    params.label, pointer, row, col
                )
            } else {
                format!(
                    "{}: pointer value {} out of range at row {}, col {}.",
                    params.label, pointer, row, col
                )
            };
            return Err(TraceFailureData {
                reason,
                last_junction: last_junction_mismatch,
            });
        }

        let dir = ctx.pntr_matches[pointer_index];
        if dir < 0 {
            let reason = if ctx.mask.is_some() && has_left_mask {
                format!(
                    "{}: downstream pointer value {} unsupported near row {}, col {}.",
                    params.label, pointer, row, col
                )
            } else {
                format!(
                    "{}: unsupported pointer value {} at row {}, col {}.",
                    params.label, pointer, row, col
                )
            };
            return Err(TraceFailureData {
                reason,
                last_junction: last_junction_mismatch,
            });
        }

        let nr = row + ctx.dy[dir as usize];
        let nc = col + ctx.dx[dir as usize];

        steps += 1;

        if nr < 0 || nr >= ctx.rows || nc < 0 || nc >= ctx.columns {
            if is_stream && junction_count == 1 {
                return Ok(TraceSuccessData {
                    outlet_row: row,
                    outlet_col: col,
                    steps_taken: steps,
                    steps_beyond_mask,
                    outlet_downstream: outlet_downstream_now || has_left_mask,
                    outlet_junction_count: junction_count,
                });
            }
            let reason = if is_stream {
                format!(
                    "{}: reached raster edge at row {}, col {} with junction count {} (expected 1).",
                    params.label, row, col, junction_count
                )
            } else {
                format!(
                    "{}: exited raster at row {}, col {} without hitting a stream.",
                    params.label, row, col
                )
            };
            return Err(TraceFailureData {
                reason,
                last_junction: last_junction_mismatch,
            });
        }

        if let Some(mask) = ctx.mask {
            if mask.get_value(row, col) == 1u8 && mask.get_value(nr, nc) == 0u8 {
                if is_stream && junction_count == 1 {
                    return Ok(TraceSuccessData {
                        outlet_row: row,
                        outlet_col: col,
                        steps_taken: steps,
                        steps_beyond_mask,
                        outlet_downstream: false,
                        outlet_junction_count: junction_count,
                    });
                }
                if is_stream && junction_count != 1 {
                    last_junction_mismatch = Some((row, col, junction_count));
                }
                has_left_mask = true;
            }
        }

        row = nr;
        col = nc;

        if ctx
            .mask
            .map(|mask| mask.get_value(row, col) == 0u8)
            .unwrap_or(false)
        {
            has_left_mask = true;
            steps_beyond_mask += 1;
        }

        let stream_val = ctx.streams[(row, col)];
        if stream_val != ctx.streams_nodata && stream_val > 0f64 {
            let junction = ctx.junction_counts.get_value(row, col);
            if junction == 1 && (matches!(params.mode, TraceStartMode::Requested) || has_left_mask)
            {
                let downstream = ctx
                    .mask
                    .map(|mask| mask.get_value(row, col) == 0u8)
                    .unwrap_or(has_left_mask);
                return Ok(TraceSuccessData {
                    outlet_row: row,
                    outlet_col: col,
                    steps_taken: steps,
                    steps_beyond_mask,
                    outlet_downstream: downstream,
                    outlet_junction_count: junction,
                });
            } else if junction != 1 {
                last_junction_mismatch = Some((row, col, junction));
            }
        }

        if steps >= ctx.max_steps {
            let reason = if ctx.mask.is_some() {
                if has_left_mask {
                    format!(
                        "{}: exceeded maximum step count ({}) while searching downstream of the watershed.",
                        params.label, ctx.max_steps
                    )
                } else {
                    format!(
                        "{}: exceeded maximum step count ({}) before exiting watershed.",
                        params.label, ctx.max_steps
                    )
                }
            } else {
                format!(
                    "{}: exceeded maximum step count ({}) while traversing flow path.",
                    params.label, ctx.max_steps
                )
            };
            return Err(TraceFailureData {
                reason,
                last_junction: last_junction_mismatch,
            });
        }
    }
}

fn find_nearest_valid_cell(
    row: isize,
    col: isize,
    rows: isize,
    columns: isize,
    pntr: &Raster,
    pntr_nodata: f64,
    pntr_matches: &[i8; 129],
    dx: &[isize; 8],
    dy: &[isize; 8],
) -> Option<((isize, isize), usize)> {
    let mut start_row = clamp_index(row, rows - 1);
    let mut start_col = clamp_index(col, columns - 1);
    if start_row < 0 {
        start_row = 0;
    }
    if start_col < 0 {
        start_col = 0;
    }

    let mut queue: VecDeque<(isize, isize, usize)> = VecDeque::new();
    let mut visited: HashSet<(isize, isize)> = HashSet::new();
    queue.push_back((start_row, start_col, 0));
    visited.insert((start_row, start_col));

    while let Some((r, c, dist)) = queue.pop_front() {
        let pointer = pntr[(r, c)];
        if pointer != pntr_nodata && pointer > 0f64 {
            let idx = pointer.round() as usize;
            if idx < pntr_matches.len() && pntr_matches[idx] >= 0 {
                return Some(((r, c), dist));
            }
        }

        for n in 0..8 {
            let nr = r + dy[n];
            let nc = c + dx[n];
            if nr >= 0 && nr < rows && nc >= 0 && nc < columns {
                if visited.insert((nr, nc)) {
                    queue.push_back((nr, nc, dist + 1));
                }
            }
        }

        if visited.len() > (rows * columns) as usize {
            break;
        }
    }

    None
}

fn clamp_index(value: isize, max: isize) -> isize {
    if value < 0 {
        0
    } else if value > max {
        max
    } else {
        value
    }
}

fn lon_lat_to_row_col(pntr: &Raster, lon: f64, lat: f64) -> Option<(isize, isize)> {
    let epsg = pntr.configs.epsg_code;
    if epsg == 0 {
        return None;
    }
    if epsg == 4326 {
        let col = ((lon - pntr.configs.west) / pntr.configs.resolution_x).round() as isize;
        let row = ((pntr.configs.north - lat) / pntr.configs.resolution_y).round() as isize;
        return Some((
            clamp_index(row, pntr.configs.rows as isize - 1),
            clamp_index(col, pntr.configs.columns as isize - 1),
        ));
    }
    None
}

impl WhiteboxTool for FindOutlet {
    fn get_source_file(&self) -> String {
        String::from(file!())
    }

    fn get_tool_name(&self) -> String {
        self.name.clone()
    }

    fn get_tool_description(&self) -> String {
        self.description.clone()
    }

    fn get_tool_parameters(&self) -> String {
        match serde_json::to_string(&self.parameters) {
            Ok(json_str) => format!("{{\"parameters\":{}}}", json_str),
            Err(err) => format!("{:?}", err),
        }
    }

    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }

    fn get_toolbox(&self) -> String {
        self.toolbox.clone()
    }

    fn run<'a>(
        &self,
        args: Vec<String>,
        working_directory: &'a str,
        verbose: bool,
    ) -> Result<(), Error> {
        let mut d8_file = String::new();
        let mut streams_file = String::new();
        let mut watershed_file = String::new();
        let mut output_file = String::new();
        let mut esri_style = false;
        let mut requested_lng_lat: Option<(f64, f64)> = None;
        let mut requested_row_col: Option<(isize, isize)> = None;

        if args.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no parameters.",
            ));
        }
        for i in 0..args.len() {
            let mut arg = args[i].replace('\"', "");
            arg = arg.replace("\'", "");
            let cmd = arg.split('=');
            let vec = cmd.collect::<Vec<&str>>();
            let keyval = vec.len() > 1;
            let flag = vec[0].to_lowercase();
            if flag == "-d8_pntr" || flag == "--d8_pntr" {
                d8_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag == "-streams" || flag == "--streams" {
                streams_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag == "-watershed" || flag == "--watershed" {
                watershed_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag == "-o" || flag == "--output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag == "--esri_pntr" || flag == "-esri_pntr" || flag == "--esri_style" {
                esri_style = true;
            } else if flag == "--requested_outlet_lng_lat" {
                let value = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
                if parts.len() != 2 {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!(
                            "--requested_outlet_lng_lat expects 'lon,lat'; received '{}'.",
                            value
                        ),
                    ));
                }
                let lon = parts[0].parse::<f64>().map_err(|_| {
                    Error::new(
                        ErrorKind::InvalidInput,
                        format!(
                            "Unable to parse longitude '{}' for --requested_outlet_lng_lat.",
                            parts[0]
                        ),
                    )
                })?;
                let lat = parts[1].parse::<f64>().map_err(|_| {
                    Error::new(
                        ErrorKind::InvalidInput,
                        format!(
                            "Unable to parse latitude '{}' for --requested_outlet_lng_lat.",
                            parts[1]
                        ),
                    )
                })?;
                requested_lng_lat = Some((lon, lat));
            } else if flag == "--requested_outlet_row_col" {
                let value = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                let parts: Vec<&str> = value.split(',').map(|s| s.trim()).collect();
                if parts.len() != 2 {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!(
                            "--requested_outlet_row_col expects 'row,col'; received '{}'.",
                            value
                        ),
                    ));
                }
                let row = parts[0].parse::<isize>().map_err(|_| {
                    Error::new(
                        ErrorKind::InvalidInput,
                        format!(
                            "Unable to parse row '{}' for --requested_outlet_row_col.",
                            parts[0]
                        ),
                    )
                })?;
                let col = parts[1].parse::<isize>().map_err(|_| {
                    Error::new(
                        ErrorKind::InvalidInput,
                        format!(
                            "Unable to parse column '{}' for --requested_outlet_row_col.",
                            parts[1]
                        ),
                    )
                })?;
                requested_row_col = Some((row, col));
            }
        }

        if d8_file.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Input D8 pointer raster (--d8_pntr) not specified.",
            ));
        }
        if streams_file.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Input streams raster (--streams) not specified.",
            ));
        }
        if watershed_file.is_empty() && requested_lng_lat.is_none() && requested_row_col.is_none() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Either --watershed must be supplied or a requested outlet location (--requested_outlet_lng_lat / --requested_outlet_row_col) must be provided.",
            ));
        }
        if output_file.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Output GeoJSON file (--output) not specified.",
            ));
        }

        if verbose {
            let tool_name = self.get_tool_name();
            let welcome_len = format!("* Welcome to {} *", tool_name).len().max(28);
            println!("{}", "*".repeat(welcome_len));
            println!(
                "* Welcome to {} {}*",
                tool_name,
                " ".repeat(welcome_len - 15 - tool_name.len())
            );
            println!(
                "* Powered by WhiteboxTools {}*",
                " ".repeat(welcome_len - 28)
            );
            println!("* www.whiteboxgeo.com {}*", " ".repeat(welcome_len - 23));
            println!("{}", "*".repeat(welcome_len));
        }

        let sep: String = path::MAIN_SEPARATOR.to_string();
        if !d8_file.contains(&sep) && !d8_file.contains('/') {
            d8_file = format!("{}{}", working_directory, d8_file);
        }
        if !streams_file.contains(&sep) && !streams_file.contains('/') {
            streams_file = format!("{}{}", working_directory, streams_file);
        }
        if !watershed_file.is_empty()
            && !watershed_file.contains(&sep)
            && !watershed_file.contains('/')
        {
            watershed_file = format!("{}{}", working_directory, watershed_file);
        }
        if !output_file.contains(&sep) && !output_file.contains('/') {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading input rasters...");
        }
        let start = Instant::now();
        let pntr = Raster::new(&d8_file, "r")?;
        let streams = Raster::new(&streams_file, "r")?;

        let rows = pntr.configs.rows as isize;
        let columns = pntr.configs.columns as isize;

        if streams.configs.rows as isize != rows || streams.configs.columns as isize != columns {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Streams raster must have the same dimensions as the D8 pointer raster.",
            ));
        }
        let mut watershed: Option<Raster> = None;
        if !watershed_file.is_empty() {
            let ws = Raster::new(&watershed_file, "r")?;
            if ws.configs.rows as isize != rows || ws.configs.columns as isize != columns {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Watershed raster must have the same dimensions as the D8 pointer raster.",
                ));
            }
            watershed = Some(ws);
        }

        let pntr_nodata = pntr.configs.nodata;
        let streams_nodata = streams.configs.nodata;

        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
        let mut inflowing_vals = [16f64, 32f64, 64f64, 128f64, 1f64, 2f64, 4f64, 8f64];
        if esri_style {
            inflowing_vals = [8f64, 16f64, 32f64, 64f64, 128f64, 1f64, 2f64, 4f64];
        }
        let mut pntr_matches: [i8; 129] = [0i8; 129];
        if !esri_style {
            pntr_matches[1] = 0i8;
            pntr_matches[2] = 1i8;
            pntr_matches[4] = 2i8;
            pntr_matches[8] = 3i8;
            pntr_matches[16] = 4i8;
            pntr_matches[32] = 5i8;
            pntr_matches[64] = 6i8;
            pntr_matches[128] = 7i8;
        } else {
            pntr_matches[1] = 1i8;
            pntr_matches[2] = 2i8;
            pntr_matches[4] = 3i8;
            pntr_matches[8] = 4i8;
            pntr_matches[16] = 5i8;
            pntr_matches[32] = 6i8;
            pntr_matches[64] = 7i8;
            pntr_matches[128] = 0i8;
        }

        if verbose {
            println!("Computing stream junction counts...");
        }
        let mut junction_counts: Array2D<i16> = Array2D::new(rows, columns, -1i16, -1i16)?;
        let mut progress: usize;
        let mut old_progress: usize = 1;
        for row in 0..rows {
            for col in 0..columns {
                let stream_val = streams[(row, col)];
                if stream_val != streams_nodata && stream_val > 0f64 {
                    let mut cnt = 0i16;
                    for n in 0..8 {
                        let nr = row + dy[n];
                        let nc = col + dx[n];
                        if nr >= 0 && nr < rows && nc >= 0 && nc < columns {
                            let neighbour_stream = streams[(nr, nc)];
                            if neighbour_stream != streams_nodata && neighbour_stream > 0f64 {
                                let neighbour_pointer = pntr[(nr, nc)];
                                if neighbour_pointer != pntr_nodata
                                    && neighbour_pointer == inflowing_vals[n]
                                {
                                    cnt += 1;
                                }
                            }
                        }
                    }
                    junction_counts.set_value(row, col, cnt);
                }
            }
            if verbose && rows > 1 {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Junction scan: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut mask: Array2D<u8> = Array2D::new(rows, columns, 0u8, 0u8)?;
        let mut distances: Array2D<i32> = Array2D::new(rows, columns, -1i32, -1i32)?;
        let mut distances_valid = false;
        let mut mask_has_data = false;
        let mut total_cells: usize = 0;
        let mut sum_row = 0f64;
        let mut sum_col = 0f64;
        let mut centroid_row = f64::NAN;
        let mut centroid_col = f64::NAN;
        let mut boundary_cells: Vec<(isize, isize)> = Vec::new();
        let mut perimeter_stream_cells: Vec<(isize, isize)> = Vec::new();

        if let Some(ref ws) = watershed {
            let ws_nodata = ws.configs.nodata;
            old_progress = 1;
            for row in 0..rows {
                for col in 0..columns {
                    let val = ws[(row, col)];
                    if val != ws_nodata && val > 0f64 {
                        mask.set_value(row, col, 1u8);
                        total_cells += 1;
                        sum_row += row as f64;
                        sum_col += col as f64;
                    }
                }
                if verbose && rows > 1 {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Building watershed mask: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            if total_cells == 0 {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Watershed raster does not contain any positive-valued cells.",
                ));
            }

            mask_has_data = true;
            centroid_row = sum_row / total_cells as f64;
            centroid_col = sum_col / total_cells as f64;

            old_progress = 1;
            for row in 0..rows {
                for col in 0..columns {
                    if mask.get_value(row, col) == 1u8 {
                        let mut is_boundary = false;
                        for n in 0..8 {
                            let nr = row + dy[n];
                            let nc = col + dx[n];
                            if nr < 0 || nr >= rows || nc < 0 || nc >= columns {
                                is_boundary = true;
                                break;
                            } else if mask.get_value(nr, nc) == 0u8 {
                                is_boundary = true;
                                break;
                            }
                        }
                        if is_boundary {
                            boundary_cells.push((row, col));
                            let stream_val = streams[(row, col)];
                            if stream_val != streams_nodata && stream_val > 0f64 {
                                perimeter_stream_cells.push((row, col));
                            }
                        }
                    }
                }
                if verbose && rows > 1 {
                    progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                    if progress != old_progress {
                        println!("Scanning watershed boundary: {}%", progress);
                        old_progress = progress;
                    }
                }
            }

            if boundary_cells.is_empty() {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "Unable to locate watershed boundary cells. Check the watershed raster values.",
                ));
            }

            if verbose {
                println!(
                    "Identified {} watershed cells ({} boundary cells).",
                    total_cells,
                    boundary_cells.len()
                );
                if !perimeter_stream_cells.is_empty() {
                    println!(
                        "Warning: Watershed perimeter intersects {} stream cells.",
                        perimeter_stream_cells.len()
                    );
                }
            }

            let mut queue: VecDeque<(isize, isize)> = VecDeque::with_capacity(boundary_cells.len());
            for &(row, col) in &boundary_cells {
                distances.set_value(row, col, 0);
                queue.push_back((row, col));
            }

            while let Some((row, col)) = queue.pop_front() {
                let base_distance = distances.get_value(row, col);
                for n in 0..8 {
                    let nr = row + dy[n];
                    let nc = col + dx[n];
                    if nr >= 0 && nr < rows && nc >= 0 && nc < columns {
                        if mask.get_value(nr, nc) == 1u8 && distances.get_value(nr, nc) == -1 {
                            distances.set_value(nr, nc, base_distance + 1);
                            queue.push_back((nr, nc));
                        }
                    }
                }
            }
            distances_valid = true;
        }

        let mut candidates: Vec<(i32, isize, isize)> = Vec::new();
        if mask_has_data {
            for row in 0..rows {
                for col in 0..columns {
                    if mask.get_value(row, col) == 1u8 {
                        let dist = distances.get_value(row, col);
                        candidates.push((dist, row, col));
                    }
                }
            }
            candidates.sort_by(|a, b| b.0.cmp(&a.0));
        }
        let max_candidates = if mask_has_data {
            candidates.len().min(512)
        } else {
            0usize
        };

        let mut attempt_summaries: Vec<String> = Vec::new();
        let max_steps = (rows * columns * 4).max(1) as usize;

        let trace_ctx = TraceContext {
            pntr: &pntr,
            streams: &streams,
            mask: if mask_has_data { Some(&mask) } else { None },
            junction_counts: &junction_counts,
            pntr_nodata,
            streams_nodata,
            pntr_matches: &pntr_matches,
            dx: &dx,
            dy: &dy,
            rows,
            columns,
            max_steps,
        };

        let mut requested_map_xy: Option<(f64, f64)> = None;
        let mut requested_cell_rowcol: Option<(isize, isize)> = None;
        if let Some((row, col)) = requested_row_col {
            let clamped_row = clamp_index(row, rows - 1);
            let clamped_col = clamp_index(col, columns - 1);
            requested_cell_rowcol = Some((clamped_row, clamped_col));
            requested_map_xy = Some((
                pntr.get_x_from_column(clamped_col),
                pntr.get_y_from_row(clamped_row),
            ));
        } else if let Some((lon, lat)) = requested_lng_lat {
            match lon_lat_to_row_col(&pntr, lon, lat) {
                Some((row, col)) => {
                    requested_cell_rowcol = Some((row, col));
                    requested_map_xy =
                        Some((pntr.get_x_from_column(col), pntr.get_y_from_row(row)));
                }
                None => {
                    let message = format!(
                        "Unable to convert requested outlet lon/lat ({}, {}) to raster coordinates for EPSG {}. Provide --requested_outlet_row_col instead.",
                        lon, lat, pntr.configs.epsg_code
                    );
                    return Err(Error::new(ErrorKind::InvalidInput, message));
                }
            }
        }

        if requested_map_xy.is_none() {
            if let Some((row, col)) = requested_cell_rowcol {
                requested_map_xy = Some((pntr.get_x_from_column(col), pntr.get_y_from_row(row)));
            }
        }

        let mut selected: Option<SelectedTrace> = None;

        if let Some((req_row, req_col)) = requested_cell_rowcol {
            if let Some(((start_row, start_col), offset)) = find_nearest_valid_cell(
                req_row,
                req_col,
                rows,
                columns,
                &pntr,
                pntr_nodata,
                &pntr_matches,
                &dx,
                &dy,
            ) {
                let start_distance_to_boundary = if distances_valid {
                    distances.get_value(start_row, start_col)
                } else {
                    -1
                };
                let label = String::from("Requested start");
                let params = TraceParams {
                    label: &label,
                    mode: TraceStartMode::Requested,
                };
                match trace_flow_path(start_row, start_col, &trace_ctx, &params) {
                    Ok(success) => {
                        selected = Some(SelectedTrace {
                            success,
                            start_row,
                            start_col,
                            start_mode: TraceStartMode::Requested,
                            distance_to_boundary: start_distance_to_boundary,
                            candidate_rank: None,
                            start_offset_cells: offset,
                        });
                    }
                    Err(failure) => {
                        let mut reason = failure.reason;
                        if let Some((jr, jc, jcnt)) = failure.last_junction {
                            reason.push_str(&format!(
                                " Latest stream encountered at row {}, col {} had junction count {}.",
                                jr, jc, jcnt
                            ));
                        }
                        if attempt_summaries.len() < 5 {
                            attempt_summaries.push(reason);
                        }
                    }
                }
            } else {
                let reason = format!(
                    "Requested start: unable to locate a valid D8 cell near row {}, col {}.",
                    req_row, req_col
                );
                if attempt_summaries.len() < 5 {
                    attempt_summaries.push(reason);
                }
            }
        }

        if selected.is_none() && mask_has_data {
            for (idx, &(distance_to_boundary, row, col)) in
                candidates.iter().take(max_candidates).enumerate()
            {
                let label = format!("Candidate {}", idx);
                let params = TraceParams {
                    label: &label,
                    mode: TraceStartMode::WatershedCandidate,
                };
                match trace_flow_path(row, col, &trace_ctx, &params) {
                    Ok(success) => {
                        selected = Some(SelectedTrace {
                            success,
                            start_row: row,
                            start_col: col,
                            start_mode: TraceStartMode::WatershedCandidate,
                            distance_to_boundary,
                            candidate_rank: Some(idx),
                            start_offset_cells: 0,
                        });
                        break;
                    }
                    Err(failure) => {
                        let mut reason = failure.reason;
                        if let Some((jr, jc, jcnt)) = failure.last_junction {
                            reason.push_str(&format!(
                                " Latest stream encountered at row {}, col {} had junction count {}.",
                                jr, jc, jcnt
                            ));
                        }
                        if attempt_summaries.len() < 5 {
                            attempt_summaries.push(reason);
                        }
                    }
                }
            }
        }

        let selected = match selected {
            Some(val) => val,
            None => {
                let mut message = if requested_cell_rowcol.is_some() {
                    String::from("Failed to trace a valid outlet from the requested location.")
                } else {
                    String::from(
                        "Failed to identify an outlet stream cell for the provided watershed mask.",
                    )
                };
                if !attempt_summaries.is_empty() {
                    message.push_str(" Reasons considered: ");
                    message.push_str(&attempt_summaries.join(" | "));
                }
                return Err(Error::new(ErrorKind::InvalidInput, message));
            }
        };

        let TraceSuccessData {
            outlet_row,
            outlet_col,
            steps_taken,
            steps_beyond_mask,
            outlet_downstream,
            outlet_junction_count,
        } = selected.success;
        let easting = pntr.get_x_from_column(outlet_col);
        let northing = pntr.get_y_from_row(outlet_row);
        let epsg_code = pntr.configs.epsg_code;
        let start_row = selected.start_row;
        let start_col = selected.start_col;
        let start_mode_str = selected.start_mode.as_str();
        let distance_to_boundary = selected.distance_to_boundary;
        let candidate_rank = selected.candidate_rank;
        let start_offset_cells = selected.start_offset_cells;
        let start_in_mask = mask_has_data && mask.get_value(start_row, start_col) == 1u8;
        let outlet_in_mask = mask_has_data && mask.get_value(outlet_row, outlet_col) == 1u8;

        let mut properties: JsonMap<String, JsonValue> = JsonMap::new();
        properties.insert("Id".to_string(), json!(0));
        properties.insert("row".to_string(), json!(outlet_row));
        properties.insert("column".to_string(), json!(outlet_col));
        properties.insert("easting".to_string(), json!(easting));
        properties.insert("northing".to_string(), json!(northing));
        properties.insert("epsg".to_string(), json!(epsg_code));
        properties.insert(
            "centroid_row".to_string(),
            if mask_has_data {
                json!(centroid_row)
            } else {
                JsonValue::Null
            },
        );
        properties.insert(
            "centroid_col".to_string(),
            if mask_has_data {
                json!(centroid_col)
            } else {
                JsonValue::Null
            },
        );
        properties.insert(
            "distance_to_boundary".to_string(),
            if distance_to_boundary >= 0 {
                json!(distance_to_boundary)
            } else {
                JsonValue::Null
            },
        );
        properties.insert("start_mode".to_string(), json!(start_mode_str));
        properties.insert("start_row".to_string(), json!(start_row));
        properties.insert("start_col".to_string(), json!(start_col));
        properties.insert("start_in_mask".to_string(), json!(start_in_mask));
        properties.insert(
            "start_distance_to_boundary".to_string(),
            if distance_to_boundary >= 0 {
                json!(distance_to_boundary)
            } else {
                JsonValue::Null
            },
        );
        properties.insert("start_offset_cells".to_string(), json!(start_offset_cells));
        properties.insert("steps_from_start".to_string(), json!(steps_taken));
        properties.insert("steps_from_center".to_string(), json!(steps_taken));
        properties.insert("steps_beyond_mask".to_string(), json!(steps_beyond_mask));
        properties.insert(
            "candidate_rank".to_string(),
            match candidate_rank {
                Some(val) => json!(val),
                None => JsonValue::Null,
            },
        );
        properties.insert("candidates_considered".to_string(), json!(max_candidates));
        properties.insert("watershed_cell_count".to_string(), json!(total_cells));
        properties.insert(
            "outlet_mask_value".to_string(),
            if mask_has_data {
                json!(mask.get_value(outlet_row, outlet_col))
            } else {
                JsonValue::Null
            },
        );
        properties.insert("outlet_in_mask".to_string(), json!(outlet_in_mask));
        properties.insert(
            "outlet_downstream_of_mask".to_string(),
            json!(outlet_downstream),
        );
        properties.insert(
            "outlet_junction_count".to_string(),
            json!(outlet_junction_count),
        );
        properties.insert(
            "perimeter_stream_count".to_string(),
            json!(perimeter_stream_cells.len()),
        );
        properties.insert(
            "requested_lon".to_string(),
            match requested_lng_lat {
                Some((lon, _)) => json!(lon),
                None => JsonValue::Null,
            },
        );
        properties.insert(
            "requested_lat".to_string(),
            match requested_lng_lat {
                Some((_, lat)) => json!(lat),
                None => JsonValue::Null,
            },
        );
        properties.insert(
            "requested_row".to_string(),
            match requested_cell_rowcol {
                Some((r, _)) => json!(r),
                None => JsonValue::Null,
            },
        );
        properties.insert(
            "requested_col".to_string(),
            match requested_cell_rowcol {
                Some((_, c)) => json!(c),
                None => JsonValue::Null,
            },
        );
        properties.insert(
            "requested_cell_offset".to_string(),
            if requested_cell_rowcol.is_some() {
                json!(start_offset_cells)
            } else {
                JsonValue::Null
            },
        );
        properties.insert(
            "requested_easting".to_string(),
            match requested_map_xy {
                Some((x, _)) => json!(x),
                None => JsonValue::Null,
            },
        );
        properties.insert(
            "requested_northing".to_string(),
            match requested_map_xy {
                Some((_, y)) => json!(y),
                None => JsonValue::Null,
            },
        );
        if !perimeter_stream_cells.is_empty() {
            if properties.get("perimeter_stream_count").is_none() {
                properties.insert(
                    "perimeter_stream_count".to_string(),
                    json!(perimeter_stream_cells.len()),
                );
            }
            let preview: Vec<JsonValue> = perimeter_stream_cells
                .iter()
                .take(5)
                .map(|(r, c)| json!({"row": r, "col": c}))
                .collect();
            properties.insert(
                "perimeter_stream_samples".to_string(),
                JsonValue::Array(preview),
            );
        }
        let geometry = Geometry::new(GeoValue::Point(vec![easting, northing]));
        let feature = Feature {
            bbox: None,
            geometry: Some(geometry),
            id: None,
            properties: Some(properties),
            foreign_members: None,
        };

        let mut foreign_members: Option<JsonMap<String, JsonValue>> = None;
        if epsg_code != 0 {
            let mut crs_map = JsonMap::new();
            crs_map.insert("type".to_string(), json!("name"));
            crs_map.insert(
                "properties".to_string(),
                json!({"name": format!("urn:ogc:def:crs:EPSG::{}", epsg_code)}),
            );
            let mut members = JsonMap::new();
            members.insert("crs".to_string(), JsonValue::Object(crs_map));
            foreign_members = Some(members);
        }

        let feature_collection = FeatureCollection {
            bbox: None,
            features: vec![feature],
            foreign_members,
        };

        if verbose {
            println!(
                "Writing outlet GeoJSON to {} (row {}, col {}, distance {}, steps {}).",
                output_file, outlet_row, outlet_col, distance_to_boundary, steps_taken
            );
        }

        let geojson = GeoJson::FeatureCollection(feature_collection).to_string();
        let mut file = File::create(&output_file)?;
        file.write_all(geojson.as_bytes())?;
        file.sync_all()?;

        let elapsed_time = get_formatted_elapsed_time(start);
        if verbose {
            println!("Elapsed Time (excluding I/O): {}", elapsed_time);
        }

        Ok(())
    }
}
