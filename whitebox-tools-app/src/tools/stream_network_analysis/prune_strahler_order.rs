/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. Roger Lew
Created: 29/09/2025
Last Modified:29/09/2025
License: MIT
*/

use crate::tools::*;
use std::collections::{HashSet, VecDeque};
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use whitebox_common::structures::Array2D;
use whitebox_raster::*;

/// This tool can be used to remove stream links in a stream network that are shorter than a user-specified length (`--min_length`).
/// The user must specify the names of a streams raster image (`--streams`) and D8 pointer image (`--d8_pntr`). Stream cells
/// are designated in the streams raster as all positive, nonzero values. Thus all non-stream or background grid cells are
/// commonly assigned either zeros or NoData values. The pointer raster is used to traverse the stream network and should only
/// be created using the D8 algorithm. Background cells will be assigned the NoData value in the output image, unless the
/// `--zero_background` parameter is used, in which case non-stream cells will be assigned zero values in the output.
///
/// By default, the pointer raster is assumed to use the clockwise indexing method used by WhiteboxTools.
/// If the pointer file contains ESRI flow direction values instead, the `--esri_pntr` parameter must be specified.
///
/// # See Also
/// `ExtractStreams`
pub struct PruneStrahlerStreamOrder {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl PruneStrahlerStreamOrder {
    pub fn new() -> PruneStrahlerStreamOrder {
        // public constructor
        let name = "PruneStrahlerStreamOrder".to_string();
        let toolbox = "Stream Network Analysis".to_string();
        let description = "Prunes the Strahler order of a stream network.".to_string();

        let mut parameters = vec![];

        parameters.push(ToolParameter {
            name: "Input Streams File".to_owned(),
            flags: vec!["--streams".to_owned()],
            description: "Input raster streams file.".to_owned(),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" ---streams=streams.tif -o=output.tif", short_exe, name).replace("*", &sep);

        PruneStrahlerStreamOrder {
            name: name,
            description: description,
            toolbox: toolbox,
            parameters: parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for PruneStrahlerStreamOrder {
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
        let mut s = String::from("{\"parameters\": [");
        for i in 0..self.parameters.len() {
            if i < self.parameters.len() - 1 {
                s.push_str(&(self.parameters[i].to_string()));
                s.push_str(",");
            } else {
                s.push_str(&(self.parameters[i].to_string()));
            }
        }
        s.push_str("]}");
        s
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
        let mut output_file = String::new();
        let mut esri_style = false;
        let mut min_length = 0.0;
        let mut max_junctions = 3i32;

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
            if flag_val == "-streams" {
                if keyval {
                    streams_file = vec[1].to_string();
                } else {
                    streams_file = args[i + 1].to_string();
                }
            } else if flag_val == "-o" || flag_val == "-output" {
                if keyval {
                    output_file = vec[1].to_string();
                } else {
                    output_file = args[i + 1].to_string();
                }
            } 
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

        if !streams_file.contains(&sep) && !streams_file.contains("/") {
            streams_file = format!("{}{}", working_directory, streams_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading streams data...")
        };
        let streams = Raster::new(&streams_file, "r")?;

        let start = Instant::now();

        let rows = streams.configs.rows as isize;
        let columns = streams.configs.columns as isize;
        let num_cells = streams.num_cells();
        let nodata = streams.configs.nodata;
        let cell_size_x = streams.configs.resolution_x;
        let cell_size_y = streams.configs.resolution_y;
        let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();

        let mut output = Raster::initialize_using_file(&output_file, &streams);
        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input d8 pointer file: {}", d8_file));
        output.add_metadata_entry(format!("Input streams file: {}", streams_file));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

        if verbose {
            println!("Saving data...")
        };
        let _ = match output.write() {
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
