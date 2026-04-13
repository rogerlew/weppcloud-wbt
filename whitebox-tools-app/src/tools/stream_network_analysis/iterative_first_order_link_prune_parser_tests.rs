use super::*;
use serde_json::Value;

fn base_args() -> Vec<String> {
    vec![
        "--d8_pntr=d8.tif".to_string(),
        "--upstream_area=area.tif".to_string(),
        "--output=streams.tif".to_string(),
        "--csa=10.0".to_string(),
        "--mscl=100.0".to_string(),
    ]
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
    args.push("-0.25".to_string());

    let parsed = parse_arguments(&args, "/tmp/wd/").expect("parse should succeed");
    assert_eq!(parsed.csa, -1.5);
    assert!((parsed.epsilon + 0.25).abs() < f64::EPSILON);
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
fn iterative_first_order_link_prune_run_returns_phase_placeholder_error() {
    let tool = IterativeFirstOrderLinkPrune::new();
    let err = tool
        .run(base_args(), "/tmp/wd/", false)
        .expect_err("placeholder run path should fail");
    assert_eq!(err.kind(), ErrorKind::Unsupported);
    assert!(
        err.to_string()
            .contains("Phase A source-area qualification is not implemented"),
        "unexpected error: {}",
        err
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
