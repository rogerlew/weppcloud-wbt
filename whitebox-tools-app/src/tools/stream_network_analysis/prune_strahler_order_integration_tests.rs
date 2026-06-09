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
        "prune_strahler_order_{}_{}_{}.tif",
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
fn blackwood_algorithm_correctness() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let fixture_root = repo_root.join("test_fixtures/blackwood_60_5");

    let input_path = fixture_root.join("strahler.tif");
    let output_path = temp_output_path("blackwood_algorithm");

    let tool = PruneStrahlerStreamOrder::new();
    let args = vec![
        format!("--streams={}", input_path.display()),
        format!("--output={}", output_path.display()),
    ];
    tool.run(args, "", false).expect("prune_strahler_order should run on blackwood fixture");

    let input = Raster::new(&input_path.to_string_lossy(), "r")
        .expect("blackwood fixture strahler input should open");
    let output = Raster::new(&output_path.to_string_lossy(), "r")
        .expect("prune_strahler_order output should open");

    assert_eq!(
        output.configs.rows,
        input.configs.rows,
        "output rows should match blackwood input"
    );
    assert_eq!(
        output.configs.columns,
        input.configs.columns,
        "output columns should match blackwood input"
    );

    let input_nodata = input.configs.nodata;
    let mut saw_order_gt_one = false;

    for row in 0..input.configs.rows as isize {
        for col in 0..input.configs.columns as isize {
            let input_value = input[(row, col)];
            let output_value = output[(row, col)];
            if input_value == input_nodata {
                assert_eq!(output_value, input_nodata, "nodata cells should remain nodata at ({row}, {col})");
            } else if input_value > 1.0 {
                saw_order_gt_one = true;
                assert_eq!(
                    output_value,
                    input_value - 1.0,
                    "order-shifted streams should decrement by one at ({row}, {col})"
                );
            } else {
                assert_eq!(
                    output_value,
                    input_nodata,
                    "non-stream/first-order cells should become nodata without zero_background at ({row}, {col})"
                );
            }
        }
    }

    assert!(
        saw_order_gt_one,
        "blackwood fixture should contain at least one stream of order >=2"
    );
    cleanup_output_path(&output_path);
}

#[test]
fn minimal_all_order_one_pruned() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let fixture_root = repo_root.join("test_fixtures/minimal_1pixel_stream");

    let input_path = fixture_root.join("strahler.tif");
    let output_path = temp_output_path("minimal_order_one");

    let tool = PruneStrahlerStreamOrder::new();
    let args = vec![
        format!("--streams={}", input_path.display()),
        format!("--output={}", output_path.display()),
    ];
    tool.run(args, "", false).expect("prune_strahler_order should run on minimal fixture");

    assert!(
        output_path.exists(),
        "pruned output should be created for minimal_1pixel_stream"
    );

    let input = Raster::new(&input_path.to_string_lossy(), "r")
        .expect("minimal fixture strahler input should open");
    let output = Raster::new(&output_path.to_string_lossy(), "r")
        .expect("pruned minimal fixture output should open");

    assert_eq!(
        output.configs.rows,
        input.configs.rows,
        "output rows should match minimal fixture input"
    );
    assert_eq!(
        output.configs.columns,
        input.configs.columns,
        "output columns should match minimal fixture input"
    );

    for row in 0..input.configs.rows as isize {
        for col in 0..input.configs.columns as isize {
            assert!(
                output[(row, col)] <= 0.0,
                "all cells should be non-positive after fully pruning order-1 stream input"
            );
        }
    }

    cleanup_output_path(&output_path);
}

#[test]
fn blackwood_zero_background() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let fixture_root = repo_root.join("test_fixtures/blackwood_60_5");

    let input_path = fixture_root.join("strahler.tif");
    let output_path = temp_output_path("blackwood_zero_background");

    let tool = PruneStrahlerStreamOrder::new();
    let args = vec![
        format!("--streams={}", input_path.display()),
        format!("--output={}", output_path.display()),
        "--zero_background".to_string(),
    ];
    tool
        .run(args, "", false)
        .expect("prune_strahler_order should run on blackwood fixture with zero_background");

    let input = Raster::new(&input_path.to_string_lossy(), "r")
        .expect("blackwood fixture strahler input should open");
    let output = Raster::new(&output_path.to_string_lossy(), "r")
        .expect("prune_strahler_order zero_background output should open");

    assert_eq!(
        output.configs.rows,
        input.configs.rows,
        "output rows should match blackwood input"
    );
    assert_eq!(
        output.configs.columns,
        input.configs.columns,
        "output columns should match blackwood input"
    );

    let input_nodata = input.configs.nodata;
    let mut saw_non_nodata_order_one_or_zero = false;

    for row in 0..input.configs.rows as isize {
        for col in 0..input.configs.columns as isize {
            let input_value = input[(row, col)];
            let output_value = output[(row, col)];
            if input_value == input_nodata {
                assert_eq!(output_value, input_nodata, "nodata cells should remain nodata at ({row}, {col})");
            } else if input_value > 1.0 {
                assert_eq!(
                    output_value,
                    input_value - 1.0,
                    "order-shifted streams should decrement by one at ({row}, {col})"
                );
            } else {
                saw_non_nodata_order_one_or_zero = true;
                assert_eq!(
                    output_value,
                    0.0,
                    "non-stream/first-order cells should become zero with zero_background at ({row}, {col})"
                );
            }
        }
    }

    assert!(
        saw_non_nodata_order_one_or_zero,
        "blackwood fixture should include at least one non-nodata input <= 1 for zero-background coverage"
    );
    cleanup_output_path(&output_path);
}

#[test]
fn blackwood_binary_output() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let fixture_root = repo_root.join("test_fixtures/blackwood_60_5");

    let input_path = fixture_root.join("strahler.tif");
    let output_path = temp_output_path("blackwood_binary");

    let tool = PruneStrahlerStreamOrder::new();
    let args = vec![
        format!("--streams={}", input_path.display()),
        format!("--output={}", output_path.display()),
        "--binary_output".to_string(),
    ];
    tool
        .run(args, "", false)
        .expect("prune_strahler_order should run on blackwood fixture with binary_output");

    let input = Raster::new(&input_path.to_string_lossy(), "r")
        .expect("blackwood fixture strahler input should open");
    let output = Raster::new(&output_path.to_string_lossy(), "r")
        .expect("prune_strahler_order binary output should open");

    assert_eq!(
        output.configs.rows,
        input.configs.rows,
        "output rows should match blackwood input"
    );
    assert_eq!(
        output.configs.columns,
        input.configs.columns,
        "output columns should match blackwood input"
    );

    let input_nodata = input.configs.nodata;
    let mut saw_order_gt_one = false;

    for row in 0..input.configs.rows as isize {
        for col in 0..input.configs.columns as isize {
            let input_value = input[(row, col)];
            let output_value = output[(row, col)];
            if input_value == input_nodata {
                assert_eq!(output_value, input_nodata, "nodata cells should remain nodata at ({row}, {col})");
            } else if input_value > 1.0 {
                saw_order_gt_one = true;
                assert_eq!(
                    output_value,
                    1.0,
                    "retained streams should be binary one at ({row}, {col})"
                );
            } else {
                assert_eq!(
                    output_value,
                    input_nodata,
                    "removed/first-order streams should map to nodata without zero_background when binary output is enabled at ({row}, {col})"
                );
            }
        }
    }

    assert!(
        saw_order_gt_one,
        "blackwood fixture should contain at least one stream of order > 1 for binary-output path"
    );
    cleanup_output_path(&output_path);
}
