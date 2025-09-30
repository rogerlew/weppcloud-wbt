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
            description: "Input watershed mask raster file (1=inside, 0=outside).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --d8_pntr='d8pntr.tif' --streams='streams.tif' --watershed='ws.tif' --output='outlet.geojson'",
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
        if watershed_file.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Input watershed raster (--watershed) not specified.",
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
        if !watershed_file.contains(&sep) && !watershed_file.contains('/') {
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
        let watershed = Raster::new(&watershed_file, "r")?;

        let rows = pntr.configs.rows as isize;
        let columns = pntr.configs.columns as isize;

        if streams.configs.rows as isize != rows || streams.configs.columns as isize != columns {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Streams raster must have the same dimensions as the D8 pointer raster.",
            ));
        }
        if watershed.configs.rows as isize != rows || watershed.configs.columns as isize != columns
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Watershed raster must have the same dimensions as the D8 pointer raster.",
            ));
        }

        let pntr_nodata = pntr.configs.nodata;
        let streams_nodata = streams.configs.nodata;
        let watershed_nodata = watershed.configs.nodata;

        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];
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

        let mut mask: Array2D<u8> = Array2D::new(rows, columns, 0u8, 0u8)?;
        let mut total_cells: usize = 0;
        let mut sum_row = 0f64;
        let mut sum_col = 0f64;

        let mut progress: usize;
        let mut old_progress: usize = 1;

        for row in 0..rows {
            for col in 0..columns {
                let val = watershed[(row, col)];
                if val != watershed_nodata && val > 0f64 {
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

        let centroid_row = sum_row / total_cells as f64;
        let centroid_col = sum_col / total_cells as f64;

        let mut boundary_cells: Vec<(isize, isize)> = Vec::new();
        let mut perimeter_stream_cells: Vec<(isize, isize)> = Vec::new();

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

        let mut distances: Array2D<i32> = Array2D::new(rows, columns, -1i32, -1i32)?;
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

        let mut candidates: Vec<(i32, isize, isize)> = Vec::with_capacity(total_cells);
        for row in 0..rows {
            for col in 0..columns {
                if mask.get_value(row, col) == 1u8 {
                    let dist = distances.get_value(row, col);
                    candidates.push((dist, row, col));
                }
            }
        }
        candidates.sort_by(|a, b| b.0.cmp(&a.0));

        let max_candidates = candidates.len().min(512);
        let mut attempt_summaries: Vec<String> = Vec::new();
        let mut outlet_cell: Option<(isize, isize, usize, i32, usize)> = None;

        for (idx, candidate) in candidates.iter().take(max_candidates).enumerate() {
            let (distance_to_boundary, mut row, mut col) = *candidate;
            let mut visited: HashSet<(isize, isize)> = HashSet::new();
            let mut steps: usize = 0;
            let max_steps = (rows * columns * 4).max(1) as usize;
            let mut failure_reason = String::new();
            loop {
                if !visited.insert((row, col)) {
                    failure_reason = format!(
                        "Candidate {}: flow path loops near row {}, col {}.",
                        idx, row, col
                    );
                    break;
                }
                let pointer = pntr[(row, col)];
                if pointer == pntr_nodata || pointer <= 0f64 {
                    failure_reason = format!(
                        "Candidate {}: encountered invalid D8 pointer ({}) at row {}, col {}.",
                        idx, pointer, row, col
                    );
                    break;
                }
                let pointer_index = pointer.round() as usize;
                if pointer_index >= pntr_matches.len() {
                    failure_reason = format!(
                        "Candidate {}: pointer value {} out of range at row {}, col {}.",
                        idx, pointer, row, col
                    );
                    break;
                }
                let dir = pntr_matches[pointer_index];
                if dir < 0 {
                    failure_reason = format!(
                        "Candidate {}: unsupported pointer value {} at row {}, col {}.",
                        idx, pointer, row, col
                    );
                    break;
                }
                let nr = row + dy[dir as usize];
                let nc = col + dx[dir as usize];
                steps += 1;
                if nr < 0 || nr >= rows || nc < 0 || nc >= columns {
                    let stream_val = streams[(row, col)];
                    if stream_val != streams_nodata && stream_val > 0f64 {
                        outlet_cell = Some((row, col, steps, distance_to_boundary, idx));
                    } else {
                        failure_reason = format!(
                            "Candidate {}: exited raster at row {}, col {} without hitting a stream (value {}).",
                            idx, row, col, stream_val
                        );
                    }
                    break;
                }
                if mask.get_value(nr, nc) == 0u8 {
                    let stream_val = streams[(row, col)];
                    if stream_val != streams_nodata && stream_val > 0f64 {
                        outlet_cell = Some((row, col, steps, distance_to_boundary, idx));
                    } else {
                        failure_reason = format!(
                            "Candidate {}: boundary cell row {}, col {} is not flagged as stream (value {}).",
                            idx, row, col, stream_val
                        );
                    }
                    break;
                }
                row = nr;
                col = nc;
                if steps >= max_steps {
                    failure_reason = format!(
                        "Candidate {}: exceeded maximum step count ({}) before exiting watershed.",
                        idx, max_steps
                    );
                    break;
                }
            }
            if let Some((_, _, _, _, candidate_idx)) = outlet_cell {
                if candidate_idx == idx {
                    break;
                }
            } else if !failure_reason.is_empty() {
                if attempt_summaries.len() < 5 {
                    attempt_summaries.push(failure_reason);
                }
            }
        }

        let (outlet_row, outlet_col, steps_taken, distance_to_boundary, candidate_rank) =
            match outlet_cell {
                Some(val) => val,
                None => {
                    let mut message = String::from(
                        "Failed to identify an outlet stream cell for the provided watershed mask.",
                    );
                    if !attempt_summaries.is_empty() {
                        message.push_str(" Reasons considered: ");
                        message.push_str(&attempt_summaries.join(" | "));
                    }
                    return Err(Error::new(ErrorKind::InvalidInput, message));
                }
            };

        let easting = pntr.get_x_from_column(outlet_col);
        let northing = pntr.get_y_from_row(outlet_row);
        let epsg_code = pntr.configs.epsg_code;

        let mut properties: JsonMap<String, JsonValue> = JsonMap::new();
        properties.insert("Id".to_string(), json!(0));
        properties.insert("row".to_string(), json!(outlet_row));
        properties.insert("column".to_string(), json!(outlet_col));
        properties.insert("easting".to_string(), json!(easting));
        properties.insert("northing".to_string(), json!(northing));
        properties.insert("epsg".to_string(), json!(epsg_code));
        properties.insert("centroid_row".to_string(), json!(centroid_row));
        properties.insert("centroid_col".to_string(), json!(centroid_col));
        properties.insert(
            "distance_to_boundary".to_string(),
            json!(distance_to_boundary),
        );
        properties.insert("steps_from_center".to_string(), json!(steps_taken));
        properties.insert("candidate_rank".to_string(), json!(candidate_rank));
        properties.insert("candidates_considered".to_string(), json!(max_candidates));
        properties.insert("watershed_cell_count".to_string(), json!(total_cells));
        properties.insert(
            "perimeter_stream_count".to_string(),
            json!(perimeter_stream_cells.len()),
        );
        if !perimeter_stream_cells.is_empty() {
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
