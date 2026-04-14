use super::*;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

fn base_args() -> Vec<String> {
    vec![
        "--d8_pntr=d8.tif".to_string(),
        "--upstream_area=area.tif".to_string(),
        "--output=streams.tif".to_string(),
        "--csa=10.0".to_string(),
        "--mscl=100.0".to_string(),
    ]
}

fn temp_threshold_table_path(stem: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("ifolp_{}_{}_{}.csv", stem, process::id(), nanos))
}

fn temp_threshold_code_raster_path(stem: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("ifolp_{}_{}_{}.tif", stem, process::id(), nanos))
}

fn temp_output_raster_path(stem: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("ifolp_{}_{}_{}.tif", stem, process::id(), nanos))
}

fn cleanup_whitebox_raster_artifacts(path: &PathBuf) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(PathBuf::from(format!("{}.aux.xml", path.to_string_lossy())));
}

fn first_active_cell(d8: &Raster, upstream_area: &Raster) -> Option<(isize, isize)> {
    let d8_nodata = d8.configs.nodata;
    let area_nodata = upstream_area.configs.nodata;
    for row in 0..d8.configs.rows as isize {
        for col in 0..d8.configs.columns as isize {
            if d8[(row, col)] != d8_nodata && upstream_area[(row, col)] != area_nodata {
                return Some((row, col));
            }
        }
    }
    None
}

fn write_threshold_code_raster_with_active_nodata(
    d8_path: &PathBuf,
    upstream_area_path: &PathBuf,
    output_path: &PathBuf,
) {
    let d8 = Raster::new(&d8_path.to_string_lossy(), "r").expect("d8 fixture should open");
    let upstream_area = Raster::new(&upstream_area_path.to_string_lossy(), "r")
        .expect("upstream-area fixture should open");
    let (active_row, active_col) =
        first_active_cell(&d8, &upstream_area).expect("fixture should include active cells");

    let mut threshold_codes = Raster::initialize_using_file(&output_path.to_string_lossy(), &d8);
    threshold_codes.reinitialize_values(1.0);
    threshold_codes.configs.nodata = -9999.0;
    threshold_codes.set_value(active_row, active_col, threshold_codes.configs.nodata);
    threshold_codes
        .write()
        .expect("temporary threshold-code raster should write");
}

fn synthetic_raster_with_geometry(
    rows: usize,
    columns: usize,
    west: f64,
    east: f64,
    south: f64,
    north: f64,
    resolution_x: f64,
    resolution_y: f64,
) -> Raster {
    let mut raster = Raster::default();
    raster.configs.rows = rows;
    raster.configs.columns = columns;
    raster.configs.west = west;
    raster.configs.east = east;
    raster.configs.south = south;
    raster.configs.north = north;
    raster.configs.resolution_x = resolution_x;
    raster.configs.resolution_y = resolution_y;
    raster
}

#[test]
fn iterative_first_order_link_prune_parser_defaults_are_applied() {
    let parsed = parse_arguments(&base_args(), "/tmp/wd/").expect("parse should succeed");

    assert_eq!(parsed.d8_pntr, "/tmp/wd/d8.tif");
    assert_eq!(parsed.upstream_area, "/tmp/wd/area.tif");
    assert_eq!(parsed.output, "/tmp/wd/streams.tif");
    assert_eq!(parsed.csa, 10.0);
    assert_eq!(parsed.mscl, 100.0);
    assert_eq!(parsed.threshold_code_raster, None);
    assert_eq!(parsed.threshold_table, None);
    assert!(!parsed.esri_pntr);
    assert!((parsed.epsilon - DEFAULT_EPSILON).abs() < f64::EPSILON);
    assert_eq!(
        parsed.fail_if_only_channel_pruned,
        DEFAULT_FAIL_IF_ONLY_CHANNEL_PRUNED
    );
}

#[test]
fn iterative_first_order_link_prune_parser_required_args_enforced() {
    let mut missing_d8 = base_args();
    missing_d8.remove(0);
    let err = parse_arguments(&missing_d8, "").expect_err("missing d8 should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("--d8_pntr"));

    let mut missing_csa = base_args();
    missing_csa.retain(|a| !a.starts_with("--csa"));
    let err = parse_arguments(&missing_csa, "").expect_err("missing csa should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("--csa"));
}

#[test]
fn iterative_first_order_link_prune_parser_rejects_missing_value_before_next_flag() {
    let mut args = base_args();
    args[0] = "--d8_pntr".to_string();

    let err = parse_arguments(&args, "")
        .expect_err("missing d8 value followed by another flag should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("--d8_pntr"));
}

#[test]
fn iterative_first_order_link_prune_parser_preserves_inner_quotes_in_values() {
    let mut args = base_args();
    args[2] = "--output=o'brien-streams.tif".to_string();

    let parsed = parse_arguments(&args, "/tmp/wd/").expect("parse should succeed");
    assert_eq!(parsed.output, "/tmp/wd/o'brien-streams.tif".to_string());
}

#[test]
fn iterative_first_order_link_prune_parser_accepts_space_separated_signed_positive_numeric_values()
{
    let mut args = base_args();
    args[3] = "--csa".to_string();
    args.insert(4, "+1.5".to_string());
    args[5] = "--mscl=+100.0".to_string();
    args.push("--epsilon".to_string());
    args.push("+0.25".to_string());

    let parsed = parse_arguments(&args, "/tmp/wd/").expect("parse should succeed");
    assert_eq!(parsed.csa, 1.5);
    assert_eq!(parsed.mscl, 100.0);
    assert!((parsed.epsilon - 0.25).abs() < f64::EPSILON);
}

#[test]
fn iterative_first_order_link_prune_parser_rejects_non_positive_csa_and_negative_mscl() {
    let mut zero_csa = base_args();
    zero_csa[3] = "--csa=0.0".to_string();
    let err = parse_arguments(&zero_csa, "").expect_err("non-positive csa should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("--csa"));
    assert!(err.to_string().contains("positive"));

    let mut negative_csa = base_args();
    negative_csa[3] = "--csa=-1.0".to_string();
    let err = parse_arguments(&negative_csa, "").expect_err("negative csa should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("--csa"));
    assert!(err.to_string().contains("positive"));

    let mut negative_mscl = base_args();
    negative_mscl[4] = "--mscl=-10.0".to_string();
    let err = parse_arguments(&negative_mscl, "").expect_err("negative mscl should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("--mscl"));
    assert!(err.to_string().contains("non-negative"));
}

#[test]
fn iterative_first_order_link_prune_parser_numeric_value_missing_before_next_flag_fails() {
    let mut args = base_args();
    args[3] = "--csa".to_string();

    let err = parse_arguments(&args, "")
        .expect_err("missing numeric value followed by another flag should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("--csa"));
}

#[test]
fn iterative_first_order_link_prune_parser_rejects_negative_epsilon() {
    let mut args = base_args();
    args.push("--epsilon=-0.00001".to_string());

    let err = parse_arguments(&args, "").expect_err("negative epsilon should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("--epsilon"));
}

#[test]
fn iterative_first_order_link_prune_parser_rejects_non_finite_numeric_values() {
    let mut nan_epsilon = base_args();
    nan_epsilon.push("--epsilon=NaN".to_string());
    let err = parse_arguments(&nan_epsilon, "").expect_err("non-finite epsilon should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("--epsilon"));
    assert!(err.to_string().contains("finite"));

    let mut inf_csa = base_args();
    inf_csa[3] = "--csa=inf".to_string();
    let err = parse_arguments(&inf_csa, "").expect_err("non-finite csa should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("--csa"));
    assert!(err.to_string().contains("finite"));

    let mut nan_mscl = base_args();
    nan_mscl[4] = "--mscl=NaN".to_string();
    let err = parse_arguments(&nan_mscl, "").expect_err("non-finite mscl should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("--mscl"));
    assert!(err.to_string().contains("finite"));
}

#[test]
fn iterative_first_order_link_prune_parser_optional_overrides() {
    let mut args = base_args();
    args.push("--threshold_code_raster=codes.tif".to_string());
    args.push("--threshold_table=thresholds.csv".to_string());
    args.push("--esri_pntr".to_string());
    args.push("--epsilon=0.25".to_string());
    args.push("--fail_if_only_channel_pruned=false".to_string());

    let parsed = parse_arguments(&args, "/tmp/wd/").expect("parse should succeed");
    assert_eq!(
        parsed.threshold_code_raster,
        Some("/tmp/wd/codes.tif".to_string())
    );
    assert_eq!(
        parsed.threshold_table,
        Some("/tmp/wd/thresholds.csv".to_string())
    );
    assert!(parsed.esri_pntr);
    assert!((parsed.epsilon - 0.25).abs() < f64::EPSILON);
    assert!(!parsed.fail_if_only_channel_pruned);
}

#[test]
fn iterative_first_order_link_prune_parser_space_separated_bool_values() {
    let mut args = base_args();
    args.push("--esri_pntr".to_string());
    args.push("false".to_string());
    args.push("--fail_if_only_channel_pruned".to_string());
    args.push("false".to_string());

    let parsed = parse_arguments(&args, "/tmp/wd/").expect("parse should succeed");
    assert!(!parsed.esri_pntr);
    assert!(!parsed.fail_if_only_channel_pruned);
}

#[test]
fn iterative_first_order_link_prune_parser_threshold_inputs_must_be_paired() {
    let mut raster_only = base_args();
    raster_only.push("--threshold_code_raster=codes.tif".to_string());
    let err = parse_arguments(&raster_only, "/tmp/wd/")
        .expect_err("threshold_code_raster without threshold_table should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("--threshold_code_raster"));
    assert!(err.to_string().contains("--threshold_table"));

    let mut table_only = base_args();
    table_only.push("--threshold_table=thresholds.csv".to_string());
    let err = parse_arguments(&table_only, "/tmp/wd/")
        .expect_err("threshold_table without threshold_code_raster should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("--threshold_code_raster"));
    assert!(err.to_string().contains("--threshold_table"));
}

#[test]
fn iterative_first_order_link_prune_parser_boolean_flags_default_true_when_bare() {
    let mut args = base_args();
    args.push("--esri_pntr".to_string());
    args.push("--fail_if_only_channel_pruned".to_string());

    let parsed = parse_arguments(&args, "/tmp/wd/").expect("parse should succeed");
    assert!(parsed.esri_pntr);
    assert!(parsed.fail_if_only_channel_pruned);
}

#[test]
fn iterative_first_order_link_prune_prepare_phase_inputs_retains_zero_pointer_cells_in_active_domain(
) {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let d8_path = repo_root.join("test_fixtures/blackwood_60_5/flovec.tif");
    let upstream_area_path = repo_root.join("test_fixtures/blackwood_60_5/floaccum.tif");
    let output_path = std::env::temp_dir().join("ifolp_prepare_phase_inputs_zero_pointer.tif");

    let args = vec![
        format!("--d8_pntr={}", d8_path.display()),
        format!("--upstream_area={}", upstream_area_path.display()),
        format!("--output={}", output_path.display()),
        "--csa=60.0".to_string(),
        "--mscl=5.0".to_string(),
    ];

    let parsed = parse_arguments(&args, "").expect("parse should succeed");
    let prepared = prepare_phase_inputs(&parsed).expect("input preparation should succeed");

    let zero_pointer_cells = prepared
        .pointers
        .iter()
        .filter(|&&value| value == 0)
        .count();
    let active_zero_pointer_cells = prepared
        .pointers
        .iter()
        .zip(prepared.active_mask.iter())
        .filter(|(value, is_active)| **value == 0 && **is_active)
        .count();

    assert!(
        zero_pointer_cells > 0,
        "fixture should include zero-coded pointer cells for regression coverage"
    );
    assert_eq!(
        active_zero_pointer_cells, zero_pointer_cells,
        "zero-coded pointer cells should remain in the valid active-domain mask footprint"
    );
}

#[test]
fn iterative_first_order_link_prune_prepare_phase_inputs_rejects_unmapped_threshold_code() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let d8_path = repo_root.join("test_fixtures/blackwood_60_5/flovec.tif");
    let upstream_area_path = repo_root.join("test_fixtures/blackwood_60_5/floaccum.tif");
    let output_path = std::env::temp_dir().join("ifolp_prepare_phase_inputs_unmapped_code.tif");
    let threshold_table_path = temp_threshold_table_path("threshold_code_unmapped");
    fs::write(&threshold_table_path, "code,csa_ha,mscl_m\n999,60,5\n").expect("table should write");

    let args = vec![
        format!("--d8_pntr={}", d8_path.display()),
        format!("--upstream_area={}", upstream_area_path.display()),
        format!("--output={}", output_path.display()),
        "--csa=60.0".to_string(),
        "--mscl=5.0".to_string(),
        format!("--threshold_code_raster={}", d8_path.display()),
        format!("--threshold_table={}", threshold_table_path.display()),
    ];

    let parsed = parse_arguments(&args, "").expect("parse should succeed");
    let err = match prepare_phase_inputs(&parsed) {
        Ok(_) => panic!("unmapped threshold code in active domain should fail"),
        Err(err) => err,
    };
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err
        .to_string()
        .contains("No threshold table entry for code"));

    fs::remove_file(&threshold_table_path).expect("temporary table should be removable");
}

#[test]
fn iterative_first_order_link_prune_prepare_phase_inputs_rejects_threshold_code_nodata_at_active_cell(
) {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let d8_path = repo_root.join("test_fixtures/blackwood_60_5/flovec.tif");
    let upstream_area_path = repo_root.join("test_fixtures/blackwood_60_5/floaccum.tif");
    let output_path =
        std::env::temp_dir().join("ifolp_prepare_phase_inputs_threshold_nodata_active.tif");
    let threshold_table_path = temp_threshold_table_path("threshold_code_nodata_active");
    let threshold_code_path = temp_threshold_code_raster_path("threshold_codes_active_nodata");

    write_threshold_code_raster_with_active_nodata(
        &d8_path,
        &upstream_area_path,
        &threshold_code_path,
    );
    fs::write(&threshold_table_path, "code,csa_ha,mscl_m\n1,60,5\n").expect("table should write");

    let args = vec![
        format!("--d8_pntr={}", d8_path.display()),
        format!("--upstream_area={}", upstream_area_path.display()),
        format!("--output={}", output_path.display()),
        "--csa=60.0".to_string(),
        "--mscl=5.0".to_string(),
        format!("--threshold_code_raster={}", threshold_code_path.display()),
        format!("--threshold_table={}", threshold_table_path.display()),
    ];

    let parsed = parse_arguments(&args, "").expect("parse should succeed");
    let err = match prepare_phase_inputs(&parsed) {
        Ok(_) => panic!("threshold-code nodata at active cells should fail"),
        Err(err) => err,
    };
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err
        .to_string()
        .contains("Threshold code missing at active cell"));

    fs::remove_file(&threshold_table_path).expect("temporary table should be removable");
    cleanup_whitebox_raster_artifacts(&threshold_code_path);
}

#[test]
fn iterative_first_order_link_prune_run_integration_writes_binary_stream_output() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let source_d8_path = repo_root.join("test_fixtures/blackwood_60_5/flovec.tif");
    let source_upstream_area_path = repo_root.join("test_fixtures/blackwood_60_5/floaccum.tif");
    let d8_path = temp_output_raster_path("ifolp_run_integration_d8");
    let upstream_area_path = temp_output_raster_path("ifolp_run_integration_area");
    let output_path = temp_output_raster_path("ifolp_run_integration");

    let source_d8 =
        Raster::new(&source_d8_path.to_string_lossy(), "r").expect("source d8 fixture should open");
    let source_upstream_area = Raster::new(&source_upstream_area_path.to_string_lossy(), "r")
        .expect("source upstream-area fixture should open");
    let mut d8 = Raster::initialize_using_file(&d8_path.to_string_lossy(), &source_d8);
    let mut upstream_area =
        Raster::initialize_using_file(&upstream_area_path.to_string_lossy(), &source_upstream_area);

    let d8_nodata = source_d8.configs.nodata;
    let area_nodata = source_upstream_area.configs.nodata;
    d8.reinitialize_values(d8_nodata);
    upstream_area.reinitialize_values(area_nodata);

    let mut retained_active = 0usize;
    for row in 0..source_d8.configs.rows as isize {
        for col in 0..source_d8.configs.columns as isize {
            let pointer_value = source_d8[(row, col)];
            let area_value = source_upstream_area[(row, col)];
            if pointer_value == d8_nodata || area_value == area_nodata {
                continue;
            }

            let pointer_code = parse_integer_raster_value(pointer_value, "--d8_pntr", row, col)
                .expect("source fixture pointer should be parseable");
            if pointer_code == 0 {
                continue;
            }

            d8.set_value(row, col, pointer_value);
            upstream_area.set_value(row, col, area_value);
            retained_active += 1;
        }
    }
    assert!(
        retained_active > 0,
        "sanitized integration fixture should retain active non-zero pointer cells"
    );
    d8.write()
        .expect("sanitized d8 raster should write for integration test");
    upstream_area
        .write()
        .expect("sanitized upstream-area raster should write for integration test");

    let tool = IterativeFirstOrderLinkPrune::new();
    let args = vec![
        format!("--d8_pntr={}", d8_path.display()),
        format!("--upstream_area={}", upstream_area_path.display()),
        format!("--output={}", output_path.display()),
        "--csa=60.0".to_string(),
        "--mscl=5.0".to_string(),
    ];
    tool.run(args, "", false)
        .expect("tool run integration should succeed");

    let output = Raster::new(&output_path.to_string_lossy(), "r")
        .expect("integration output raster should open");
    let nodata = output.configs.nodata;
    let mut active_stream_count = 0usize;
    for row in 0..output.configs.rows as isize {
        for col in 0..output.configs.columns as isize {
            let value = output[(row, col)];
            if value == nodata {
                continue;
            }
            assert!(
                (value - 0.0).abs() < f64::EPSILON || (value - 1.0).abs() < f64::EPSILON,
                "output value at ({}, {}) must be binary 0/1 or NoData, got {}",
                row,
                col,
                value
            );
            if (value - 1.0).abs() < f64::EPSILON {
                active_stream_count += 1;
            }
        }
    }
    assert!(
        active_stream_count > 0,
        "integration run should retain stream cells"
    );

    cleanup_whitebox_raster_artifacts(&d8_path);
    cleanup_whitebox_raster_artifacts(&upstream_area_path);
    cleanup_whitebox_raster_artifacts(&output_path);
}

#[test]
fn iterative_first_order_link_prune_help_contract_contains_all_flags() {
    let tool = IterativeFirstOrderLinkPrune::new();
    let params_json = tool.get_tool_parameters();
    let parsed: Value = serde_json::from_str(&params_json).expect("valid parameters json");
    let parameters = parsed["parameters"]
        .as_array()
        .expect("parameters array expected");

    let mut flags: Vec<String> = vec![];
    for p in parameters {
        let entries = p["flags"].as_array().expect("flags array expected");
        for f in entries {
            flags.push(f.as_str().unwrap().to_string());
        }
    }

    for required_flag in [
        "--d8_pntr",
        "--upstream_area",
        "--output",
        "--csa",
        "--mscl",
        "--threshold_code_raster",
        "--threshold_table",
        "--esri_pntr",
        "--epsilon",
        "--fail_if_only_channel_pruned",
    ] {
        assert!(
            flags.contains(&required_flag.to_string()),
            "missing expected flag {}",
            required_flag
        );
    }
}

#[test]
fn iterative_first_order_link_prune_csa_conversion_rounds_to_nearest_cell() {
    let cells = csa_hectares_to_cells(60.0, 900.0).expect("conversion should succeed");
    assert_eq!(cells, 667.0);
}

#[test]
fn iterative_first_order_link_prune_csa_conversion_clamps_to_minimum_one_cell() {
    let cells = csa_hectares_to_cells(0.0, 900.0).expect("conversion should succeed");
    assert_eq!(cells, 1.0);
}

#[test]
fn iterative_first_order_link_prune_threshold_table_rejects_duplicate_codes() {
    let table_path = temp_threshold_table_path("threshold_duplicate");
    fs::write(&table_path, "1,10,100\n1,20,200\n").expect("table should write");

    let err = parse_threshold_table(&table_path.to_string_lossy())
        .expect_err("duplicate threshold codes should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("Duplicate threshold table code 1"));

    fs::remove_file(&table_path).expect("temporary table should be removable");
}

#[test]
fn iterative_first_order_link_prune_threshold_table_rejects_non_finite_values() {
    let table_path = temp_threshold_table_path("threshold_non_finite");
    fs::write(&table_path, "code,csa_ha,mscl_m\n1,NaN,100\n").expect("table should write");

    let err = parse_threshold_table(&table_path.to_string_lossy())
        .expect_err("non-finite threshold row should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err
        .to_string()
        .contains("must use finite csa_ha and mscl_m values"));

    fs::remove_file(&table_path).expect("temporary table should be removable");
}

#[test]
fn iterative_first_order_link_prune_threshold_table_rejects_non_physical_threshold_values() {
    let table_path = temp_threshold_table_path("threshold_non_physical");
    fs::write(&table_path, "code,csa_ha,mscl_m\n1,0,100\n").expect("table should write");
    let err = parse_threshold_table(&table_path.to_string_lossy())
        .expect_err("non-positive csa_ha should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("csa_ha > 0"));
    fs::remove_file(&table_path).expect("temporary table should be removable");

    let table_path = temp_threshold_table_path("threshold_negative_mscl");
    fs::write(&table_path, "code,csa_ha,mscl_m\n1,10,-1\n").expect("table should write");
    let err = parse_threshold_table(&table_path.to_string_lossy())
        .expect_err("negative mscl_m should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("mscl_m >= 0"));
    fs::remove_file(&table_path).expect("temporary table should be removable");
}

#[test]
fn iterative_first_order_link_prune_threshold_table_rejects_non_header_parse_error_on_first_line() {
    let table_path = temp_threshold_table_path("threshold_first_line_parse_error");
    fs::write(&table_path, "not_a_code,10,100\n2,20,200\n").expect("table should write");

    let err = parse_threshold_table(&table_path.to_string_lossy())
        .expect_err("non-header first-line parse errors must not be silently skipped");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err
        .to_string()
        .contains("Threshold table parse error at line 1"));

    fs::remove_file(&table_path).expect("temporary table should be removable");
}

#[test]
fn iterative_first_order_link_prune_threshold_table_accepts_supported_header_aliases() {
    let table_path = temp_threshold_table_path("threshold_header_aliases");
    fs::write(&table_path, "threshold_code,csa,mscl\n1,12.5,100\n").expect("table should write");

    let parsed =
        parse_threshold_table(&table_path.to_string_lossy()).expect("header aliases should parse");
    let entry = parsed.get(&1).expect("table must contain parsed code row");
    assert!((entry.csa_ha - 12.5).abs() < f64::EPSILON);
    assert!((entry.mscl_m - 100.0).abs() < f64::EPSILON);

    fs::remove_file(&table_path).expect("temporary table should be removable");
}

#[test]
fn iterative_first_order_link_prune_threshold_table_accepts_header_after_comments() {
    let table_path = temp_threshold_table_path("threshold_header_after_comments");
    fs::write(
        &table_path,
        "# generated by integration harness\n\ncode,csa_ha,mscl_m\n1,9.5,42\n",
    )
    .expect("table should write");

    let parsed = parse_threshold_table(&table_path.to_string_lossy())
        .expect("header after comments should parse");
    let entry = parsed.get(&1).expect("table must contain parsed code row");
    assert!((entry.csa_ha - 9.5).abs() < f64::EPSILON);
    assert!((entry.mscl_m - 42.0).abs() < f64::EPSILON);

    fs::remove_file(&table_path).expect("temporary table should be removable");
}

#[test]
fn iterative_first_order_link_prune_threshold_table_accepts_whitespace_delimited_rows() {
    let table_path = temp_threshold_table_path("threshold_whitespace_rows");
    fs::write(&table_path, "code csa_ha mscl_m\n1 12.5 100\n2 8.0 25\n")
        .expect("table should write");

    let parsed = parse_threshold_table(&table_path.to_string_lossy())
        .expect("whitespace-delimited threshold table should parse");
    let first = parsed.get(&1).expect("table must contain first code row");
    let second = parsed.get(&2).expect("table must contain second code row");
    assert!((first.csa_ha - 12.5).abs() < f64::EPSILON);
    assert!((first.mscl_m - 100.0).abs() < f64::EPSILON);
    assert!((second.csa_ha - 8.0).abs() < f64::EPSILON);
    assert!((second.mscl_m - 25.0).abs() < f64::EPSILON);

    fs::remove_file(&table_path).expect("temporary table should be removable");
}

#[test]
fn iterative_first_order_link_prune_threshold_table_rejects_whitespace_rows_with_extra_fields() {
    let table_path = temp_threshold_table_path("threshold_whitespace_extra_fields");
    fs::write(&table_path, "code csa_ha mscl_m\n1 12.5 100 unexpected\n")
        .expect("table should write");

    let err = parse_threshold_table(&table_path.to_string_lossy())
        .expect_err("whitespace rows with extra fields should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err
        .to_string()
        .contains("must contain exactly code,csa_ha,mscl_m"));

    fs::remove_file(&table_path).expect("temporary table should be removable");
}

#[test]
fn iterative_first_order_link_prune_threshold_table_rejects_rows_with_extra_fields() {
    let table_path = temp_threshold_table_path("threshold_extra_fields");
    fs::write(&table_path, "code,csa_ha,mscl_m\n1,12.5,100,unexpected\n")
        .expect("table should write");

    let err = parse_threshold_table(&table_path.to_string_lossy())
        .expect_err("rows with extra fields should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err
        .to_string()
        .contains("must contain exactly code,csa_ha,mscl_m"));

    fs::remove_file(&table_path).expect("temporary table should be removable");
}

#[test]
fn iterative_first_order_link_prune_geometry_validation_rejects_extent_mismatch() {
    let reference = synthetic_raster_with_geometry(2, 2, 100.0, 160.0, 40.0, 100.0, 30.0, 30.0);
    let other = synthetic_raster_with_geometry(2, 2, 100.5, 160.5, 40.0, 100.0, 30.0, 30.0);

    let err = validate_matching_geometry(&reference, &other, "--upstream_area")
        .expect_err("extent mismatch should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("Raster geometry mismatch"));
    assert!(err.to_string().contains("west expected"));
}

#[test]
fn iterative_first_order_link_prune_geometry_validation_rejects_resolution_mismatch() {
    let reference = synthetic_raster_with_geometry(2, 2, 100.0, 160.0, 40.0, 100.0, 30.0, 30.0);
    let other = synthetic_raster_with_geometry(2, 2, 100.0, 160.0, 40.0, 100.0, 30.1, 30.0);

    let err = validate_matching_geometry(&reference, &other, "--threshold_code_raster")
        .expect_err("resolution mismatch should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("resolution_x expected"));
}
