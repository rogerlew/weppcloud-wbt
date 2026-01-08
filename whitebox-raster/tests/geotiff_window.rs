use std::path::{Path, PathBuf};

use whitebox_raster::geotiff::read_geotiff_window;
use whitebox_raster::{DataType, Raster, RasterConfigs};

fn data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("vrt_test_data")
}

fn data_path(rel: &str) -> PathBuf {
    data_root().join(rel)
}

fn diff_paths(path: &Path, base: &Path) -> Option<PathBuf> {
    let path_components: Vec<_> = path.components().collect();
    let base_components: Vec<_> = base.components().collect();

    let mut i = 0;
    while i < path_components.len()
        && i < base_components.len()
        && path_components[i] == base_components[i]
    {
        i += 1;
    }

    if i == 0 {
        return None;
    }

    let mut result = PathBuf::new();
    for _ in i..base_components.len() {
        result.push("..");
    }
    for comp in &path_components[i..] {
        result.push(comp.as_os_str());
    }
    Some(result)
}

fn read_window(
    source: &Path,
    x_off: isize,
    y_off: isize,
    x_size: isize,
    y_size: isize,
) -> (RasterConfigs, Vec<f64>) {
    let mut configs = RasterConfigs::default();
    let mut data = Vec::new();
    read_geotiff_window(
        &source.to_string_lossy().to_string(),
        x_off,
        y_off,
        x_size,
        y_size,
        &mut configs,
        &mut data,
    )
    .expect("Expected windowed read to succeed.");
    (configs, data)
}

fn assert_window_matches_reference(
    window_configs: &RasterConfigs,
    window_data: &[f64],
    reference: &Raster,
) {
    assert_eq!(window_configs.rows, reference.configs.rows);
    assert_eq!(window_configs.columns, reference.configs.columns);
    assert_eq!(window_configs.data_type, reference.configs.data_type);
    assert_eq!(window_configs.photometric_interp, reference.configs.photometric_interp);
    assert_eq!(window_configs.epsg_code, reference.configs.epsg_code);

    let geo_tol = 1e-6;
    assert!((window_configs.west - reference.configs.west).abs() <= geo_tol);
    assert!((window_configs.east - reference.configs.east).abs() <= geo_tol);
    assert!((window_configs.north - reference.configs.north).abs() <= geo_tol);
    assert!((window_configs.south - reference.configs.south).abs() <= geo_tol);
    assert!((window_configs.resolution_x - reference.configs.resolution_x).abs() <= geo_tol);
    assert!((window_configs.resolution_y - reference.configs.resolution_y).abs() <= geo_tol);

    let tol = match reference.configs.data_type {
        DataType::F32 => 1e-6,
        DataType::F64 => 1e-9,
        _ => 0.0,
    };

    for row in 0..window_configs.rows {
        for col in 0..window_configs.columns {
            let idx = row * window_configs.columns + col;
            let expected = reference[(row as isize, col as isize)];
            let actual = window_data[idx];
            if reference.configs.data_type.is_float() {
                assert!(
                    (actual - expected).abs() <= tol,
                    "Mismatch at row {} col {}: {} vs {}",
                    row,
                    col,
                    actual,
                    expected
                );
            } else {
                assert!(
                    actual == expected,
                    "Mismatch at row {} col {}: {} vs {}",
                    row,
                    col,
                    actual,
                    expected
                );
            }
        }
    }
}

fn run_window_test(
    source_rel: &str,
    reference_rel: &str,
    x_off: isize,
    y_off: isize,
    x_size: isize,
    y_size: isize,
) {
    let source_path = data_path(source_rel);
    let reference_path = data_path(reference_rel);
    let (configs, data) = read_window(&source_path, x_off, y_off, x_size, y_size);
    let reference = Raster::new(reference_path.to_str().unwrap(), "r")
        .expect("Expected reference raster to load.");
    assert_window_matches_reference(&configs, &data, &reference);
}

#[test]
fn test_window_stripped_center() {
    run_window_test(
        "source/dem_500x500.tif",
        "reference/crop_center_100x100.tif",
        200,
        200,
        100,
        100,
    );
}

#[test]
fn test_window_lzw() {
    run_window_test(
        "source/dem_500x500_lzw.tif",
        "reference/crop_lzw_200x200.tif",
        100,
        100,
        200,
        200,
    );
}

#[test]
fn test_window_deflate() {
    run_window_test(
        "source/dem_500x500_deflate.tif",
        "reference/crop_deflate_300x300.tif",
        50,
        50,
        300,
        300,
    );
}

#[test]
fn test_window_tiled_nonaligned() {
    run_window_test(
        "source/dem_500x500_tiled.tif",
        "reference/crop_tiled_nonaligned.tif",
        37,
        41,
        67,
        83,
    );
}

#[test]
fn test_window_tiled_lzw() {
    run_window_test(
        "source/dem_500x500_tiled_lzw.tif",
        "reference/crop_tiled_lzw_150x150.tif",
        50,
        50,
        150,
        150,
    );
}

#[test]
fn test_window_int16() {
    run_window_test(
        "source/dem_100x100_int16.tif",
        "reference/crop_int16_50x50.tif",
        25,
        25,
        50,
        50,
    );
}

#[test]
fn test_window_float32() {
    run_window_test(
        "source/dem_100x100_float32.tif",
        "reference/crop_float32_50x50.tif",
        25,
        25,
        50,
        50,
    );
}

#[test]
fn test_window_int16_lzw_pred2() {
    run_window_test(
        "source/dem_100x100_int16_lzw_pred2.tif",
        "reference/crop_int16_pred2_50x50.tif",
        25,
        25,
        50,
        50,
    );
}

#[test]
fn test_window_int16_packbits() {
    run_window_test(
        "source/dem_100x100_int16_packbits.tif",
        "reference/crop_int16_packbits_50x50.tif",
        25,
        25,
        50,
        50,
    );
}

#[test]
fn test_window_sparse_tiles() {
    run_window_test(
        "source/dem_500x500_sparse.tif",
        "reference/crop_sparse_120x120.tif",
        0,
        0,
        120,
        120,
    );
}

#[test]
fn test_window_relative_path() {
    let source_abs = data_path("source/dem_500x500.tif");
    let cwd = std::env::current_dir().expect("Expected current directory.");
    let source_rel = diff_paths(&source_abs, &cwd).expect("Expected relative path.");
    assert!(!source_rel.is_absolute());

    let (configs, data) = read_window(&source_rel, 200, 200, 100, 100);
    let reference_path = data_path("reference/crop_center_100x100.tif");
    let reference = Raster::new(reference_path.to_str().unwrap(), "r")
        .expect("Expected reference raster to load.");
    assert_window_matches_reference(&configs, &data, &reference);
}

fn assert_invalid_window(source_rel: &str, x_off: isize, y_off: isize, x_size: isize, y_size: isize) {
    let source_path = data_path(source_rel);
    let mut configs = RasterConfigs::default();
    let mut data = Vec::new();
    let result = read_geotiff_window(
        &source_path.to_string_lossy().to_string(),
        x_off,
        y_off,
        x_size,
        y_size,
        &mut configs,
        &mut data,
    );
    assert!(result.is_err());
}

#[test]
fn test_window_negative_offset() {
    assert_invalid_window("source/dem_500x500.tif", -1, 0, 10, 10);
}

#[test]
fn test_window_zero_size() {
    assert_invalid_window("source/dem_500x500.tif", 0, 0, 0, 10);
}

#[test]
fn test_window_out_of_bounds() {
    assert_invalid_window("source/dem_500x500.tif", 450, 0, 100, 10);
}
