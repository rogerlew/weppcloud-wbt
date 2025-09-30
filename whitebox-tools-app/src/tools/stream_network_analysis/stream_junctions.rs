use crate::tools::*;
use std::env;
use std::f64;
use std::io::{Error, ErrorKind};
use std::path;
use whitebox_raster::*;

pub struct StreamJunctionIdentifier {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl StreamJunctionIdentifier {
    pub fn new() -> StreamJunctionIdentifier {
        let name = "StreamJunctionIdentifier".to_string();
        let toolbox = "Stream Network Analysis".to_string();
        let description =
            "Calculates the number of inflowing channel pixels for each channel pixel.".to_string();

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
        parameters.push(ToolParameter {
            name: "Does the pointer file use the ESRI pointer scheme?".to_owned(),
            flags: vec!["--esri_pntr".to_owned()],
            description: "D8 pointer uses the ESRI style scheme.".to_owned(),
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
        let usage = format!(
            ">>.*{} -r={} -v --wd=\"*path*to*data*\" --d8_pntr=D8.tif --streams=streams.tif -o=output.tif
>>.*{} -r={} -v --wd=\"*path*to*data*\" --d8_pntr=D8.tif --streams=streams.tif -o=output.tif --esri_pntr --zero_background",
            short_exe, name, short_exe, name
        ).replace("*", &sep);

        StreamJunctionIdentifier {
            name,
            description,
            toolbox,
            parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for StreamJunctionIdentifier {
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
        let mut background_val = f64::NEG_INFINITY;

        if args.len() == 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no parameters.",
            ));
        }
        for i in 0..args.len() {
            let arg = args[i].replace("\"", "").replace("\'", "");
            let cmd = arg.split("=");
            let vec = cmd.collect::<Vec<&str>>();
            let keyval = vec.len() > 1;
            if vec[0].to_lowercase() == "-d8_pntr" || vec[0].to_lowercase() == "--d8_pntr" {
                d8_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if vec[0].to_lowercase() == "-streams" || vec[0].to_lowercase() == "--streams" {
                streams_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if vec[0].to_lowercase() == "-o" || vec[0].to_lowercase() == "--output" {
                output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if vec[0].to_lowercase() == "--esri_pntr" {
                if vec.len() == 1 || !vec[1].to_lowercase().contains("false") {
                    esri_style = true;
                }
            }
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
        if !d8_file.contains(&sep) && !d8_file.contains("/") {
            d8_file = format!("{}{}", working_directory, d8_file);
        }
        if !streams_file.contains(&sep) && !streams_file.contains("/") {
            streams_file = format!("{}{}", working_directory, streams_file);
        }
        if !output_file.contains(&sep) && !output_file.contains("/") {
            output_file = format!("{}{}", working_directory, output_file);
        }

        if verbose {
            println!("Reading pointer data...");
        }
        let pntr = Raster::new(&d8_file, "r")?;
        if verbose {
            println!("Reading streams data...");
        }
        let streams = Raster::new(&streams_file, "r")?;

        let start = Instant::now();

        let rows = pntr.configs.rows as isize;
        let columns = pntr.configs.columns as isize;
        let nodata = -32768.0;
        if background_val == f64::NEG_INFINITY {
            background_val = nodata;
        }

        if streams.configs.rows != pntr.configs.rows
            || streams.configs.columns != pntr.configs.columns
        {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "The input files must have the same number of rows and columns and spatial extent.",
            ));
        }

        let mut output = Raster::initialize_using_file(&output_file, &streams);

        let dx = [1, 1, 1, 0, -1, -1, -1, 0];
        let dy = [-1, 0, 1, 1, 1, 0, -1, -1];

        let mut inflowing_vals = [16f64, 32f64, 64f64, 128f64, 1f64, 2f64, 4f64, 8f64];
        if esri_style {
            inflowing_vals = [8f64, 16f64, 32f64, 64f64, 128f64, 1f64, 2f64, 4f64];
        }

        let num_cells = (rows * columns) as usize;
        let mut processed = 0usize;
        for row in 0..rows {
            for col in 0..columns {
                if streams[(row, col)] > 0.0 {
                    let mut cnt = 0i16;
                    for k in 0..8 {
                        let rn = row + dy[k];
                        let cn = col + dx[k];
                        if streams[(rn, cn)] > 0.0 && pntr[(rn, cn)] == inflowing_vals[k] {
                            cnt += 1;
                        }
                    }
                    output[(row, col)] = cnt as f64;
                } else {
                    output[(row, col)] = background_val; // 0 or NoData
                }
                processed += 1;
            }
            if verbose {
                let prog = (100.0 * processed as f64 / num_cells as f64) as usize;
                println!("Progress: {}%", prog);
            }
        }

        let elapsed_time = get_formatted_elapsed_time(start);
        output.configs.palette = "qual.plt".to_string();
        output.configs.photometric_interp = PhotometricInterpretation::Categorical;
        output.add_metadata_entry(format!(
            "Created by whitebox_tools\' {} tool",
            self.get_tool_name()
        ));
        output.add_metadata_entry(format!("Input d8 pointer file: {}", d8_file));
        output.add_metadata_entry(format!("Input streams file: {}", streams_file));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

        if verbose {
            println!("Saving data...");
        }
        let _ = output.write().map_err(|e| {
            if verbose {
                println!("Error writing output file: {:?}", e);
            }
            e
        })?;
        if verbose {
            println!("Output file written");
            println!("Elapsed Time (excluding I/O): {}", elapsed_time);
        }

        Ok(())
    }
}
