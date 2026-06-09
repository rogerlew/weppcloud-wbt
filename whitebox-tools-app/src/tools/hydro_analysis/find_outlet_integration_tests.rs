use super::*;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_output_path(stem: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("find_outlet_{}_{}_{}.geojson", stem, process::id(), nanos))
}

fn cleanup_output_path(path: &PathBuf) {
    let _ = fs::remove_file(path);
}

fn read_feature_collection(path: &PathBuf) -> Value {
    let payload = fs::read_to_string(path).expect("outlet GeoJSON should be readable");
    serde_json::from_str(&payload).expect("outlet GeoJSON should parse as JSON")
}

fn feature_coordinates(feature: &Value) -> (f64, f64) {
    let coords = feature
        .get("geometry")
        .and_then(|geometry| geometry.get("coordinates"))
        .and_then(|coordinates| coordinates.as_array())
        .expect("feature geometry coordinates should be an array");
    assert_eq!(coords.len(), 2, "geometry should contain a single X/Y coordinate pair");
    let x = coords[0].as_f64().expect("easting should be numeric");
    let y = coords[1].as_f64().expect("northing should be numeric");
    (x, y)
}

#[test]
fn find_outlet_integration_from_requested_row_col_uses_requested_start() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let fixture_root = repo_root.join("test_fixtures/blackwood_60_5");

    let d8_path = fixture_root.join("flovec.tif");
    let streams_path = fixture_root.join("netw0.tif");
    let output_path = temp_output_path("requested_row_col");

    let tool = FindOutlet::new();
    let args = vec![
        format!("--d8_pntr={}", d8_path.display()),
        format!("--streams={}", streams_path.display()),
        "--requested_outlet_row_col=142,353".to_string(),
        format!("--output={}", output_path.display()),
    ];

    tool.run(args, "", false)
        .expect("find_outlet should execute from requested start cell");

    let parsed = read_feature_collection(&output_path);
    let features = parsed
        .get("features")
        .and_then(|value| value.as_array())
        .expect("feature collection should include an array of features");
    assert_eq!(features.len(), 1, "requested outlet execution should emit one feature");

    let feature = &features[0];
    let geometry_type = feature
        .get("geometry")
        .and_then(|value| value.get("type"))
        .and_then(Value::as_str)
        .expect("geometry type should be present");
    assert_eq!(geometry_type, "Point", "outlet should emit a point");

    let (easting, northing) = feature_coordinates(feature);
    assert!((easting - 745374.4161227769).abs() < 1e-6, "easting should match fixture outlet");
    assert!(
        (northing - 4332519.648953125).abs() < 1e-6,
        "northing should match fixture outlet"
    );

    let properties = feature
        .get("properties")
        .and_then(Value::as_object)
        .expect("feature properties should be present");
    assert_eq!(properties.get("start_mode").and_then(Value::as_str), Some("requested"));
    assert_eq!(properties.get("row").and_then(Value::as_i64), Some(142));
    assert_eq!(properties.get("column").and_then(Value::as_i64), Some(353));
    assert_eq!(
        properties.get("requested_row").and_then(Value::as_i64),
        Some(142)
    );
    assert_eq!(
        properties.get("requested_col").and_then(Value::as_i64),
        Some(353)
    );
    assert_eq!(
        properties
            .get("candidates_considered")
            .and_then(Value::as_u64),
        Some(0)
    );
    assert!(
        properties
            .get("steps_from_start")
            .and_then(Value::as_u64)
            .is_some(),
        "requested start walk should include a step count"
    );

    cleanup_output_path(&output_path);
}

#[test]
fn find_outlet_integration_from_watershed_mask_prefers_mask_candidate() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let fixture_root = repo_root.join("test_fixtures/blackwood_60_5");

    let d8_path = fixture_root.join("flovec.tif");
    let streams_path = fixture_root.join("netw0.tif");
    let watershed_path = fixture_root.join("bound.tif");
    let output_path = temp_output_path("watershed_mode");

    let tool = FindOutlet::new();
    let args = vec![
        format!("--d8_pntr={}", d8_path.display()),
        format!("--streams={}", streams_path.display()),
        format!("--watershed={}", watershed_path.display()),
        format!("--output={}", output_path.display()),
    ];

    tool.run(args, "", false)
        .expect("find_outlet should execute using the watershed mask mode");

    let parsed = read_feature_collection(&output_path);
    let features = parsed
        .get("features")
        .and_then(|value| value.as_array())
        .expect("feature collection should include an array of features");
    assert_eq!(features.len(), 1, "watershed-based execution should emit one feature");

    let feature = &features[0];
    let (easting, northing) = feature_coordinates(feature);
    let properties = feature
        .get("properties")
        .and_then(Value::as_object)
        .expect("feature properties should be present");

    assert_eq!(properties.get("start_mode").and_then(Value::as_str), Some("watershed"));
    assert_eq!(
        properties.get("start_in_mask").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        properties
            .get("outlet_in_mask")
            .and_then(Value::as_bool),
        Some(true)
    );
    let candidates = properties
        .get("candidates_considered")
        .and_then(Value::as_u64)
        .expect("candidates_considered should be numeric");
    assert!(candidates > 0, "watershed mode should consider candidates");
    assert_eq!(
        properties
            .get("candidate_rank")
            .and_then(Value::as_u64)
            .is_some(),
        true,
        "watershed mode should include a numeric candidate rank"
    );
    assert_eq!(
        properties.get("outlet_junction_count").and_then(Value::as_i64),
        Some(1)
    );
    assert!(
        properties
            .get("steps_from_start")
            .and_then(Value::as_u64)
            .unwrap()
            > 0,
        "watershed mode should trace at least one step"
    );

    let crs_name = parsed
        .get("crs")
        .and_then(|value| value.get("properties"))
        .and_then(|value| value.get("name"))
        .and_then(Value::as_str)
        .expect("GeoJSON should include EPSG metadata");
    assert_eq!(
        crs_name,
        "urn:ogc:def:crs:EPSG::32610",
        "fixture CRS should be EPSG:32610"
    );

    assert!(
        easting.is_finite() && northing.is_finite(),
        "output coordinates should be finite"
    );

    cleanup_output_path(&output_path);
}
