use super::*;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_output_path(stem: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "clip_raster_to_raster_{}_{}_{}.tif",
        stem,
        process::id(),
        nanos
    ))
}

fn cleanup_output_path(path: &PathBuf) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(format!("{}.aux.xml", path.to_string_lossy()));
}

#[test]
fn blackwood_watershed_clip() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let fixture_root = repo_root.join("test_fixtures/blackwood_60_5");

    let input_path = fixture_root.join("relief.tif");
    let mask_path = fixture_root.join("bound.tif");
    let output_path = temp_output_path("blackwood_watershed_clip");

    let tool = ClipRasterToRaster::new();
    let args = vec![
        format!("--input={}", input_path.display()),
        format!("--mask={}", mask_path.display()),
        format!("--output={}", output_path.display()),
    ];
    tool
        .run(args, "", false)
        .expect("clip_raster_to_raster should run on blackwood watershed fixture");

    let input = Raster::new(&input_path.to_string_lossy(), "r")
        .expect("blackwood relief fixture should open");
    let mask = Raster::new(&mask_path.to_string_lossy(), "r")
        .expect("blackwood mask fixture should open");
    let output = Raster::new(&output_path.to_string_lossy(), "r")
        .expect("clip_raster_to_raster output should open");

    assert_eq!(
        output.configs.rows,
        input.configs.rows,
        "output rows should match input rows"
    );
    assert_eq!(
        output.configs.columns,
        input.configs.columns,
        "output columns should match input columns"
    );

    let input_nodata = input.configs.nodata;
    let mask_nodata = mask.configs.nodata;
    let mut saw_pass_through = false;
    let mut saw_exclusion = false;

    for row in 0..input.configs.rows as isize {
        for col in 0..input.configs.columns as isize {
            let input_value = input[(row, col)];
            let mask_value = mask[(row, col)];
            let output_value = output[(row, col)];

            if mask_value != mask_nodata && mask_value != 0.0 {
                saw_pass_through = true;
                assert_eq!(
                    output_value, input_value,
                    "cells with valid non-zero mask should pass through input value at ({row}, {col})"
                );
            } else {
                saw_exclusion = true;
                assert_eq!(
                    output_value, input_nodata,
                    "masked-out cells should become nodata at ({row}, {col})"
                );
            }
        }
    }

    assert!(saw_pass_through, "fixture should include at least one pass-through cell");
    assert!(saw_exclusion, "fixture should include at least one exclusion cell");
    cleanup_output_path(&output_path);
}

#[test]
fn minimal_zero_mask_becomes_nodata() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let fixture_root = repo_root.join("test_fixtures/minimal_2pixel_stream");

    let input_path = fixture_root.join("relief.tif");
    let mask_path = fixture_root.join("netw0.tif");
    let output_path = temp_output_path("minimal_zero_mask");

    let tool = ClipRasterToRaster::new();
    let args = vec![
        format!("--input={}", input_path.display()),
        format!("--mask={}", mask_path.display()),
        format!("--output={}", output_path.display()),
    ];
    tool
        .run(args, "", false)
        .expect("clip_raster_to_raster should run on minimal 2-pixel stream fixture");

    let input = Raster::new(&input_path.to_string_lossy(), "r")
        .expect("minimal input relief should open");
    let mask = Raster::new(&mask_path.to_string_lossy(), "r")
        .expect("minimal mask raster should open");
    let output = Raster::new(&output_path.to_string_lossy(), "r")
        .expect("clip_raster_to_raster output should open");

    let input_nodata = input.configs.nodata;
    let mask_nodata = mask.configs.nodata;
    let mut retained_cells = 0usize;

    for row in 0..input.configs.rows as isize {
        for col in 0..input.configs.columns as isize {
            let input_value = input[(row, col)];
            let mask_value = mask[(row, col)];
            let output_value = output[(row, col)];

            if output_value != input_nodata {
                retained_cells += 1;
                assert!(
                    mask_value != mask_nodata && mask_value != 0.0,
                    "non-nodata output should only occur under valid non-zero mask at ({row}, {col})"
                );
                assert_eq!(
                    output_value, input_value,
                    "pass-through cells should preserve input value at ({row}, {col})"
                );
            } else {
                assert_eq!(
                    output_value, input_nodata,
                    "all non-pass-through cells should be nodata at ({row}, {col})"
                );
            }
        }
    }

    assert_eq!(retained_cells, 2, "minimal stream fixture should retain exactly two cells");
    cleanup_output_path(&output_path);
}

#[test]
fn mismatched_geometry_returns_error() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let fixture_root_input = repo_root.join("test_fixtures/blackwood_60_5");
    let fixture_root_mask = repo_root.join("test_fixtures/minimal_1pixel_stream");

    let input_path = fixture_root_input.join("relief.tif");
    let mask_path = fixture_root_mask.join("bound.tif");
    let output_path = temp_output_path("mismatched_geometry");

    let tool = ClipRasterToRaster::new();
    let args = vec![
        format!("--input={}", input_path.display()),
        format!("--mask={}", mask_path.display()),
        format!("--output={}", output_path.display()),
    ];

    let result = tool.run(args, "", false);
    assert!(
        result.is_err(),
        "clip_raster_to_raster should error when input and mask geometry differ"
    );
    cleanup_output_path(&output_path);
}
