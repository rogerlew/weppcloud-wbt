/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: WEPPcloud contributors
Created: 2025-02-18
License: MIT
*/

use crate::tools::*;
use num_cpus;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::Instant;
use whitebox_common::utils::{get_formatted_elapsed_time, haversine_distance, vincenty_distance};
use whitebox_raster::*;

/// Calculates slope gradient in the direction of flow (flow vector slope) using a D8 pointer
/// raster and a DEM. Unlike the standard `Slope` tool which calculates the maximum terrain
/// gradient, this tool follows the flow direction for each cell, matching the behaviour used
/// by TOPAZ for channel routing.
///
/// # See Also
/// `D8Pointer`, `Slope`
pub struct FVSlope {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl FVSlope {
    pub fn new() -> FVSlope {
        // public constructor
        let name = "FVSlope".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description = "Calculates slope gradient in the D8 flow direction.".to_string();

        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input DEM File".to_owned(),
            flags: vec!["-i".to_owned(), "--dem".to_owned()],
            description: "Input raster DEM file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Input D8 Pointer File".to_owned(),
            flags: vec!["--d8_pntr".to_owned()],
            description: "Input D8 pointer raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Does the pointer file use the ESRI pointer scheme?".to_owned(),
            flags: vec!["--esri_pntr".to_owned()],
            description: "D8 pointer uses the ESRI style scheme.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Z Conversion Factor".to_owned(),
            flags: vec!["--zfactor".to_owned()],
            description:
                "Optional multiplier for when the vertical and horizontal units are not the same."
                    .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Units".to_owned(),
            flags: vec!["--units".to_owned()],
            description:
                "Units of output raster; options include 'degrees', 'radians', 'percent', 'ratio'."
                    .to_owned(),
            parameter_type: ParameterType::OptionList(vec![
                "degrees".to_owned(),
                "radians".to_owned(),
                "percent".to_owned(),
                "ratio".to_owned(),
            ]),
            default_value: Some("degrees".to_owned()),
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
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" --dem=DEM.tif --d8_pntr=D8.tif -o=fvslope.tif --units=\"degrees\"",
            short_exe, name
        )
        .replace("*", &sep);

        FVSlope {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for FVSlope {
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
            Ok(json_str) => return format!("{{\"parameters\":{}}}", json_str),
            Err(err) => return format!("{:?}", err),
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
        let mut dem_file = String::new();
        let mut d8_file = String::new();
        let mut output_file = String::new();
        let mut esri_style = false;
        let mut z_factor = 1f64;
        let mut units = "degrees".to_string();
        let mut units_numeric = 1; // degrees

        if args.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no parameters.",
            ));
        }
        for i in 0..args.len() {
            let mut arg = args[i].replace("\"", "");
            arg = arg.replace("\'", "");
            let cmd = arg.split("="); // in case an equals sign was used
            let vec = cmd.collect::<Vec<&str>>();
            let mut keyval = false;
            if vec.len() > 1 {
                keyval = true;
            }
            let flag_val = vec[0].to_lowercase().replace("--", "-");
            if flag_val == "-i" || flag_val == "-input" || flag_val == "-dem" {
                if keyval {
                    dem_file = vec[1].to_string();
                } else {
                    dem_file = args[i + 1].to_string();
                }
            } else if flag_val == "-d8_pntr" {
                if keyval {
                    d8_file = vec[1].to_string();
                } else {
                    d8_file = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } else if flag_val == "-esri_pntr" || flag_val == "-esri_style" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    esri_style = true;
                }
            } else if flag_val == "-zfactor" {
                if keyval {
                    z_factor = vec[1].parse::<f64>().map_err(|_| {
                        Error::new(ErrorKind::InvalidInput, "Error reading zfactor.")
                    })?;
                } else {
                    z_factor = args[i + 1].parse::<f64>().map_err(|_| {
                        Error::new(ErrorKind::InvalidInput, "Error reading zfactor.")
                    })?;
                }
            } else if flag_val == "-units" {
                if keyval {
                    units = vec[1].to_string();
                } else {
                    units = args[i + 1].to_string();
                }
                units = units.to_lowercase();
                units_numeric = match units.as_ref() {
                    "degrees" | "degree" | "deg" => 1,
                    "radians" | "radian" | "rad" => 2,
                    "percent" | "per" | "pct" => 3,
                    "ratio" => 4,
                    _ => {
                        return Err(Error::new(
                            ErrorKind::InvalidInput,
                            "Unrecognized units option (degrees, radians, percent, ratio).",
                        ))
                    }
                };
            }
        }

        if dem_file.is_empty() || d8_file.is_empty() || output_file.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "DEM, D8 pointer, and output files must be specified.",
            ));
        }

        if verbose {
            let tool_name = self.get_tool_name();
            let welcome_len = format!("* Welcome to {} *", tool_name).len().max(28);
            // 28 = length of the 'Powered by' by statement.
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

        let mut progress: usize;
        let mut old_progress: usize = 1;

        if !dem_file.contains(&sep) && !dem_file.contains("/") {
            dem_file = format!("{}{}", working_directory, dem_file);
        }
        if !d8_file.contains(&sep) && !d8_file.contains("/") {
            d8_file = format!("{}{}", working_directory, d8_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading DEM data...")
        };
        let dem = Arc::new(Raster::new(&dem_file, "r")?);
        if verbose {
            println!("Reading D8 pointer data...")
        };
        let pntr = Arc::new(Raster::new(&d8_file, "r")?);

        // make sure the input files have the same size
        if dem.configs.rows != pntr.configs.rows || dem.configs.columns != pntr.configs.columns {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }

        let start = Instant::now();

        let rows = dem.configs.rows as isize;
        let columns = dem.configs.columns as isize;
        let nodata = dem.configs.nodata;
        let pntr_nodata = pntr.configs.nodata;
        let cell_size_x = dem.configs.resolution_x;
        let cell_size_y = dem.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
        let out_nodata = nodata;
        let is_geographic =
            dem.is_in_geographic_coordinates() || pntr.is_in_geographic_coordinates();

        let use_haversine = if is_geographic {
            let phi1 = dem.get_y_from_row(0);
            let lambda1 = dem.get_x_from_column(0);
            let phi2 = phi1;
            let lambda2 = dem.get_x_from_column(-1);
            let vin_dist = vincenty_distance((phi1, lambda1), (phi2, lambda2));
            let hav_dist = haversine_distance((phi1, lambda1), (phi2, lambda2));
            if vin_dist != 0.0 {
                let diff = 100. * (vin_dist - hav_dist).abs() / vin_dist;
                diff < 0.5
            } else {
                true
            }
        } else {
            false
        };

        let mut num_procs = num_cpus::get() as isize;
        let configs = whitebox_common::configs::get_configs()?;
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
        let (tx, rx) = mpsc::channel();
        for tid in 0..num_procs {
            let dem = dem.clone();
            let pntr = pntr.clone();
            let tx1 = tx.clone();
            thread::spawn(move || {
                let dx = [1isize, 1, 1, 0, -1, -1, -1, 0];
                let dy = [-1isize, 0, 1, 1, 1, 0, -1, -1];
                let mut pntr_matches: [usize; 129] = [999usize; 129];
                if !esri_style {
                    // Whitebox-style D8 pointer values (1,2,4,8,16,32,64,128)
                    pntr_matches[1] = 0usize;
                    pntr_matches[2] = 1usize;
                    pntr_matches[4] = 2usize;
                    pntr_matches[8] = 3usize;
                    pntr_matches[16] = 4usize;
                    pntr_matches[32] = 5usize;
                    pntr_matches[64] = 6usize;
                    pntr_matches[128] = 7usize;
                } else {
                    // ESRI-style D8 pointer values (1,2,4,8,16,32,64,128)
                    pntr_matches[1] = 1usize;
                    pntr_matches[2] = 2usize;
                    pntr_matches[4] = 3usize;
                    pntr_matches[8] = 4usize;
                    pntr_matches[16] = 5usize;
                    pntr_matches[32] = 6usize;
                    pntr_matches[64] = 7usize;
                    pntr_matches[128] = 0usize;
                }

                for row in (0..rows).filter(|r| r % num_procs == tid) {
                    let mut data = vec![out_nodata; columns as usize];
                    let y1 = dem.get_y_from_row(row);
                    for col in 0..columns {
                        let z_here = dem[(row, col)];
                        let dir_val = pntr[(row, col)];
                        if z_here != nodata && dir_val != pntr_nodata && dir_val > 0.0 {
                            let dir = dir_val as usize;
                            if dir < pntr_matches.len() && pntr_matches[dir] != 999 {
                                let c = pntr_matches[dir];
                                let n_row = row + dy[c];
                                let n_col = col + dx[c];
                                if n_row >= 0 && n_row < rows && n_col >= 0 && n_col < columns {
                                    let z_dn = dem[(n_row, n_col)];
                                    if z_dn != nodata {
                                        let distance = if is_geographic {
                                            let y2 = dem.get_y_from_row(n_row);
                                            let x1 = dem.get_x_from_column(col);
                                            let x2 = dem.get_x_from_column(n_col);
                                            if use_haversine {
                                                haversine_distance((y1, x1), (y2, x2))
                                            } else {
                                                vincenty_distance((y1, x1), (y2, x2))
                                            }
                                        } else if dx[c] == 0 || dy[c] == 0 {
                                            if dx[c] == 0 {
                                                cell_size_y
                                            } else {
                                                cell_size_x
                                            }
                                        } else {
                                            diag_cell_size
                                        };

                                        if distance > 0.0 {
                                            let mut slope = ((z_here * z_factor)
                                                - (z_dn * z_factor))
                                                / distance;
                                            if slope < 0.0 {
                                                slope = 0.0;
                                            }
                                            data[col as usize] = match units_numeric {
                                                1 => slope.atan().to_degrees(), // degrees
                                                2 => slope.atan(),              // radians
                                                3 => slope * 100f64,            // percent
                                                _ => slope,                     // ratio
                                            };
                                        }
                                    }
                                }
                            }
                        }
                    }
                    tx1.send((row, data)).unwrap();
                }
            });
        }

        let mut output = Raster::initialize_using_file(&output_file, &dem);
        output.configs.data_type = DataType::F32;
        output.configs.photometric_interp = PhotometricInterpretation::Continuous;
        output.configs.palette = "spectrum.plt".to_string();
        for row in 0..rows {
            let data = rx.recv().expect("Error receiving data from thread.");
            output.set_row_data(data.0, data.1);

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1) as f64) as usize;
                if progress != old_progress {
                    println!("Performing analysis: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input DEM file: {}", dem_file));
        output.add_metadata_entry(format!("Input D8 pointer file: {}", d8_file));
        output.add_metadata_entry(format!("Slope units: {}", units));
        output.add_metadata_entry(format!("ESRI pointer: {}", esri_style));
        output.add_metadata_entry(format!("Z-factor: {}", z_factor));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

        if verbose {
            println!("Saving data...")
        };
        match output.write() {
            Ok(_) => {
                if verbose {
                    println!("Output file written")
                }
            }
            Err(e) => return Err(e),
        };
        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }

        Ok(())
    }
}
