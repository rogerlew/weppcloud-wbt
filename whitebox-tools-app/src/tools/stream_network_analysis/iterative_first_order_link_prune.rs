/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: WEPPcloud Team
Created: 13/04/2026
License: MIT
*/

use crate::tools::*;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{BufRead, BufReader, Error, ErrorKind};
use std::path;
use std::time::Instant;
use whitebox_raster::{PhotometricInterpretation, Raster};

#[path = "iterative_first_order_link_prune_phase_a.rs"]
mod iterative_first_order_link_prune_phase_a;
#[path = "iterative_first_order_link_prune_phase_b.rs"]
mod iterative_first_order_link_prune_phase_b;
#[allow(dead_code)]
#[path = "iterative_first_order_link_prune_topology.rs"]
mod iterative_first_order_link_prune_topology;

use self::iterative_first_order_link_prune_phase_a::{
    run_phase_a_qualification, PhaseAInputs, PhaseAResult,
};
use self::iterative_first_order_link_prune_phase_b::{
    run_phase_b_pruning, PhaseBInputs, PhaseBResult,
};
use self::iterative_first_order_link_prune_topology::D8PointerScheme;

const DEFAULT_EPSILON: f64 = 1e-5;
const DEFAULT_FAIL_IF_ONLY_CHANNEL_PRUNED: bool = true;
const HECTARE_TO_SQUARE_METERS: f64 = 10_000.0;

#[derive(Debug, Clone, Copy)]
struct ThresholdTableEntry {
    csa_ha: f64,
    mscl_m: f64,
}

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

#[derive(Clone)]
struct PreparedPhaseInputs {
    d8: Raster,
    rows: isize,
    columns: isize,
    pointers: Vec<u8>,
    pointer_scheme: D8PointerScheme,
    upstream_area_cells: Vec<f64>,
    active_mask: Vec<bool>,
    local_csa_cells: Vec<f64>,
    local_mscl_m: Vec<f64>,
    min_active_csa_cells: f64,
    cell_size_x: f64,
    cell_size_y: f64,
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
            "Performs iterative first-order link prune stream-network qualification/pruning."
                .to_string();

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

fn validate_matching_geometry(reference: &Raster, other: &Raster, name: &str) -> Result<(), Error> {
    if reference.configs.rows != other.configs.rows
        || reference.configs.columns != other.configs.columns
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Raster geometry mismatch: expected {}x{} for {}, got {}x{}",
                reference.configs.rows,
                reference.configs.columns,
                name,
                other.configs.rows,
                other.configs.columns
            ),
        ));
    }
    Ok(())
}

fn parse_integer_raster_value(
    value: f64,
    raster_name: &str,
    row: isize,
    col: isize,
) -> Result<i64, Error> {
    if !value.is_finite() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("Non-finite value in {} at ({}, {})", raster_name, row, col),
        ));
    }
    let rounded = value.round();
    if (value - rounded).abs() > 1e-6 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Expected integer value in {} at ({}, {}), got {}",
                raster_name, row, col, value
            ),
        ));
    }
    Ok(rounded as i64)
}

fn csa_hectares_to_cells(csa_ha: f64, cell_area_m2: f64) -> Result<f64, Error> {
    if !csa_ha.is_finite() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("CSA threshold must be finite, got {}", csa_ha),
        ));
    }
    if cell_area_m2 <= 0.0 || !cell_area_m2.is_finite() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Cell area must be positive and finite for CSA conversion, got {}",
                cell_area_m2
            ),
        ));
    }
    let csa_cells = (csa_ha * HECTARE_TO_SQUARE_METERS / cell_area_m2).round();
    Ok(csa_cells.max(1.0))
}

fn parse_threshold_table(table_path: &str) -> Result<HashMap<i64, ThresholdTableEntry>, Error> {
    let file = File::open(table_path).map_err(|err| {
        Error::new(
            ErrorKind::InvalidInput,
            format!("Failed to open threshold table {}: {}", table_path, err),
        )
    })?;
    let reader = BufReader::new(file);

    let mut table = HashMap::new();
    for (line_idx, line_result) in reader.lines().enumerate() {
        let line = line_result.map_err(|err| {
            Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Failed reading threshold table {} line {}: {}",
                    table_path,
                    line_idx + 1,
                    err
                ),
            )
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let parts: Vec<&str> = if trimmed.contains(',') {
            trimmed.split(',').map(|token| token.trim()).collect()
        } else {
            trimmed.split_whitespace().collect()
        };
        if parts.len() < 3 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Threshold table line {} must contain code,csa_ha,mscl_m",
                    line_idx + 1
                ),
            ));
        }

        let parsed_code = parts[0].parse::<i64>();
        let parsed_csa = parts[1].parse::<f64>();
        let parsed_mscl = parts[2].parse::<f64>();
        if parsed_code.is_err() || parsed_csa.is_err() || parsed_mscl.is_err() {
            if line_idx == 0 {
                continue;
            }
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Threshold table parse error at line {} (expected numeric code,csa_ha,mscl_m)",
                    line_idx + 1
                ),
            ));
        }

        table.insert(
            parsed_code.unwrap(),
            ThresholdTableEntry {
                csa_ha: parsed_csa.unwrap(),
                mscl_m: parsed_mscl.unwrap(),
            },
        );
    }

    if table.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Threshold table did not contain any threshold mappings.",
        ));
    }

    Ok(table)
}

fn prepare_phase_inputs(args: &ParsedArgs) -> Result<PreparedPhaseInputs, Error> {
    let d8 = Raster::new(&args.d8_pntr, "r")?;
    let upstream_area = Raster::new(&args.upstream_area, "r")?;
    validate_matching_geometry(&d8, &upstream_area, "--upstream_area")?;

    let threshold_code_raster = if let Some(code_path) = args.threshold_code_raster.as_ref() {
        let raster = Raster::new(code_path, "r")?;
        validate_matching_geometry(&d8, &raster, "--threshold_code_raster")?;
        Some(raster)
    } else {
        None
    };
    let threshold_table = if let Some(table_path) = args.threshold_table.as_ref() {
        Some(parse_threshold_table(table_path)?)
    } else {
        None
    };

    let rows = d8.configs.rows as isize;
    let columns = d8.configs.columns as isize;
    let expected_len = (rows * columns) as usize;

    let pointer_nodata = d8.configs.nodata;
    let area_nodata = upstream_area.configs.nodata;
    let threshold_code_nodata = threshold_code_raster.as_ref().map(|r| r.configs.nodata);
    let cell_area_m2 =
        upstream_area.configs.resolution_x.abs() * upstream_area.configs.resolution_y.abs();
    let cell_size_x = upstream_area.configs.resolution_x.abs();
    let cell_size_y = upstream_area.configs.resolution_y.abs();
    let default_csa_cells = csa_hectares_to_cells(args.csa, cell_area_m2)?;
    if !args.mscl.is_finite() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("MSCL threshold must be finite, got {}", args.mscl),
        ));
    }

    let mut pointers = vec![0u8; expected_len];
    let mut upstream_area_cells = vec![0.0f64; expected_len];
    let mut active_mask = vec![false; expected_len];
    let mut local_csa_cells = vec![default_csa_cells; expected_len];
    let mut local_mscl_m = vec![args.mscl; expected_len];
    let mut min_active_csa_cells = f64::INFINITY;

    for row in 0..rows {
        for col in 0..columns {
            let idx = (row * columns + col) as usize;
            let pointer_value = d8[(row, col)];
            let area_value = upstream_area[(row, col)];
            if pointer_value == pointer_nodata || area_value == area_nodata {
                continue;
            }

            let pointer_code = parse_integer_raster_value(pointer_value, "--d8_pntr", row, col)?;
            if !(0..=255).contains(&pointer_code) {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "D8 pointer value out of range at ({}, {}): {}",
                        row, col, pointer_code
                    ),
                ));
            }
            if pointer_code == 0 {
                // 0 encodes "no downslope link"; exclude these cells from IFOLP active domain.
                continue;
            }
            pointers[idx] = pointer_code as u8;

            upstream_area_cells[idx] = area_value;
            active_mask[idx] = true;

            let cell_csa_cells = if let (Some(code_raster), Some(table)) =
                (threshold_code_raster.as_ref(), threshold_table.as_ref())
            {
                let code_value = code_raster[(row, col)];
                if code_value == threshold_code_nodata.unwrap() {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!("Threshold code missing at active cell ({}, {})", row, col),
                    ));
                }
                let code =
                    parse_integer_raster_value(code_value, "--threshold_code_raster", row, col)?;
                let entry = table.get(&code).ok_or_else(|| {
                    Error::new(
                        ErrorKind::InvalidInput,
                        format!(
                            "No threshold table entry for code {} at ({}, {})",
                            code, row, col
                        ),
                    )
                })?;
                let csa_cells = csa_hectares_to_cells(entry.csa_ha, cell_area_m2)?;
                if !entry.mscl_m.is_finite() {
                    return Err(Error::new(
                        ErrorKind::InvalidInput,
                        format!(
                            "Threshold table MSCL value must be finite for code {} at ({}, {})",
                            code, row, col
                        ),
                    ));
                }
                local_mscl_m[idx] = entry.mscl_m;
                csa_cells
            } else {
                default_csa_cells
            };

            local_csa_cells[idx] = cell_csa_cells;
            min_active_csa_cells = min_active_csa_cells.min(cell_csa_cells);
        }
    }

    if !min_active_csa_cells.is_finite() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "No valid active cells were found while preparing Phase A qualification.",
        ));
    }

    let pointer_scheme = if args.esri_pntr {
        D8PointerScheme::Esri
    } else {
        D8PointerScheme::Whitebox
    };

    Ok(PreparedPhaseInputs {
        d8,
        rows,
        columns,
        pointers,
        pointer_scheme,
        upstream_area_cells,
        active_mask,
        local_csa_cells,
        local_mscl_m,
        min_active_csa_cells,
        cell_size_x,
        cell_size_y,
    })
}

fn run_phase_a(
    prepared: &PreparedPhaseInputs,
    epsilon: f64,
    verbose: bool,
) -> Result<PhaseAResult, Error> {
    let phase_a = run_phase_a_qualification(&PhaseAInputs {
        rows: prepared.rows,
        columns: prepared.columns,
        pointers: prepared.pointers.clone(),
        pointer_scheme: prepared.pointer_scheme,
        upstream_area_cells: prepared.upstream_area_cells.clone(),
        active_mask: prepared.active_mask.clone(),
        local_csa_cells: prepared.local_csa_cells.clone(),
        min_active_csa_cells: prepared.min_active_csa_cells,
        epsilon,
    })?;

    if verbose {
        let stream_count = phase_a.stream_mask.iter().filter(|cell| **cell).count();
        println!(
            "IFOLP WP-03 Phase A complete: {} stream cells remain after {} pass(es).",
            stream_count,
            phase_a.pass_traces.len()
        );
    }

    Ok(phase_a)
}

fn run_phase_b(
    prepared: &PreparedPhaseInputs,
    phase_a: &PhaseAResult,
    args: &ParsedArgs,
    verbose: bool,
) -> Result<PhaseBResult, Error> {
    let phase_b = run_phase_b_pruning(&PhaseBInputs {
        rows: prepared.rows,
        columns: prepared.columns,
        pointers: prepared.pointers.clone(),
        pointer_scheme: prepared.pointer_scheme,
        initial_stream_mask: phase_a.stream_mask.clone(),
        local_mscl_m: prepared.local_mscl_m.clone(),
        epsilon: args.epsilon,
        fail_if_only_channel_pruned: args.fail_if_only_channel_pruned,
        cell_size_x: prepared.cell_size_x,
        cell_size_y: prepared.cell_size_y,
    })?;

    if verbose {
        let stream_count = phase_b.stream_mask.iter().filter(|cell| **cell).count();
        println!(
            "IFOLP WP-04 Phase B complete: {} stream cells remain after {} pass(es).",
            stream_count,
            phase_b.pass_traces.len()
        );
    }

    Ok(phase_b)
}

fn write_stream_mask_output(
    args: &ParsedArgs,
    prepared: &PreparedPhaseInputs,
    phase_a: &PhaseAResult,
    phase_b: &PhaseBResult,
    elapsed_time: &str,
    verbose: bool,
) -> Result<(), Error> {
    let expected_len = (prepared.rows * prepared.columns) as usize;
    if phase_b.stream_mask.len() != expected_len {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Phase B output length mismatch (expected {}, got {})",
                expected_len,
                phase_b.stream_mask.len()
            ),
        ));
    }

    let mut output = Raster::initialize_using_file(&args.output, &prepared.d8);
    let nodata = output.configs.nodata;
    output.configs.palette = "qual.plt".to_string();
    output.configs.photometric_interp = PhotometricInterpretation::Categorical;

    for row in 0..prepared.rows {
        let mut data = vec![nodata; prepared.columns as usize];
        for col in 0..prepared.columns {
            let idx = (row * prepared.columns + col) as usize;
            if !prepared.active_mask[idx] {
                data[col as usize] = nodata;
            } else if phase_b.stream_mask[idx] {
                data[col as usize] = 1.0;
            } else {
                data[col as usize] = 0.0;
            }
        }
        output.set_row_data(row, data);
    }

    output.add_metadata_entry(format!(
        "Created by whitebox_tools' {} tool",
        IterativeFirstOrderLinkPrune::new().get_tool_name()
    ));
    output.add_metadata_entry(format!("Input d8 pointer file: {}", args.d8_pntr));
    output.add_metadata_entry(format!("Input upstream area file: {}", args.upstream_area));
    output.add_metadata_entry(format!("CSA threshold (ha): {}", args.csa));
    output.add_metadata_entry(format!("MSCL threshold (m): {}", args.mscl));
    output.add_metadata_entry(format!("Epsilon: {}", args.epsilon));
    output.add_metadata_entry(format!(
        "Fail if only channel pruned: {}",
        args.fail_if_only_channel_pruned
    ));
    output.add_metadata_entry(format!("Phase A passes: {}", phase_a.pass_traces.len()));
    output.add_metadata_entry(format!("Phase B passes: {}", phase_b.pass_traces.len()));
    output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));

    if verbose {
        println!("Saving data...");
    }
    output.write()?;
    if verbose {
        println!("Output file written");
    }
    Ok(())
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
                "IFOLP parsed args: csa={}, mscl={}, epsilon={}, esri_pntr={}, fail_if_only_channel_pruned={}",
                parsed.csa,
                parsed.mscl,
                parsed.epsilon,
                parsed.esri_pntr,
                parsed.fail_if_only_channel_pruned
            );
        }

        let start = Instant::now();
        let prepared = prepare_phase_inputs(&parsed)?;
        let phase_a = run_phase_a(&prepared, parsed.epsilon, verbose)?;
        let phase_b = run_phase_b(&prepared, &phase_a, &parsed, verbose)?;
        let elapsed_time = get_formatted_elapsed_time(start);

        write_stream_mask_output(
            &parsed,
            &prepared,
            &phase_a,
            &phase_b,
            &elapsed_time,
            verbose,
        )?;

        if verbose {
            println!(
                "{}",
                &format!("Elapsed Time (excluding I/O): {}", elapsed_time)
            );
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "iterative_first_order_link_prune_parser_tests.rs"]
mod iterative_first_order_link_prune_parser_tests;

#[cfg(test)]
#[path = "iterative_first_order_link_prune_topology_tests.rs"]
mod iterative_first_order_link_prune_topology_tests;

#[cfg(test)]
#[path = "iterative_first_order_link_prune_phase_a_tests.rs"]
mod iterative_first_order_link_prune_phase_a_tests;

#[cfg(test)]
#[path = "iterative_first_order_link_prune_phase_b_tests.rs"]
mod iterative_first_order_link_prune_phase_b_tests;
