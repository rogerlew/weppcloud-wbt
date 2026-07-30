use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

const SIZE: usize = 7;

fn temp_raster_path(stem: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "fill_depressions_edge_outlet_{}_{}_{}.tif",
        stem,
        process::id(),
        nanos
    ))
}

fn cleanup_raster(path: &Path) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(format!("{}.aux.xml", path.to_string_lossy()));
}

fn west_edge_matrix() -> Vec<Vec<f64>> {
    vec![
        vec![20.0, 20.0, 20.0, 20.0, 20.0, 0.0, 20.0],
        vec![20.0, 20.0, 20.0, 20.0, 20.0, 0.0, 20.0],
        vec![5.0, 5.0, 5.0, 5.0, 15.0, 0.0, 20.0],
        vec![5.0, 5.0, 5.0, 5.0, 15.0, 0.0, 20.0],
        vec![5.0, 5.0, 5.0, 5.0, 15.0, 0.0, 20.0],
        vec![20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 20.0],
        vec![20.0, 20.0, 20.0, 20.0, 20.0, 20.0, 20.0],
    ]
}

fn rotate_clockwise(values: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let mut rotated = vec![vec![0.0; SIZE]; SIZE];
    for (row, values_row) in values.iter().enumerate() {
        for (column, value) in values_row.iter().enumerate() {
            rotated[column][SIZE - 1 - row] = *value;
        }
    }
    rotated
}

fn rotate(values: &[Vec<f64>], turns: usize) -> Vec<Vec<f64>> {
    let mut rotated = values.to_vec();
    for _ in 0..turns {
        rotated = rotate_clockwise(&rotated);
    }
    rotated
}

fn write_raster(path: &Path, values: &[Vec<f64>]) {
    let configs = RasterConfigs {
        rows: SIZE,
        columns: SIZE,
        nodata: -32768.0,
        north: SIZE as f64,
        south: 0.0,
        east: SIZE as f64,
        west: 0.0,
        resolution_x: 1.0,
        resolution_y: 1.0,
        data_type: DataType::F64,
        photometric_interp: PhotometricInterpretation::Continuous,
        ..Default::default()
    };
    let mut raster = Raster::initialize_using_config(&path.to_string_lossy(), &configs);
    for (row, values_row) in values.iter().enumerate() {
        raster.set_row_data(row as isize, values_row.clone());
    }
    raster.write().expect("synthetic input raster should write");
}

fn run_fill(
    stem: &str,
    values: &[Vec<f64>],
    fix_flats: bool,
    flat_increment: Option<f64>,
    max_depth: Option<f64>,
) -> Raster {
    let input_path = temp_raster_path(&format!("{}_input", stem));
    let output_path = temp_raster_path(&format!("{}_output", stem));
    write_raster(&input_path, values);

    let mut args = vec![
        format!("--dem={}", input_path.display()),
        format!("--output={}", output_path.display()),
        format!("--fix_flats={}", fix_flats),
    ];
    if let Some(value) = flat_increment {
        args.push(format!("--flat_increment={}", value));
    }
    if let Some(value) = max_depth {
        args.push(format!("--max_depth={}", value));
    }

    FillDepressions::new()
        .run(args, "", false)
        .expect("FillDepressions should run on the synthetic fixture");
    let output =
        Raster::new(&output_path.to_string_lossy(), "r").expect("output raster should open");

    cleanup_raster(&input_path);
    cleanup_raster(&output_path);
    output
}

fn assert_rasters_equal(expected: &[Vec<f64>], actual: &Raster) {
    for (row, expected_row) in expected.iter().enumerate() {
        for (column, expected_value) in expected_row.iter().enumerate() {
            assert_eq!(
                actual[(row as isize, column as isize)],
                *expected_value,
                "unexpected value at ({}, {})",
                row,
                column
            );
        }
    }
}

#[test]
fn fill_depressions_edge_outlet_preserves_all_four_outer_edges() {
    let west = west_edge_matrix();
    for turns in 0..4 {
        let values = rotate(&west, turns);
        let output = run_fill(&format!("rotation_{}", turns), &values, false, None, None);
        assert_rasters_equal(&values, &output);
    }
}

#[test]
fn fill_depressions_edge_outlet_still_fills_enclosed_depression() {
    let mut values = west_edge_matrix();
    for row in 2..=4 {
        values[row][0] = 20.0;
    }

    let output = run_fill("enclosed", &values, false, None, None);
    for row in 2..=4 {
        for column in 1..=3 {
            assert_eq!(
                output[(row as isize, column as isize)],
                15.0,
                "enclosed low region should fill to its saddle"
            );
        }
    }
}

#[test]
fn fill_depressions_edge_outlet_seeds_flat_gradient_at_edge() {
    let values = west_edge_matrix();
    let output = run_fill("flat_gradient", &values, true, Some(0.1), None);

    for row in 2..=4 {
        assert_eq!(
            output[(row as isize, 0)],
            5.0,
            "outer-edge outlet elevation should remain unchanged"
        );
    }
    assert!(
        output[(3, 3)] > output[(3, 0)],
        "flat gradient should rise away from the edge outlet"
    );
}

#[test]
fn fill_depressions_edge_outlet_preserves_max_depth() {
    let mut values = west_edge_matrix();
    for row in 2..=4 {
        values[row][0] = 20.0;
    }

    let output = run_fill("max_depth", &values, false, None, Some(5.0));
    assert_rasters_equal(&values, &output);
}

#[test]
fn fill_depressions_edge_outlet_preserves_established_nodata_boundary_behavior() {
    let nodata = -32768.0;
    let mut values = west_edge_matrix();
    for row in 2..=4 {
        values[row][0] = 20.0;
    }
    values[3][4] = nodata;

    let output = run_fill("interior_nodata", &values, false, None, None);
    for row in 2..=4 {
        for column in 1..=3 {
            assert_eq!(
                output[(row as isize, column as isize)],
                15.0,
                "interior NoData must not become an outlet for the low region"
            );
        }
    }
    assert_eq!(
        output[(3, 4)],
        nodata,
        "interior NoData must remain unchanged"
    );
}
