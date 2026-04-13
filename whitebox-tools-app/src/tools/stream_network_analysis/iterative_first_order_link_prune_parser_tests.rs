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
    std::env::temp_dir().join(format!(
        "ifolp_{}_{}_{}.csv",
        stem,
        process::id(),
        nanos
    ))
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
fn iterative_first_order_link_prune_parser_accepts_space_separated_signed_numeric_values() {
    let mut args = base_args();
    args[3] = "--csa".to_string();
    args.insert(4, "-1.5".to_string());
    args.push("--epsilon".to_string());
    args.push("+0.25".to_string());

    let parsed = parse_arguments(&args, "/tmp/wd/").expect("parse should succeed");
    assert_eq!(parsed.csa, -1.5);
    assert!((parsed.epsilon - 0.25).abs() < f64::EPSILON);
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
fn iterative_first_order_link_prune_prepare_phase_inputs_excludes_zero_pointer_cells() {
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

    let zero_pointer_cells = prepared.pointers.iter().filter(|&&value| value == 0).count();
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
        active_zero_pointer_cells, 0,
        "zero-coded pointer cells must be excluded from active IFOLP domain"
    );
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
    assert!(
        err.to_string()
            .contains("must use finite csa_ha and mscl_m values")
    );

    fs::remove_file(&table_path).expect("temporary table should be removable");
}
