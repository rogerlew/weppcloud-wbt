/*!
This tool is part of the WhiteboxTools geospatial analysis library.
Author: Roger Lew
Created: 08/06/2025
License: MIT
*/

use crate::tools::*;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;
use std::sync::Arc;
use whitebox_common::utils::get_formatted_elapsed_time;
use whitebox_raster::*;

pub struct ClipRasterToRaster {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl ClipRasterToRaster {
    pub fn new() -> ClipRasterToRaster {
        // --- metadata ---
        let name = "ClipRasterToRaster".to_string();
        let toolbox = "GIS Analysis/Overlay Tools".to_string();
        let description = "Clips a raster to a raster mask.".to_string();

        // --- parameters ---
        let mut parameters = vec![];
        parameters.push(ToolParameter {
            name: "Input File".to_owned(),
            flags: vec!["-i".to_owned(), "--input".to_owned()],
            description: "Input raster file.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Mask Raster".to_owned(),
            flags: vec!["-m".to_owned(), "--mask".to_owned()],
            description: "Raster defining the clip area (cells with nodata OR value 0 are excluded)."
                .to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });
        
        parameters.push(ToolParameter {
            name: "Output Raster".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        // --- example usage ---
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let exe = format!("{}", env::current_exe().unwrap().display());
        let mut parent = env::current_exe().unwrap();
        parent.pop();
        let exe_short = exe
            .replace(&format!("{}", parent.display()), "")
            .replace(".exe", "")
            .replace(&sep, "");
        let usage = format!(
            ">>{} -r={} -v --wd=\"*path*to*wd*\" -i=input.tif -m=mask.tif -o=clipped.tif",
            exe_short, name
        )
        .replace("*", &sep);

        ClipRasterToRaster {
            name,
            description,
            toolbox,
            parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for ClipRasterToRaster {
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
        serde_json::to_string(&self.parameters)
            .map(|s| format!("{{\"parameters\":{}}}", s))
            .unwrap_or_else(|e| format!("{:?}", e))
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
        // --------------------------------------------------
        //              Parse arguments
        // --------------------------------------------------
        if args.is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput, "No parameters supplied."));
        }
        let sep: String = path::MAIN_SEPARATOR.to_string();
        let mut input_file = String::new();
        let mut mask_file = String::new();
        let mut output_file = String::new();

        if args.len() == 0 {
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
            if flag_val == "-i" || flag_val == "-input" {
                input_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-m" || flag_val == "-mask" {
                mask_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-o" || flag_val == "-output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            }
        }

        if input_file.is_empty() || mask_file.is_empty() || output_file.is_empty() {
            return Err(Error::new(ErrorKind::InvalidInput, "Missing required arguments."));
        }
        if !input_file.contains(&sep) && !input_file.contains('/') {
            input_file = format!("{}{}", working_directory, input_file);
        }
        if !mask_file.contains(&sep) && !mask_file.contains('/') {
            mask_file = format!("{}{}", working_directory, mask_file);
        }
        if !output_file.contains(&sep) && !output_file.contains('/') {
            output_file = format!("{}{}", working_directory, output_file);
        }

        // --------------------------------------------------
        //          Open rasters and sanity checks
        // --------------------------------------------------
        let input = Arc::new(Raster::new(&input_file, "r")?);
        let mask = Arc::new(Raster::new(&mask_file, "r")?);
        if input.configs.rows != mask.configs.rows
            || input.configs.columns != mask.configs.columns
            || (input.configs.resolution_x - mask.configs.resolution_x).abs() > f64::EPSILON
            || (input.configs.resolution_y - mask.configs.resolution_y).abs() > f64::EPSILON
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Input and mask rasters must have identical extent, rows, columns, and resolution.",
            ));
        }

        //--------------------------------------------------
        //              Core clipping loop
        //--------------------------------------------------
        let rows     = input.configs.rows as isize;
        let columns  = input.configs.columns as isize;
        let nodata_i = input.configs.nodata;
        let nodata_m = mask.configs.nodata;

        let start     = std::time::Instant::now();
        let mut output   = Raster::initialize_using_file(&output_file, &input);

        let mut old_progress = 0usize;
        for row in 0..rows {
            for col in 0..columns {
                let m_val = mask.get_value(row, col);
                if m_val != nodata_m && m_val != 0.0 {
                    output[(row, col)] = input.get_value(row, col);
                } else {
                    output[(row, col)] = nodata_i;
                }
            }

            if verbose {
                let progress = ((row as f64) / ((rows - 1) as f64) * 100.0) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        if verbose {
            println!("Saving data…");
        }
        output.add_metadata_entry(format!(
            "Created by whitebox_tools' {}",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input:  {}", input_file));
        output.add_metadata_entry(format!("Mask:   {}", mask_file));
        output.add_metadata_entry(format!(
            "Elapsed Time (excluding I/O): {}",
            get_formatted_elapsed_time(start)
        ));
        output.write()?;

        if verbose {
            println!(
                "Elapsed Time (excluding I/O): {}",
                get_formatted_elapsed_time(start)
            );
        }
        Ok(())
    }
}