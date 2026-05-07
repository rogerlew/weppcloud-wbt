/*
This tool is part of the WhiteboxTools geospatial analysis library.
Authors: WEPPcloud Contributors
Created: 20/03/2026
Last Modified: 20/03/2026
License: MIT
*/

use crate::tools::hydro_analysis::{
    BreachSingleCellPits, D8FlowAccumulation, DInfFlowAccumulation, FD8FlowAccumulation,
    FindNoFlowCells,
};
use crate::tools::terrain_analysis::Slope;
use crate::tools::*;
use std::env;
use std::f64;
use std::fs;
use std::io::{Error, ErrorKind};
use std::path::{self, Path};
use std::time::{SystemTime, UNIX_EPOCH};
use whitebox_raster::*;

/// Computes a purpose-built RUSLE LS factor using locked v1 equations and controls.
///
/// `L` is computed from effective slope length using Desmet-Govers style scaling
/// and `m` from the McCool beta relationship. `S` uses the McCool/RUSLE piecewise
/// slope steepness equations.
///
/// If `sca` or `slope_deg` are omitted, they are derived from `dem` using the
/// selected routing mode for `sca` and the `Slope` tool for slope.
///
/// # See Also
/// `DInfFlowAccumulation`, `FD8FlowAccumulation`, `D8FlowAccumulation`, `Slope`
pub struct RusleLsFactor {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl RusleLsFactor {
    pub fn new() -> RusleLsFactor {
        let name = "RusleLsFactor".to_string();
        let toolbox = "Geomorphometric Analysis".to_string();
        let description =
            "Computes RUSLE LS using Desmet-Govers L and McCool/RUSLE S equations.".to_string();

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
            name: "Output LS Raster".to_owned(),
            flags: vec!["-o".to_owned(), "--output".to_owned()],
            description: "Output LS raster file.".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: false,
        });

        parameters.push(ToolParameter {
            name: "Output L Raster".to_owned(),
            flags: vec!["--l_output".to_owned()],
            description: "Output L raster file (defaults beside --output).".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output S Raster".to_owned(),
            flags: vec!["--s_output".to_owned()],
            description: "Output S raster file (defaults beside --output).".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output SCA Raster".to_owned(),
            flags: vec!["--sca_output".to_owned()],
            description: "Output SCA raster file (defaults beside --output).".to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Output Effective Slope Length Raster".to_owned(),
            flags: vec!["--effective_slope_length_output".to_owned()],
            description: "Output effective slope length raster file (defaults beside --output)."
                .to_owned(),
            parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Optional input SCA Raster".to_owned(),
            flags: vec!["--sca".to_owned()],
            description: "Optional input specific catchment area raster (m^2/m).".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Optional input slope degrees Raster".to_owned(),
            flags: vec!["--slope_deg".to_owned()],
            description: "Optional input slope raster in degrees.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Optional channel stop mask".to_owned(),
            flags: vec!["--channel_mask".to_owned()],
            description: "Optional channel mask where values > 0 are LS stop cells.".to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Optional blocking stop mask".to_owned(),
            flags: vec!["--blocking_mask".to_owned()],
            description:
                "Optional blocking mask where values > 0 are LS stop cells; nodata is pass-through."
                    .to_owned(),
            parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
            default_value: None,
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Routing mode".to_owned(),
            flags: vec!["--routing".to_owned()],
            description: "Routing mode; one of 'dinf' (default), 'fd8', 'd8'.".to_owned(),
            parameter_type: ParameterType::OptionList(vec![
                "dinf".to_owned(),
                "fd8".to_owned(),
                "d8".to_owned(),
            ]),
            default_value: Some("dinf".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "Maximum slope length (m)".to_owned(),
            flags: vec!["--max_slope_length_m".to_owned()],
            description:
                "Maximum effective slope length in meters; default 304.8 (RUSLE2 handbook 1000 ft)."
                    .to_owned(),
            parameter_type: ParameterType::Float,
            default_value: Some("304.8".to_owned()),
            optional: true,
        });

        parameters.push(ToolParameter {
            name: "m regime".to_owned(),
            flags: vec!["--m_regime".to_owned()],
            description: "m regime; one of 'slight', 'moderate' (default), 'high_rill'.".to_owned(),
            parameter_type: ParameterType::OptionList(vec![
                "slight".to_owned(),
                "moderate".to_owned(),
                "high_rill".to_owned(),
            ]),
            default_value: Some("moderate".to_owned()),
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
            ">>.*{0} -r={1} -v --wd=\"*path*to*data*\" --dem=dem.tif --output=ls.tif --routing=dinf --max_slope_length_m=304.8 --m_regime=moderate",
            short_exe, name
        )
        .replace("*", &sep);

        RusleLsFactor {
            name,
            description,
            toolbox,
            parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for RusleLsFactor {
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
        let mut dem_file = String::new();
        let mut output_file = String::new();
        let mut l_output_file = String::new();
        let mut s_output_file = String::new();
        let mut sca_output_file = String::new();
        let mut eff_output_file = String::new();

        let mut sca_input_file = String::new();
        let mut slope_input_file = String::new();
        let mut channel_mask_file = String::new();
        let mut blocking_mask_file = String::new();

        let mut routing_mode = "dinf".to_string();
        let mut max_slope_length_m = 304.8_f64;
        let mut m_regime = "moderate".to_string();

        if args.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no parameters.",
            ));
        }

        for i in 0..args.len() {
            let mut arg = args[i].replace('"', "");
            arg = arg.replace('\'', "");
            let cmd = arg.split('=');
            let vec = cmd.collect::<Vec<&str>>();
            let keyval = vec.len() > 1;
            let flag_val = vec[0].to_lowercase().replace("--", "-");

            if flag_val == "-i" || flag_val == "-dem" || flag_val == "-input" {
                dem_file = if keyval {
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
            } else if flag_val == "-l_output" {
                l_output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-s_output" {
                s_output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-sca_output" {
                sca_output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-effective_slope_length_output" {
                eff_output_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-sca" {
                sca_input_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-slope_deg" {
                slope_input_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-channel_mask" {
                channel_mask_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-blocking_mask" {
                blocking_mask_file = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
            } else if flag_val == "-routing" {
                routing_mode = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                routing_mode = normalize_routing_mode(&routing_mode)?;
            } else if flag_val == "-max_slope_length_m" {
                let parsed = if keyval {
                    vec[1].parse::<f64>().map_err(|_| {
                        Error::new(
                            ErrorKind::InvalidInput,
                            "Could not parse --max_slope_length_m as float.",
                        )
                    })?
                } else {
                    args[i + 1].parse::<f64>().map_err(|_| {
                        Error::new(
                            ErrorKind::InvalidInput,
                            "Could not parse --max_slope_length_m as float.",
                        )
                    })?
                };
                max_slope_length_m = parsed;
            } else if flag_val == "-m_regime" {
                m_regime = if keyval {
                    vec[1].to_string()
                } else {
                    args[i + 1].to_string()
                };
                m_regime = normalize_m_regime(&m_regime)?;
            }
        }

        if dem_file.trim().is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Input DEM not specified (--dem).",
            ));
        }

        if output_file.trim().is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Output LS raster not specified (--output).",
            ));
        }

        dem_file = resolve_path(&dem_file, working_directory);
        output_file = resolve_path(&output_file, working_directory);

        if l_output_file.trim().is_empty() {
            l_output_file = default_derived_path(&output_file, "l");
        }
        if s_output_file.trim().is_empty() {
            s_output_file = default_derived_path(&output_file, "s");
        }
        if sca_output_file.trim().is_empty() {
            sca_output_file = default_derived_path(&output_file, "sca");
        }
        if eff_output_file.trim().is_empty() {
            eff_output_file = default_derived_path(&output_file, "effective_slope_length");
        }

        l_output_file = resolve_path(&l_output_file, working_directory);
        s_output_file = resolve_path(&s_output_file, working_directory);
        sca_output_file = resolve_path(&sca_output_file, working_directory);
        eff_output_file = resolve_path(&eff_output_file, working_directory);

        if !sca_input_file.trim().is_empty() {
            sca_input_file = resolve_path(&sca_input_file, working_directory);
        }
        if !slope_input_file.trim().is_empty() {
            slope_input_file = resolve_path(&slope_input_file, working_directory);
        }
        if !channel_mask_file.trim().is_empty() {
            channel_mask_file = resolve_path(&channel_mask_file, working_directory);
        }
        if !blocking_mask_file.trim().is_empty() {
            blocking_mask_file = resolve_path(&blocking_mask_file, working_directory);
        }

        let start = Instant::now();

        let dem = Raster::new(&dem_file, "r")?;
        let rows = dem.configs.rows as isize;
        let columns = dem.configs.columns as isize;
        let dem_nodata = dem.configs.nodata;

        let channel_mask = if channel_mask_file.is_empty() {
            None
        } else {
            let raster = Raster::new(&channel_mask_file, "r")?;
            ensure_same_grid(&dem, &raster, "channel_mask")?;
            Some(raster)
        };

        let blocking_mask = if blocking_mask_file.is_empty() {
            None
        } else {
            let raster = Raster::new(&blocking_mask_file, "r")?;
            ensure_same_grid(&dem, &raster, "blocking_mask")?;
            Some(raster)
        };

        let stop_mask = build_stop_mask(
            rows,
            columns,
            &dem,
            channel_mask.as_ref(),
            blocking_mask.as_ref(),
        );

        let mut temp_paths: Vec<String> = vec![];
        let mut dem_for_derivation_file = dem_file.clone();
        let mut noflow_guard_status = "not_checked".to_string();
        let mut noflow_initial_count: usize = 0;
        let mut noflow_post_fallback_count: usize = 0;
        let mut noflow_eligible_count: usize = 0;

        let (sca_raster, sca_source) = if !sca_input_file.is_empty() {
            let raster = Raster::new(&sca_input_file, "r")?;
            ensure_same_grid(&dem, &raster, "sca")?;
            (raster, "input".to_string())
        } else {
            if routing_mode == "dinf" {
                let tmp_noflow = make_temp_raster_path(&output_file, "noflow_check");
                run_noflow_tool(&dem_for_derivation_file, &tmp_noflow, working_directory, verbose)?;
                let noflow = Raster::new(&tmp_noflow, "r")?;
                let noflow_stats =
                    count_interior_noflow(&dem, &noflow, rows, columns, &stop_mask);
                noflow_initial_count = noflow_stats.interior_count;
                noflow_post_fallback_count = noflow_stats.interior_count;
                noflow_eligible_count = noflow_stats.eligible_interior_cells;
                temp_paths.push(tmp_noflow);

                if noflow_stats.interior_count > 0 {
                    let fallback_threshold =
                        interior_noflow_fallback_threshold(noflow_stats.eligible_interior_cells);
                    if noflow_stats.interior_count > fallback_threshold {
                        return Err(Error::new(
                            ErrorKind::InvalidInput,
                            format!(
                                "DEM contains {} interior no-flow cells. Conservative fallback only applies to small defects (<= {} interior cells, {:.4} of eligible interior cells). Condition DEM upstream before running RusleLsFactor.",
                                noflow_stats.interior_count,
                                fallback_threshold,
                                INTERIOR_NOFLOW_FALLBACK_MAX_FRACTION,
                            ),
                        ));
                    }

                    let corrected_dem = make_temp_raster_path(&output_file, "dem_single_cell_breach");
                    run_breach_single_cell_pits_tool(
                        &dem_for_derivation_file,
                        &corrected_dem,
                        working_directory,
                        verbose,
                    )?;
                    temp_paths.push(corrected_dem.clone());

                    let tmp_corrected_noflow =
                        make_temp_raster_path(&output_file, "noflow_check_corrected");
                    run_noflow_tool(
                        &corrected_dem,
                        &tmp_corrected_noflow,
                        working_directory,
                        verbose,
                    )?;
                    let corrected_noflow = Raster::new(&tmp_corrected_noflow, "r")?;
                    let corrected_dem_raster = Raster::new(&corrected_dem, "r")?;
                    let corrected_stats = count_interior_noflow(
                        &corrected_dem_raster,
                        &corrected_noflow,
                        rows,
                        columns,
                        &stop_mask,
                    );
                    noflow_post_fallback_count = corrected_stats.interior_count;
                    noflow_eligible_count = corrected_stats.eligible_interior_cells;
                    temp_paths.push(tmp_corrected_noflow);

                    if corrected_stats.interior_count > 0 {
                        return Err(Error::new(
                            ErrorKind::InvalidInput,
                            format!(
                                "DEM contains {} interior no-flow cells; conservative single-cell pit fallback reduced this to {} but did not fully correct the DEM. Condition DEM upstream before running RusleLsFactor.",
                                noflow_stats.interior_count, corrected_stats.interior_count
                            ),
                        ));
                    }

                    noflow_guard_status = "breach_single_cell_pits".to_string();
                    dem_for_derivation_file = corrected_dem;
                    if verbose {
                        println!(
                            "Applied conservative single-cell pit fallback for {} interior no-flow cells.",
                            noflow_stats.interior_count
                        );
                    }
                } else {
                    noflow_guard_status = "none".to_string();
                }
            }

            let tmp = make_temp_raster_path(&output_file, "sca_work");
            run_sca_tool(
                &dem_for_derivation_file,
                &tmp,
                &routing_mode,
                working_directory,
                verbose,
            )?;
            let raster = Raster::new(&tmp, "r")?;
            ensure_same_grid(&dem, &raster, "derived_sca")?;
            temp_paths.push(tmp.clone());
            (raster, "derived".to_string())
        };

        let (slope_raster, slope_source) = if !slope_input_file.is_empty() {
            let raster = Raster::new(&slope_input_file, "r")?;
            ensure_same_grid(&dem, &raster, "slope_deg")?;
            (raster, "input".to_string())
        } else {
            let tmp = make_temp_raster_path(&output_file, "slope_work");
            run_slope_tool(&dem_for_derivation_file, &tmp, working_directory, verbose)?;
            let raster = Raster::new(&tmp, "r")?;
            ensure_same_grid(&dem, &raster, "derived_slope")?;
            temp_paths.push(tmp.clone());
            (raster, "derived".to_string())
        };

        let mut ls_output = Raster::initialize_using_file(&output_file, &dem);
        let mut l_output = Raster::initialize_using_file(&l_output_file, &dem);
        let mut s_output = Raster::initialize_using_file(&s_output_file, &dem);
        let mut sca_output = Raster::initialize_using_file(&sca_output_file, &dem);
        let mut eff_output = Raster::initialize_using_file(&eff_output_file, &dem);

        configure_float_output(&mut ls_output, dem_nodata);
        configure_float_output(&mut l_output, dem_nodata);
        configure_float_output(&mut s_output, dem_nodata);
        configure_float_output(&mut sca_output, dem_nodata);
        configure_float_output(&mut eff_output, dem_nodata);

        let regime_scale = m_regime_scale(&m_regime);

        let mut progress: usize;
        let mut old_progress: usize = 1;

        for row in 0..rows {
            for col in 0..columns {
                let z = dem[(row, col)];
                if z == dem_nodata {
                    continue;
                }

                let idx = (row * columns + col) as usize;
                if stop_mask[idx] {
                    continue;
                }

                let sca_nodata = sca_raster.configs.nodata;
                let slope_nodata = slope_raster.configs.nodata;
                let sca_val = sca_raster[(row, col)];
                let slope_deg = slope_raster[(row, col)];

                if sca_val == sca_nodata || slope_deg == slope_nodata {
                    continue;
                }

                let slope_rad = slope_deg.to_radians();
                let m = compute_m_from_slope_rad(slope_rad, regime_scale);
                let effective_slope_length =
                    apply_slope_length_cap(sca_val.max(0.0), max_slope_length_m);
                let l_val = compute_l_from_effective_slope_length(effective_slope_length, m);
                let s_val = compute_mccool_s_from_slope_rad(slope_rad);

                ls_output[(row, col)] = l_val * s_val;
                l_output[(row, col)] = l_val;
                s_output[(row, col)] = s_val;
                sca_output[(row, col)] = sca_val;
                eff_output[(row, col)] = effective_slope_length;
            }

            if verbose {
                progress = (100.0_f64 * row as f64 / (rows - 1).max(1) as f64) as usize;
                if progress != old_progress {
                    println!("Computing LS: {}%", progress);
                    old_progress = progress;
                }
            }
        }

        let mut stop_components: Vec<String> = Vec::new();
        if channel_mask.is_some() {
            stop_components.push("channel_mask".to_string());
        }
        if blocking_mask.is_some() {
            stop_components.push("blocking_mask".to_string());
        }
        let stop_mask_components = if stop_components.is_empty() {
            "none".to_string()
        } else {
            stop_components.join(",")
        };

        let blocking_source = if blocking_mask.is_some() {
            "input_raster"
        } else {
            "none"
        };

        let elapsed_time = get_formatted_elapsed_time(start);

        add_ls_metadata(
            &mut ls_output,
            &dem_file,
            &routing_mode,
            &m_regime,
            &sca_source,
            &slope_source,
            max_slope_length_m,
            &stop_mask_components,
            blocking_source,
            &noflow_guard_status,
            noflow_initial_count,
            noflow_post_fallback_count,
            noflow_eligible_count,
            &elapsed_time,
        );
        add_ls_metadata(
            &mut l_output,
            &dem_file,
            &routing_mode,
            &m_regime,
            &sca_source,
            &slope_source,
            max_slope_length_m,
            &stop_mask_components,
            blocking_source,
            &noflow_guard_status,
            noflow_initial_count,
            noflow_post_fallback_count,
            noflow_eligible_count,
            &elapsed_time,
        );
        add_ls_metadata(
            &mut s_output,
            &dem_file,
            &routing_mode,
            &m_regime,
            &sca_source,
            &slope_source,
            max_slope_length_m,
            &stop_mask_components,
            blocking_source,
            &noflow_guard_status,
            noflow_initial_count,
            noflow_post_fallback_count,
            noflow_eligible_count,
            &elapsed_time,
        );
        add_ls_metadata(
            &mut sca_output,
            &dem_file,
            &routing_mode,
            &m_regime,
            &sca_source,
            &slope_source,
            max_slope_length_m,
            &stop_mask_components,
            blocking_source,
            &noflow_guard_status,
            noflow_initial_count,
            noflow_post_fallback_count,
            noflow_eligible_count,
            &elapsed_time,
        );
        add_ls_metadata(
            &mut eff_output,
            &dem_file,
            &routing_mode,
            &m_regime,
            &sca_source,
            &slope_source,
            max_slope_length_m,
            &stop_mask_components,
            blocking_source,
            &noflow_guard_status,
            noflow_initial_count,
            noflow_post_fallback_count,
            noflow_eligible_count,
            &elapsed_time,
        );

        ls_output.write()?;
        l_output.write()?;
        s_output.write()?;
        sca_output.write()?;
        eff_output.write()?;

        for tmp in temp_paths {
            let _ = fs::remove_file(tmp);
        }

        if verbose {
            println!("Output files written");
            println!("Elapsed Time (excluding I/O): {}", elapsed_time);
        }

        Ok(())
    }
}

fn configure_float_output(raster: &mut Raster, nodata: f64) {
    raster.configs.data_type = DataType::F32;
    raster.configs.photometric_interp = PhotometricInterpretation::Continuous;
    raster.configs.nodata = nodata;
    raster.reinitialize_values(nodata);
}

fn add_ls_metadata(
    raster: &mut Raster,
    dem_file: &str,
    routing_mode: &str,
    m_regime: &str,
    sca_source: &str,
    slope_source: &str,
    max_slope_length_m: f64,
    stop_mask_components: &str,
    blocking_mask_source: &str,
    interior_noflow_fallback: &str,
    interior_noflow_cells_initial: usize,
    interior_noflow_cells_post_fallback: usize,
    interior_noflow_cells_eligible: usize,
    elapsed_time: &str,
) {
    raster.add_metadata_entry("tool = RusleLsFactor".to_string());
    raster.add_metadata_entry(format!("Input DEM: {}", dem_file));
    raster.add_metadata_entry("l_method = desmet_govers_1996".to_string());
    raster.add_metadata_entry("s_method = mccool_rusle_piecewise".to_string());
    raster.add_metadata_entry("m_method = mccool_1989_beta_moderate_base".to_string());
    raster.add_metadata_entry(format!("m_regime = {}", m_regime));
    raster.add_metadata_entry(format!("routing_mode = {}", routing_mode));
    raster.add_metadata_entry("dem_hydrologically_sound_assumed = true".to_string());
    raster.add_metadata_entry(format!("max_slope_length_m = {:.6}", max_slope_length_m));
    raster.add_metadata_entry("max_slope_length_basis = rusle2_handbook_1000ft".to_string());
    raster.add_metadata_entry(format!("stop_mask_components = {}", stop_mask_components));
    raster.add_metadata_entry(
        "stop_mask_routing_behavior = terminal_sink_no_renormalization".to_string(),
    );
    raster.add_metadata_entry(format!("sca_source = {}", sca_source));
    raster.add_metadata_entry(format!("slope_source = {}", slope_source));
    raster.add_metadata_entry(format!("blocking_mask_source = {}", blocking_mask_source));
    raster.add_metadata_entry(format!(
        "interior_noflow_fallback = {}",
        interior_noflow_fallback
    ));
    raster.add_metadata_entry(format!(
        "interior_noflow_cells_initial = {}",
        interior_noflow_cells_initial
    ));
    raster.add_metadata_entry(format!(
        "interior_noflow_cells_post_fallback = {}",
        interior_noflow_cells_post_fallback
    ));
    raster.add_metadata_entry(format!(
        "interior_noflow_cells_eligible = {}",
        interior_noflow_cells_eligible
    ));
    raster.add_metadata_entry(format!(
        "interior_noflow_fallback_max_count = {}",
        INTERIOR_NOFLOW_FALLBACK_MAX_COUNT
    ));
    raster.add_metadata_entry(format!(
        "interior_noflow_fallback_max_fraction = {:.6}",
        INTERIOR_NOFLOW_FALLBACK_MAX_FRACTION
    ));
    raster.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed_time));
}

fn ensure_same_grid(reference: &Raster, other: &Raster, label: &str) -> Result<(), Error> {
    if reference.configs.rows != other.configs.rows
        || reference.configs.columns != other.configs.columns
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!("{} raster grid dimensions do not match DEM.", label),
        ));
    }
    Ok(())
}

const INTERIOR_NOFLOW_FALLBACK_MAX_COUNT: usize = 64;
const INTERIOR_NOFLOW_FALLBACK_MAX_FRACTION: f64 = 0.001;

struct InteriorNoFlowStats {
    interior_count: usize,
    eligible_interior_cells: usize,
}

fn interior_noflow_fallback_threshold(eligible_interior_cells: usize) -> usize {
    if eligible_interior_cells == 0 {
        return 0;
    }
    let fractional_limit = ((eligible_interior_cells as f64) * INTERIOR_NOFLOW_FALLBACK_MAX_FRACTION)
        .ceil()
        .max(1.0) as usize;
    fractional_limit.min(INTERIOR_NOFLOW_FALLBACK_MAX_COUNT)
}

fn count_interior_noflow(
    dem: &Raster,
    noflow: &Raster,
    rows: isize,
    columns: isize,
    stop_mask: &[bool],
) -> InteriorNoFlowStats {
    let mut interior_count: usize = 0;
    let mut eligible_interior_cells: usize = 0;
    let dem_nodata = dem.configs.nodata;
    let noflow_nodata = noflow.configs.nodata;

    for row in 1..(rows - 1).max(1) {
        for col in 1..(columns - 1).max(1) {
            if dem[(row, col)] == dem_nodata {
                continue;
            }
            let idx = (row * columns + col) as usize;
            if stop_mask[idx] {
                continue;
            }
            eligible_interior_cells += 1;
            let v = noflow[(row, col)];
            if v != noflow_nodata && v > 0.0 {
                interior_count += 1;
            }
        }
    }
    InteriorNoFlowStats {
        interior_count,
        eligible_interior_cells,
    }
}

fn build_stop_mask(
    rows: isize,
    columns: isize,
    dem: &Raster,
    channel_mask: Option<&Raster>,
    blocking_mask: Option<&Raster>,
) -> Vec<bool> {
    let mut stop_mask = vec![false; (rows * columns) as usize];
    let dem_nodata = dem.configs.nodata;

    for row in 0..rows {
        for col in 0..columns {
            if dem[(row, col)] == dem_nodata {
                continue;
            }
            let mut stop = false;

            if let Some(mask) = channel_mask {
                let v = mask[(row, col)];
                if v != mask.configs.nodata && v > 0.0 {
                    stop = true;
                }
            }

            if let Some(mask) = blocking_mask {
                let v = mask[(row, col)];
                if v != mask.configs.nodata && v > 0.0 {
                    stop = true;
                }
            }

            stop_mask[(row * columns + col) as usize] = stop;
        }
    }

    stop_mask
}

fn normalize_routing_mode(value: &str) -> Result<String, Error> {
    let v = value.trim().to_lowercase();
    match v.as_str() {
        "dinf" | "fd8" | "d8" => Ok(v),
        _ => Err(Error::new(
            ErrorKind::InvalidInput,
            "Invalid --routing value. Expected one of: dinf, fd8, d8.",
        )),
    }
}

fn normalize_m_regime(value: &str) -> Result<String, Error> {
    let v = value.trim().to_lowercase();
    match v.as_str() {
        "slight" | "moderate" | "high_rill" => Ok(v),
        _ => Err(Error::new(
            ErrorKind::InvalidInput,
            "Invalid --m_regime value. Expected one of: slight, moderate, high_rill.",
        )),
    }
}

fn m_regime_scale(m_regime: &str) -> f64 {
    match m_regime {
        "slight" => 0.5,
        "high_rill" => 2.0,
        _ => 1.0,
    }
}

fn compute_m_from_slope_rad(slope_rad: f64, regime_scale: f64) -> f64 {
    let sin_theta = slope_rad.sin().max(0.0);
    if sin_theta <= 0.0 {
        return 0.0;
    }
    let beta_base = (sin_theta / 0.0896) / (3.0 * sin_theta.powf(0.8) + 0.56);
    let beta = beta_base * regime_scale;
    if beta <= 0.0 {
        0.0
    } else {
        beta / (1.0 + beta)
    }
}

fn compute_mccool_s_from_slope_rad(slope_rad: f64) -> f64 {
    let sin_theta = slope_rad.sin().max(0.0);
    let tan_theta = slope_rad.tan().abs();
    if tan_theta < 0.09 {
        (10.8 * sin_theta + 0.03).max(0.0)
    } else {
        (16.8 * sin_theta - 0.50).max(0.0)
    }
}

fn apply_slope_length_cap(value_m: f64, max_slope_length_m: f64) -> f64 {
    if max_slope_length_m.is_finite() && max_slope_length_m > 0.0 && value_m > max_slope_length_m {
        max_slope_length_m
    } else {
        value_m
    }
}

fn compute_l_from_effective_slope_length(effective_slope_length_m: f64, m: f64) -> f64 {
    if effective_slope_length_m <= 0.0 {
        0.0
    } else {
        (effective_slope_length_m / 22.13).powf(m)
    }
}

fn run_sca_tool(
    dem_file: &str,
    sca_output_file: &str,
    routing_mode: &str,
    working_directory: &str,
    verbose: bool,
) -> Result<(), Error> {
    match routing_mode {
        "dinf" => {
            let tool = DInfFlowAccumulation::new();
            let args = vec![
                format!("--input={}", dem_file),
                format!("--output={}", sca_output_file),
                "--out_type=sca".to_string(),
            ];
            tool.run(args, working_directory, verbose)?;
        }
        "fd8" => {
            let tool = FD8FlowAccumulation::new();
            let args = vec![
                format!("--dem={}", dem_file),
                format!("--output={}", sca_output_file),
                "--out_type=specific contributing area".to_string(),
            ];
            tool.run(args, working_directory, verbose)?;
        }
        "d8" => {
            let tool = D8FlowAccumulation::new();
            let args = vec![
                format!("--input={}", dem_file),
                format!("--output={}", sca_output_file),
                "--out_type=specific contributing area".to_string(),
            ];
            tool.run(args, working_directory, verbose)?;
        }
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Unsupported routing mode for SCA derivation.",
            ))
        }
    }
    Ok(())
}

fn run_slope_tool(
    dem_file: &str,
    slope_output_file: &str,
    working_directory: &str,
    verbose: bool,
) -> Result<(), Error> {
    let tool = Slope::new();
    let args = vec![
        format!("--dem={}", dem_file),
        format!("--output={}", slope_output_file),
        "--units=degrees".to_string(),
    ];
    tool.run(args, working_directory, verbose)?;
    Ok(())
}

fn run_noflow_tool(
    dem_file: &str,
    noflow_output_file: &str,
    working_directory: &str,
    verbose: bool,
) -> Result<(), Error> {
    let tool = FindNoFlowCells::new();
    let args = vec![
        format!("--dem={}", dem_file),
        format!("--output={}", noflow_output_file),
    ];
    tool.run(args, working_directory, verbose)?;
    Ok(())
}

fn run_breach_single_cell_pits_tool(
    dem_file: &str,
    corrected_dem_file: &str,
    working_directory: &str,
    verbose: bool,
) -> Result<(), Error> {
    let tool = BreachSingleCellPits::new();
    let args = vec![
        format!("--dem={}", dem_file),
        format!("--output={}", corrected_dem_file),
    ];
    tool.run(args, working_directory, verbose)?;
    Ok(())
}

fn resolve_path(path_value: &str, working_directory: &str) -> String {
    let sep = path::MAIN_SEPARATOR.to_string();
    if path_value.contains(&sep) || path_value.contains('/') {
        path_value.to_string()
    } else {
        format!("{}{}", working_directory, path_value)
    }
}

fn default_derived_path(output_file: &str, stem_suffix: &str) -> String {
    let p = Path::new(output_file);
    let parent = p.parent().unwrap_or_else(|| Path::new("."));
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("rusle_ls");
    let ext = p.extension().and_then(|e| e.to_str()).unwrap_or("tif");
    parent
        .join(format!("{}_{}.{}", stem, stem_suffix, ext))
        .to_string_lossy()
        .to_string()
}

fn make_temp_raster_path(output_file: &str, tag: &str) -> String {
    let p = Path::new(output_file);
    let parent = p.parent().unwrap_or_else(|| Path::new("."));
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("rusle_ls");
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    parent
        .join(format!(
            "{}_{}_{}_{}.tif",
            stem,
            tag,
            std::process::id(),
            now
        ))
        .to_string_lossy()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_routing_mode_parse() {
        assert_eq!(normalize_routing_mode("DINF").unwrap(), "dinf");
        assert_eq!(normalize_routing_mode("fd8").unwrap(), "fd8");
        assert!(normalize_routing_mode("bad").is_err());
    }

    #[test]
    fn test_m_regime_parse_and_scale() {
        assert_eq!(normalize_m_regime("moderate").unwrap(), "moderate");
        assert_eq!(m_regime_scale("slight"), 0.5);
        assert_eq!(m_regime_scale("moderate"), 1.0);
        assert_eq!(m_regime_scale("high_rill"), 2.0);
        assert!(normalize_m_regime("unknown").is_err());
    }

    #[test]
    fn test_default_derived_path_stems() {
        let p = default_derived_path("/tmp/ls.tif", "l");
        assert!(p.ends_with("ls_l.tif"));
    }

    #[test]
    fn test_mccool_s_branch_split() {
        let low = 5.0_f64.to_radians();
        let high = 10.0_f64.to_radians();
        let low_expected = (10.8 * low.sin() + 0.03).max(0.0);
        let high_expected = (16.8 * high.sin() - 0.50).max(0.0);
        assert!((compute_mccool_s_from_slope_rad(low) - low_expected).abs() < 1.0e-12);
        assert!((compute_mccool_s_from_slope_rad(high) - high_expected).abs() < 1.0e-12);
    }

    #[test]
    fn test_m_regime_influences_m() {
        let slope_rad = 10.0_f64.to_radians();
        let m_slight = compute_m_from_slope_rad(slope_rad, m_regime_scale("slight"));
        let m_moderate = compute_m_from_slope_rad(slope_rad, m_regime_scale("moderate"));
        let m_high = compute_m_from_slope_rad(slope_rad, m_regime_scale("high_rill"));
        assert!(m_slight < m_moderate);
        assert!(m_moderate < m_high);
    }

    #[test]
    fn test_slope_length_cap_and_l_formula() {
        assert_eq!(apply_slope_length_cap(500.0, 304.8), 304.8);
        assert_eq!(apply_slope_length_cap(120.0, 304.8), 120.0);
        let l = compute_l_from_effective_slope_length(22.13, 0.5);
        assert!((l - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn test_interior_noflow_fallback_threshold_policy() {
        assert_eq!(interior_noflow_fallback_threshold(0), 0);
        assert_eq!(interior_noflow_fallback_threshold(1), 1);
        assert_eq!(interior_noflow_fallback_threshold(100), 1);
        assert_eq!(interior_noflow_fallback_threshold(5_000), 5);
        assert_eq!(interior_noflow_fallback_threshold(1_000_000), 64);
    }
}
