/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: WEPPcloud Team
Created: 13/04/2026
License: MIT
*/

use crate::tools::*;
use std::env;
use std::io::{Error, ErrorKind};
use std::path;

#[allow(dead_code)]
#[path = "iterative_first_order_link_prune_topology.rs"]
mod iterative_first_order_link_prune_topology;

const DEFAULT_EPSILON: f64 = 1e-5;
const DEFAULT_FAIL_IF_ONLY_CHANNEL_PRUNED: bool = true;

#[derive(Debug, Clone, PartialEq)]
struct ParsedArgs {
    d8_pntr: String,
    upstream_area: String,
    output: String,
    csa: f64,
    mscl: f64,
    threshold_code_raster: Option<String>,
    threshold_table: Option<String>,
    esri_pntr: bool,
    epsilon: f64,
    fail_if_only_channel_pruned: bool,
}

/// Implements IFOLP command contract scaffolding for WP-01.
///
/// Behavior note: the algorithm body is intentionally unimplemented in this package.
/// Phase logic placeholders return explicit errors and will be implemented in WP-02+.
pub struct IterativeFirstOrderLinkPrune {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl IterativeFirstOrderLinkPrune {
    pub fn new() -> IterativeFirstOrderLinkPrune {
        let name = "IterativeFirstOrderLinkPrune".to_string();
        let toolbox = "Stream Network Analysis".to_string();
        let description =
            "Scaffolds iterative first-order link prune command contract (WP-01).".to_string();

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
            name: "Input Upstream Area File".to_owned(),
            flags: vec!["--upstream_area".to_owned()],
            description: "Input upstream area raster file (cells).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output File".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output stream raster file (binary stream mask).".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Critical Source Area (ha)".to_owned(),
            flags: vec!["--csa".to_owned()],
            description: "Default critical source area threshold in hectares.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Minimum Source Channel Length (m)".to_owned(),
            flags: vec!["--mscl".to_owned()],
            description: "Default minimum source channel length threshold in meters.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Threshold Code Raster".to_owned(),
            flags: vec!["--threshold_code_raster".to_owned()],
            description: "Optional integer code raster for spatial threshold lookup.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Threshold Table".to_owned(),
            flags: vec!["--threshold_table".to_owned()],
            description: "Optional threshold table mapping code -> (csa_ha, mscl_m).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Text),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Use ESRI Pointer Scheme".to_owned(),
            flags: vec!["--esri_pntr".to_owned()],
            description: "Pointer raster uses ESRI D8 encoding.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some("false".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Floating Comparison Tolerance".to_owned(),
            flags: vec!["--epsilon".to_owned()],
            description: "Floating tolerance for strict-improvement comparisons.".to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some(DEFAULT_EPSILON.to_string()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Fail If Only Channel Pruned".to_owned(),
            flags: vec!["--fail_if_only_channel_pruned".to_owned()],
            description: "Enable only-channel prune guard failure behavior.".to_owned(),
            parameter_type: ParameterType::Boolean,
            default_value: Some(DEFAULT_FAIL_IF_ONLY_CHANNEL_PRUNED.to_string()),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --d8_pntr=d8.tif --upstream_area=area.tif --output=streams.tif --csa=10.0 --mscl=100.0\n>>.*{0} -r={1} -v --wd=\"*path*to*data*\" --d8_pntr=d8.tif --upstream_area=area.tif --output=streams.tif --csa=10.0 --mscl=100.0 --threshold_code_raster=codes.tif --threshold_table=thresholds.csv --epsilon=1e-5 --fail_if_only_channel_pruned=true",
            short_exe, name
        )
        .replace("*", &sep);

        IterativeFirstOrderLinkPrune {
            name,
            description,
            toolbox,
            parameters,
            example_usage: usage,
        }
    }
}

fn parse_bool(text: &str, flag: &str) -> Result<bool, Error> {
    match text.trim().to_lowercase().as_str() {
        "true" | "t" | "1" | "yes" | "y" => Ok(true),
        "false" | "f" | "0" | "no" | "n" => Ok(false),
        _ => Err(Error::new(
            ErrorKind::InvalidInput,
            format!("Error parsing {} as bool: {}", flag, text),
        )),
    }
}

fn parse_optional_bool_argument(
    args: &[String],
    i: usize,
    keyval: bool,
    vec: &[&str],
    flag: &str,
) -> Result<bool, Error> {
    if keyval {
        let value = normalize_value_token(vec[1]);
        if value.is_empty() {
            Ok(true)
        } else {
            parse_bool(&value, flag)
        }
    } else {
        match args.get(i + 1) {
            Some(next) if !next.trim().starts_with('-') => {
                parse_bool(&normalize_value_token(next), flag)
            }
            _ => Ok(true),
        }
    }
}

fn normalize_value_token(value: &str) -> String {
    let trimmed = value.trim();
    let maybe_double = trimmed
        .strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(trimmed);
    maybe_double
        .strip_prefix('\'')
        .and_then(|s| s.strip_suffix('\''))
        .unwrap_or(maybe_double)
        .to_string()
}

fn parse_value_argument(
    args: &[String],
    i: usize,
    keyval: bool,
    vec: &[&str],
    flag: &str,
) -> Result<String, Error> {
    let value = if keyval {
        normalize_value_token(vec[1])
    } else {
        let next = args.get(i + 1).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("Expected value following {}", flag),
            )
        })?;
        normalize_value_token(next)
    };

    if value.is_empty() || (!keyval && value.starts_with('-')) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("Expected value following {}", flag),
        ));
    }

    Ok(value)
}

fn parse_numeric_argument(
    args: &[String],
    i: usize,
    keyval: bool,
    vec: &[&str],
    flag: &str,
) -> Result<String, Error> {
    let value = if keyval {
        normalize_value_token(vec[1])
    } else {
        let next = args.get(i + 1).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("Expected value following {}", flag),
            )
        })?;
        normalize_value_token(next)
    };

    if value.is_empty() || (!keyval && value.starts_with('-') && value.parse::<f64>().is_err()) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("Expected value following {}", flag),
        ));
    }

    Ok(value)
}

fn parse_f64(text: &str, flag: &str) -> Result<f64, Error> {
    text.parse::<f64>().map_err(|_| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("Error parsing {} as f64: {}", flag, text),
        )
    })
}

fn resolve_path(file_name: &str, working_directory: &str) -> String {
    let sep: String = path::MAIN_SEPARATOR.to_string();
    if !file_name.contains(&sep) && !file_name.contains("/") {
        format!("{}{}", working_directory, file_name)
    } else {
        file_name.to_string()
    }
}

fn parse_arguments(args: &[String], working_directory: &str) -> Result<ParsedArgs, Error> {
    let mut d8_pntr = String::new();
    let mut upstream_area = String::new();
    let mut output = String::new();
    let mut csa = None;
    let mut mscl = None;
    let mut threshold_code_raster: Option<String> = None;
    let mut threshold_table: Option<String> = None;
    let mut esri_pntr = false;
    let mut epsilon = DEFAULT_EPSILON;
    let mut fail_if_only_channel_pruned = DEFAULT_FAIL_IF_ONLY_CHANNEL_PRUNED;

    for i in 0..args.len() {
        let cmd = args[i].splitn(2, '=');
        let vec = cmd.collect::<Vec<&str>>();
        let keyval = vec.len() > 1;
        let flag_val = vec[0].trim().to_lowercase().replace("--", "-");

        if flag_val == "-d8_pntr" {
            d8_pntr = parse_value_argument(args, i, keyval, &vec, "--d8_pntr")?;
        } else if flag_val == "-upstream_area" {
            upstream_area = parse_value_argument(args, i, keyval, &vec, "--upstream_area")?;
        } else if flag_val == "-output" || flag_val == "-o" {
            output = parse_value_argument(args, i, keyval, &vec, "--output")?;
        } else if flag_val == "-csa" {
            let value = parse_numeric_argument(args, i, keyval, &vec, "--csa")?;
            csa = Some(parse_f64(&value, "--csa")?);
        } else if flag_val == "-mscl" {
            let value = parse_numeric_argument(args, i, keyval, &vec, "--mscl")?;
            mscl = Some(parse_f64(&value, "--mscl")?);
        } else if flag_val == "-threshold_code_raster" {
            threshold_code_raster = Some(parse_value_argument(
                args,
                i,
                keyval,
                &vec,
                "--threshold_code_raster",
            )?);
        } else if flag_val == "-threshold_table" {
            threshold_table = Some(parse_value_argument(
                args,
                i,
                keyval,
                &vec,
                "--threshold_table",
            )?);
        } else if flag_val == "-esri_pntr" {
            esri_pntr = parse_optional_bool_argument(args, i, keyval, &vec, "--esri_pntr")?;
        } else if flag_val == "-epsilon" {
            let value = parse_numeric_argument(args, i, keyval, &vec, "--epsilon")?;
            epsilon = parse_f64(&value, "--epsilon")?;
        } else if flag_val == "-fail_if_only_channel_pruned" {
            fail_if_only_channel_pruned = parse_optional_bool_argument(
                args,
                i,
                keyval,
                &vec,
                "--fail_if_only_channel_pruned",
            )?;
        }
    }

    if d8_pntr.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Input D8 pointer file not specified (--d8_pntr).",
        ));
    }
    if upstream_area.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Input upstream area file not specified (--upstream_area).",
        ));
    }
    if output.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Output raster file not specified (--output).",
        ));
    }
    let csa = csa.ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            "Critical source area not specified (--csa).",
        )
    })?;
    let mscl = mscl.ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            "Minimum source channel length not specified (--mscl).",
        )
    })?;
    if epsilon < 0.0 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Floating comparison tolerance must be non-negative (--epsilon).",
        ));
    }
    if threshold_code_raster.is_some() != threshold_table.is_some() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Optional threshold inputs must be provided together (--threshold_code_raster and --threshold_table).",
        ));
    }

    let threshold_code_raster = threshold_code_raster.map(|v| resolve_path(&v, working_directory));
    let threshold_table = threshold_table.map(|v| resolve_path(&v, working_directory));

    Ok(ParsedArgs {
        d8_pntr: resolve_path(&d8_pntr, working_directory),
        upstream_area: resolve_path(&upstream_area, working_directory),
        output: resolve_path(&output, working_directory),
        csa,
        mscl,
        threshold_code_raster,
        threshold_table,
        esri_pntr,
        epsilon,
        fail_if_only_channel_pruned,
    })
}

fn run_phase_a_placeholder(_args: &ParsedArgs) -> Result<(), Error> {
    Err(Error::new(
        ErrorKind::Unsupported,
        "Phase A source-area qualification is not implemented in WP-01 scaffolding.",
    ))
}

fn run_phase_b_placeholder(_args: &ParsedArgs) -> Result<(), Error> {
    Err(Error::new(
        ErrorKind::Unsupported,
        "Phase B first-order-link pruning is not implemented in WP-01 scaffolding.",
    ))
}

impl WhiteboxTool for IterativeFirstOrderLinkPrune {
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
                s.push(',');
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
        if args.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no parameters.",
            ));
        }

        let parsed = parse_arguments(&args, working_directory)?;

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

            println!(
                "IFOLP WP-01 scaffolding parsed args: csa={}, mscl={}, epsilon={}, esri_pntr={}, fail_if_only_channel_pruned={}",
                parsed.csa,
                parsed.mscl,
                parsed.epsilon,
                parsed.esri_pntr,
                parsed.fail_if_only_channel_pruned
            );
        }

        run_phase_a_placeholder(&parsed)?;
        run_phase_b_placeholder(&parsed)?;
        Ok(())
    }
}

#[cfg(test)]
#[path = "iterative_first_order_link_prune_parser_tests.rs"]
mod iterative_first_order_link_prune_parser_tests;

#[cfg(test)]
#[path = "iterative_first_order_link_prune_topology_tests.rs"]
mod iterative_first_order_link_prune_topology_tests;
