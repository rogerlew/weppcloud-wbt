/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: Dr. Roger Lew and CODEX (gpt-5-codex high)
Created: 29/09/2025
Last Modified: 29/09/2025
License: MIT
*/

use crate::tools::*;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;
use whitebox_raster::*;

/// This tool removes first-order (order value of one) links from an existing Strahler
/// stream-order raster and then renumbers the remaining orders so that every retained
/// channel order is decreased by one. Non-stream cells are assigned either the input
/// raster's NoData value or zero when the `--zero_background` flag is supplied.
///
/// The user must specify the names of an input Strahler-order raster (`--streams`) and
/// an output raster (`--output`). The input raster is expected to contain integer order
/// values, where headwater streams are coded as one. After pruning, former order-two
/// streams become order one, order-three become order two, and so on.
///
/// # See Also
/// `StrahlerStreamOrder`
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
            name: "Input Strahler-Order Raster".to_owned(),
            flags: vec!["--streams".to_owned()],
            description: "Input raster containing Strahler stream orders.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output Raster".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Should a background value of zero be used?".to_owned(),
            flags: vec!["--zero_background".to_owned()],
            description: "Assign zero to non-stream cells instead of NoData.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_owned()),
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
        let usage = format!(">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --streams=strahler.tif -o=pruned.tif\n>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --streams=strahler.tif -o=pruned.tif --zero_background", short_exe, name).replace("*", &sep);

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
        let mut streams_file = String::new();
        let mut output_file = String::new();
        let mut zero_background = false;

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
            } else if flag_val == "-zero_background" {
                if vec.len() == 1 || !vec[1].to_string().to_lowercase().contains("false") {
                    zero_background = true;
                }
            }
        }

        if streams_file.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Input Strahler-order raster (--streams) not specified.",
            ));
        }
        if output_file.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Output raster (--output) not specified.",
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
        let nodata = streams.configs.nodata;

        let mut output = Raster::initialize_using_file(&output_file, &streams);
        let background_val = if zero_background { 0.0 } else { nodata };

        // Shift remaining stream orders down by one and drop first-order links.
        for row in 0..rows {
            for col in 0..columns {
                let z = streams.get_value(row, col);
                if z == nodata {
                    output.set_value(row, col, nodata);
                } else if z > 1.0 {
                    output.set_value(row, col, z - 1.0);
                } else {
                    // Includes order-one streams and background cells.
                    output.set_value(row, col, background_val);
                }
            }
            if verbose && rows > 0 {
                progress = (100.0_f64 * (row + 1) as f64 / rows as f64) as usize;
                if progress != old_progress {
                    println!("Progress: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input streams file: {}", streams_file));
        output.add_metadata_entry(format!("Zero background: {}", zero_background));
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
