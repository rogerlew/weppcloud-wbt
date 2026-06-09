use super::*;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

const WATERSHED_NODATA: f64 = -32768.0;

fn temp_output_path(stem: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "watershed_integration_{}_{}_{}.tif",
        stem,
        process::id(),
        nanos
    ))
}

fn temp_geojson_path(stem: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "watershed_integration_{}_{}_{}.geojson",
        stem,
        process::id(),
        nanos
    ))
}

fn cleanup_output_path(path: &PathBuf) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(format!("{}.aux.xml", path.to_string_lossy()));
}

fn cleanup_geojson_path(path: &PathBuf) {
    let _ = fs::remove_file(path);
}

fn load_fixture_root(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve")
        .join(format!("test_fixtures/{}", name))
}

fn read_geojson(path: &PathBuf) -> Value {
    let payload = fs::read_to_string(path).expect("GeoJSON fixture should be readable");
    serde_json::from_str(&payload).expect("GeoJSON should parse as valid JSON")
}

fn point_from_feature_collection(path: &PathBuf) -> (f64, f64) {
    let parsed = read_geojson(path);
    let features = parsed
        .get("features")
        .and_then(Value::as_array)
        .expect("GeoJSON should include features array");
    let feature = features.first().expect("GeoJSON should include at least one feature");
    let coords = feature
        .get("geometry")
        .and_then(|geometry| geometry.get("coordinates"))
        .and_then(Value::as_array)
        .expect("Point feature should include coordinates");
    assert_eq!(coords.len(), 2, "Point coordinates should contain exactly two values");
    let x = coords[0].as_f64().expect("X coordinate should be numeric");
    let y = coords[1].as_f64().expect("Y coordinate should be numeric");
    (x, y)
}

fn assert_output_within_bound(bound: &Raster, output: &Raster) {
    let bound_nodata = bound.configs.nodata;
    let output_nodata = output.configs.nodata;
    assert_eq!(
        bound.configs.rows, output.configs.rows,
        "rows should match bound raster"
    );
    assert_eq!(
        bound.configs.columns, output.configs.columns,
        "columns should match bound raster"
    );

    let mut saw_labeled_cell = false;
    let mut saw_outside_bound = false;

    for row in 0..output.configs.rows as isize {
        for col in 0..output.configs.columns as isize {
            let watershed = bound[(row, col)];
            let output_value = output[(row, col)];
            let _in_bound = watershed != bound_nodata && watershed != 0.0;
            let is_labeled = output_value != output_nodata;

            if _in_bound {
                if is_labeled {
                    saw_labeled_cell = true;
                }
            } else {
                saw_outside_bound = true;
                assert_eq!(
                    output_value, WATERSHED_NODATA,
                    "non-watershed cells should become hardcoded watershed nodata at ({}, {})",
                    row,
                    col
                );
            }
        }
    }

    for row in 0..output.configs.rows as isize {
        for col in 0..output.configs.columns as isize {
            let watershed = bound[(row, col)];
            let output_value = output[(row, col)];
            let in_bound = watershed != bound_nodata && watershed != 0.0;
            let is_labeled = output_value != output_nodata;
            if is_labeled {
                assert!(
                    in_bound,
                    "labeled cells should be inside the fixture watershed at ({}, {})",
                    row,
                    col
                );
            }
        }
    }

    assert!(
        saw_labeled_cell,
        "fixture should yield at least one labeled watershed cell"
    );
    assert!(
        saw_outside_bound,
        "fixture should include at least one non-watershed cell"
    );
}

fn assert_matching_labeled_extent(expected: &Raster, actual: &Raster, nodata: f64) {
    assert_eq!(
        expected.configs.rows,
        actual.configs.rows,
        "labeled rasters should have matching row counts"
    );
    assert_eq!(
        expected.configs.columns,
        actual.configs.columns,
        "labeled rasters should have matching column counts"
    );

    for row in 0..expected.configs.rows as isize {
        for col in 0..expected.configs.columns as isize {
            let expected_labeled = expected[(row, col)] != nodata;
            let actual_labeled = actual[(row, col)] != nodata;
            assert_eq!(
                expected_labeled,
                actual_labeled,
                "GeoJSON and raster pour-point modes should delineate identical extent at ({}, {})",
                row,
                col
            );
        }
    }
}

#[test]
fn geojson_delineates_watershed_matching_bound() {
    let fixture_root = load_fixture_root("minimal_1pixel_stream");
    let d8_path = fixture_root.join("flovec.tif");
    let pour_pts_path = fixture_root.join("outlet.geojson");
    let raster_pour_pts_path = fixture_root.join("netw0.tif");
    let bound_path = fixture_root.join("bound.tif");
    let output_path = temp_output_path("geojson_delineates_bound");
    let output_raster_path = temp_output_path("geojson_delineates_bound_raster");

    let tool = Watershed::new();
    let geojson_args = vec![
        format!("--d8_pntr={}", d8_path.display()),
        format!("--pour_pts={}", pour_pts_path.display()),
        format!("--output={}", output_path.display()),
    ];
    tool.run(geojson_args, "", false)
        .expect("watershed should run using GeoJSON pour points");

    let raster_args = vec![
        format!("--d8_pntr={}", d8_path.display()),
        format!("--pour_pts={}", raster_pour_pts_path.display()),
        format!("--output={}", output_raster_path.display()),
    ];
    tool.run(raster_args, "", false)
        .expect("watershed should run using raster pour points");

    let bound = Raster::new(&bound_path.to_string_lossy(), "r")
        .expect("watershed fixture bound raster should open");
    let output = Raster::new(&output_path.to_string_lossy(), "r")
        .expect("watershed output should open");
    let output_raster = Raster::new(&output_raster_path.to_string_lossy(), "r")
        .expect("raster watershed output should open");

    assert_eq!(
        output.configs.rows,
        bound.configs.rows,
        "watershed output rows should match input D8 rows"
    );
    assert_eq!(
        output.configs.columns,
        bound.configs.columns,
        "watershed output columns should match input D8 columns"
    );

    assert_output_within_bound(&bound, &output);
    assert_matching_labeled_extent(&output, &output_raster, WATERSHED_NODATA);

    for row in 0..bound.configs.rows as isize {
        for col in 0..bound.configs.columns as isize {
            let output_value = output[(row, col)];
            if output_value != WATERSHED_NODATA {
                assert_eq!(
                    output_value, 1.0,
                    "single GeoJSON feature should produce FID=1 labels"
                );
            }
        }
    }

    cleanup_output_path(&output_path);
    cleanup_output_path(&output_raster_path);
}

#[test]
fn geojson_raster_parity() {
    let fixture_root = load_fixture_root("minimal_2pixel_stream");
    let d8_path = fixture_root.join("flovec.tif");
    let geojson_path = fixture_root.join("outlet.geojson");
    let raster_path = fixture_root.join("netw0.tif");

    let output_geojson = temp_output_path("geojson_parity_geojson");
    let output_raster = temp_output_path("geojson_parity_raster");

    let tool = Watershed::new();
    let args_geojson = vec![
        format!("--d8_pntr={}", d8_path.display()),
        format!("--pour_pts={}", geojson_path.display()),
        format!("--output={}", output_geojson.display()),
    ];
    tool.run(args_geojson, "", false)
        .expect("watershed should run with GeoJSON pour points");

    let args_raster = vec![
        format!("--d8_pntr={}", d8_path.display()),
        format!("--pour_pts={}", raster_path.display()),
        format!("--output={}", output_raster.display()),
    ];
    tool.run(args_raster, "", false)
        .expect("watershed should run with raster pour points");

    let geojson_output = Raster::new(&output_geojson.to_string_lossy(), "r")
        .expect("GeoJSON watershed output should open");
    let raster_output = Raster::new(&output_raster.to_string_lossy(), "r")
        .expect("raster watershed output should open");

    assert_eq!(
        geojson_output.configs.rows,
        raster_output.configs.rows,
        "parity outputs should have matching rows"
    );
    assert_eq!(
        geojson_output.configs.columns,
        raster_output.configs.columns,
        "parity outputs should have matching columns"
    );

    assert_eq!(geojson_output.configs.nodata, WATERSHED_NODATA);
    assert_eq!(raster_output.configs.nodata, WATERSHED_NODATA);
    let mut matched = true;
    for row in 0..geojson_output.configs.rows as isize {
        for col in 0..geojson_output.configs.columns as isize {
            let a = geojson_output[(row, col)] != WATERSHED_NODATA;
            let b = raster_output[(row, col)] != WATERSHED_NODATA;
            if a != b {
                matched = false;
                break;
            }
        }
    }

    assert!(matched, "geojson and raster pour-point modes should delineate identical cell extents");
    cleanup_output_path(&output_geojson);
    cleanup_output_path(&output_raster);
}

#[test]
fn multipoint_geojson_produces_watershed() {
    let fixture_root = load_fixture_root("minimal_1pixel_stream");
    let d8_path = fixture_root.join("flovec.tif");
    let bound_path = fixture_root.join("bound.tif");
    let point_fixture = fixture_root.join("outlet.geojson");

    let (x, y) = point_from_feature_collection(&point_fixture);
    let temp_geojson = temp_geojson_path("multipoint_geojson");
    let output_path = temp_output_path("multipoint_geojson");

    let multipoint_geojson = json!({
        "type": "FeatureCollection",
        "features": [
            {
                "type": "Feature",
                "geometry": {
                    "type": "MultiPoint",
                    "coordinates": [[x, y]],
                }
            }
        ]
    });
    fs::write(&temp_geojson, serde_json::to_string(&multipoint_geojson).expect("temporary geojson should serialize"))
        .expect("temporary MultiPoint GeoJSON should write");

    let tool = Watershed::new();
    let args = vec![
        format!("--d8_pntr={}", d8_path.display()),
        format!("--pour_pts={}", temp_geojson.display()),
        format!("--output={}", output_path.display()),
    ];
    tool
        .run(args, "", false)
        .expect("watershed should run with synthetic MultiPoint GeoJSON");

    let bound = Raster::new(&bound_path.to_string_lossy(), "r")
        .expect("watershed fixture bound raster should open");
    let output = Raster::new(&output_path.to_string_lossy(), "r")
        .expect("watershed output should open");

    assert_output_within_bound(&bound, &output);
    cleanup_output_path(&output_path);
    cleanup_geojson_path(&temp_geojson);
}

#[test]
fn non_feature_collection_returns_error() {
    let fixture_root = load_fixture_root("minimal_1pixel_stream");
    let d8_path = fixture_root.join("flovec.tif");
    let temp_geojson = temp_geojson_path("invalid_geojson");
    let output_path = temp_output_path("invalid_geojson");

    let point_json = json!({
        "type": "Point",
        "coordinates": [278362.5000017698, 4868384.500118681],
    });
    fs::write(&temp_geojson, serde_json::to_string(&point_json).expect("invalid geojson should serialize"))
        .expect("bare Point GeoJSON should write");

    let tool = Watershed::new();
    let args = vec![
        format!("--d8_pntr={}", d8_path.display()),
        format!("--pour_pts={}", temp_geojson.display()),
        format!("--output={}", output_path.display()),
    ];
    let result = tool.run(args, "", false);
    assert!(
        result.is_err(),
        "non-FeatureCollection GeoJSON should return an error"
    );
    cleanup_output_path(&output_path);
    cleanup_geojson_path(&temp_geojson);
}
