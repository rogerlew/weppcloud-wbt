use super::*;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

const FVSLOPE_TOLERANCE: f64 = 1e-4;

fn temp_output_path(stem: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("fvslope_{}_{}_{}.tif", stem, process::id(), nanos))
}

fn cleanup_output_path(path: &PathBuf) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(format!("{}.aux.xml", path.to_string_lossy()));
}

fn flow_downstream(row: isize, col: isize, dir: usize, use_esri_style: bool) -> Option<(isize, isize)> {
    let dx = [1isize, 1, 1, 0, -1, -1, -1, 0];
    let dy = [-1isize, 0, 1, 1, 1, 0, -1, -1];
    let mut pntr_matches: [usize; 129] = [999usize; 129];
    if !use_esri_style {
        pntr_matches[1] = 0usize;
        pntr_matches[2] = 1usize;
        pntr_matches[4] = 2usize;
        pntr_matches[8] = 3usize;
        pntr_matches[16] = 4usize;
        pntr_matches[32] = 5usize;
        pntr_matches[64] = 6usize;
        pntr_matches[128] = 7usize;
    } else {
        pntr_matches[1] = 1usize;
        pntr_matches[2] = 2usize;
        pntr_matches[4] = 3usize;
        pntr_matches[8] = 4usize;
        pntr_matches[16] = 5usize;
        pntr_matches[32] = 6usize;
        pntr_matches[64] = 7usize;
        pntr_matches[128] = 0usize;
    }
    if dir >= pntr_matches.len() || pntr_matches[dir] == 999 {
        return None;
    }
    let c = pntr_matches[dir];
    Some((row + dy[c], col + dx[c]))
}

fn expected_ratio_from_minimal_inputs(dem: &Raster, d8: &Raster) -> ((isize, isize), f64) {
    let rows = dem.configs.rows as isize;
    let cols = dem.configs.columns as isize;
    let dem_nodata = dem.configs.nodata;
    let d8_nodata = d8.configs.nodata;
    let cell_size_x = dem.configs.resolution_x.abs();
    let cell_size_y = dem.configs.resolution_y.abs();
    let diag_cell_size = (cell_size_x * cell_size_x + cell_size_y * cell_size_y).sqrt();
    let mut best = None;

    for row in 0..rows {
        for col in 0..cols {
            let z_here = dem[(row, col)];
            if z_here == dem_nodata {
                continue;
            }
            let dir_val = d8[(row, col)];
            if dir_val == d8_nodata || dir_val <= 0.0 {
                continue;
            }
            let dir = dir_val as usize;
            let Some((n_row, n_col)) = flow_downstream(row, col, dir, false) else {
                continue;
            };
            if n_row < 0 || n_row >= rows || n_col < 0 || n_col >= cols {
                continue;
            }
            let z_down = dem[(n_row, n_col)];
            if z_down == dem_nodata || z_here <= z_down {
                continue;
            }

            let dx = (n_col - col).abs();
            let dy = (n_row - row).abs();
            let distance = if dx == 0 || dy == 0 {
                if dx == 0 { cell_size_y } else { cell_size_x }
            } else {
                diag_cell_size
            };

            if distance > 0.0 {
                let ratio = (z_here - z_down) / distance;
                if best
                    .as_ref()
                    .is_none_or(|&(_, best_ratio)| ratio > best_ratio)
                {
                    best = Some(((row, col), ratio));
                }
            }
        }
    }

    best.expect("minimal fixture should include a positive along-flow elevation drop")
}

fn validate_unit_mask_and_shape(reference: &Raster, candidate: &Raster) {
    assert_eq!(
        reference.configs.rows,
        candidate.configs.rows,
        "unit output rows should match ratio"
    );
    assert_eq!(
        reference.configs.columns,
        candidate.configs.columns,
        "unit output columns should match ratio"
    );
}

#[test]
fn blackwood_regression() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let fixture_root = repo_root.join("test_fixtures/blackwood_60_5");

    let dem_path = fixture_root.join("relief.tif");
    let d8_path = fixture_root.join("flovec.tif");
    let reference_path = fixture_root.join("fvslop.tif");
    let output_path = temp_output_path("blackwood_ratio");

    let tool = FVSlope::new();
    let args = vec![
        format!("--dem={}", dem_path.display()),
        format!("--d8_pntr={}", d8_path.display()),
        format!("--output={}", output_path.display()),
        "--units=ratio".to_string(),
    ];
    tool.run(args, "", false).expect("fvslope should run for blackwood fixture");

    assert!(output_path.exists(), "integration output raster should exist");

    let output = Raster::new(&output_path.to_string_lossy(), "r")
        .expect("output fvslope raster should open");
    let reference = Raster::new(&reference_path.to_string_lossy(), "r")
        .expect("reference fvslope raster should open");

    assert_eq!(
        output.configs.rows,
        reference.configs.rows,
        "output rows should match reference"
    );
    assert_eq!(
        output.configs.columns,
        reference.configs.columns,
        "output columns should match reference"
    );

    let output_nodata = output.configs.nodata;
    let reference_nodata = reference.configs.nodata;
    let mut valid_count = 0usize;
    let mut max_diff = 0.0f64;
    for row in 0..output.configs.rows as isize {
        for col in 0..output.configs.columns as isize {
            let reference_value = reference[(row, col)];
            let output_value = output[(row, col)];
            if reference_value == reference_nodata {
                continue;
            }
            valid_count += 1;
            let diff = (reference_value - output_value).abs();
            if diff > max_diff {
                max_diff = diff;
            }
            assert!(
                diff < 1e-5,
                "fvslope blackwood regression should match reference (diff {}) at ({}, {})",
                diff,
                row,
                col
            );
            assert_ne!(
                output_value, output_nodata,
                "fvslope blackwood regression should not replace valid reference nodata"
            );
        }
    }
    assert_eq!(
        reference.configs.nodata, output_nodata,
        "fvslope blackwood regression should preserve reference nodata value"
    );
    assert!(
        valid_count > 0,
        "blackwood fixture must include at least one valid regression pixel"
    );
    assert!(
        max_diff < 1e-5,
        "fvslope blackwood regression should stay within float tolerance"
    );

    cleanup_output_path(&output_path);
}

#[test]
fn blackwood_units_consistency() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let fixture_root = repo_root.join("test_fixtures/blackwood_60_5");

    let dem_path = fixture_root.join("relief.tif");
    let d8_path = fixture_root.join("flovec.tif");
    let (ratio_path, degrees_path, radians_path, percent_path) = (
        temp_output_path("blackwood_ratio_units"),
        temp_output_path("blackwood_degrees_units"),
        temp_output_path("blackwood_radians_units"),
        temp_output_path("blackwood_percent_units"),
    );

    let tool = FVSlope::new();
    let runs = vec![
        (ratio_path.clone(), "ratio"),
        (degrees_path.clone(), "degrees"),
        (radians_path.clone(), "radians"),
        (percent_path.clone(), "percent"),
    ];
    for (output_path, units) in runs.iter() {
        let args = vec![
            format!("--dem={}", dem_path.display()),
            format!("--d8_pntr={}", d8_path.display()),
            format!("--output={}", output_path.display()),
            format!("--units={}", units),
        ];
        tool.run(args, "", false)
            .expect("fvslope should run for all requested unit outputs");
        assert!(
            output_path.exists(),
            "fvslope unit output raster should exist"
        );
    }

    let ratio_raster = Raster::new(&ratio_path.to_string_lossy(), "r")
        .expect("ratio fvslope raster should open");
    let degrees_raster = Raster::new(&degrees_path.to_string_lossy(), "r")
        .expect("degrees fvslope raster should open");
    let radians_raster = Raster::new(&radians_path.to_string_lossy(), "r")
        .expect("radians fvslope raster should open");
    let percent_raster = Raster::new(&percent_path.to_string_lossy(), "r")
        .expect("percent fvslope raster should open");

    validate_unit_mask_and_shape(&ratio_raster, &degrees_raster);
    validate_unit_mask_and_shape(&ratio_raster, &radians_raster);
    validate_unit_mask_and_shape(&ratio_raster, &percent_raster);

    let ratio_nodata = ratio_raster.configs.nodata;
    for row in 0..ratio_raster.configs.rows as isize {
        for col in 0..ratio_raster.configs.columns as isize {
            let ratio = ratio_raster[(row, col)];
            let degrees = degrees_raster[(row, col)];
            let radians = radians_raster[(row, col)];
            let percent = percent_raster[(row, col)];

            if ratio == ratio_nodata {
                assert_eq!(
                    ratio,
                    degrees,
                    "degrees output should preserve ratio nodata mask"
                );
                assert_eq!(
                    ratio,
                    radians,
                    "radians output should preserve ratio nodata mask"
                );
                assert_eq!(
                    ratio,
                    percent,
                    "percent output should preserve ratio nodata mask"
                );
            } else {
                assert!(degrees.is_finite());
                assert!(radians.is_finite());
                assert!(percent.is_finite());
                assert!(
                    (degrees - ratio.atan().to_degrees()).abs() < FVSLOPE_TOLERANCE,
                    "degrees output should equal atan(ratio) for valid pixels"
                );
                assert!(
                    (radians - ratio.atan()).abs() < FVSLOPE_TOLERANCE,
                    "radians output should equal atan(ratio) for valid pixels"
                );
                assert!(
                    (percent - ratio * 100f64).abs() < FVSLOPE_TOLERANCE,
                    "percent output should equal ratio * 100 for valid pixels"
                );
                assert_eq!(
                    ratio >= 0.0,
                    true,
                    "ratio should remain non-negative for valid pixels"
                );
            }
        }
    }

    cleanup_output_path(&ratio_path);
    cleanup_output_path(&degrees_path);
    cleanup_output_path(&radians_path);
    cleanup_output_path(&percent_path);
}

#[test]
fn minimal_structural() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let fixture_root = repo_root.join("test_fixtures/minimal_2pixel_stream");

    let dem_path = fixture_root.join("relief.tif");
    let d8_path = fixture_root.join("flovec.tif");
    let output_path = temp_output_path("minimal_ratio");
    let dem = Raster::new(&dem_path.to_string_lossy(), "r").expect("minimal DEM should open");
    let d8 = Raster::new(&d8_path.to_string_lossy(), "r").expect("minimal pointer raster should open");
    let ((upstream_row, upstream_col), expected_ratio) = expected_ratio_from_minimal_inputs(&dem, &d8);

    let tool = FVSlope::new();
    let args = vec![
        format!("--dem={}", dem_path.display()),
        format!("--d8_pntr={}", d8_path.display()),
        format!("--output={}", output_path.display()),
        "--units=ratio".to_string(),
    ];
    tool
        .run(args, "", false)
        .expect("fvslope should run for minimal structural fixture");

    assert!(output_path.exists(), "fvslope output should be created");
    let output = Raster::new(&output_path.to_string_lossy(), "r")
        .expect("minimal fvslope raster should open");

    assert_eq!(output.configs.rows, dem.configs.rows, "output rows should match DEM");
    assert_eq!(
        output.configs.columns,
        dem.configs.columns,
        "output columns should match DEM"
    );

    let output_nodata = output.configs.nodata;
    let mut valid_count = 0usize;
    let mut positive_count = 0usize;
    for row in 0..output.configs.rows as isize {
        for col in 0..output.configs.columns as isize {
            let value = output[(row, col)];
            if value == output_nodata {
                continue;
            }
            valid_count += 1;
            assert!(
                value >= 0.0,
                "FVSlope outputs should not include negative slopes"
            );
            if value > 0.0 {
                positive_count += 1;
            }
        }
    }
    assert!(
        valid_count > 0,
        "minimal fixture should include at least one valid output value"
    );
    assert!(
        positive_count > 0,
        "minimal fixture should include at least one positive slope"
    );

    let upstream_value = output[(upstream_row, upstream_col)];
    assert_ne!(
        upstream_value, output_nodata,
        "upstream cell should contain a valid slope"
    );
    let diff = (upstream_value - expected_ratio).abs();
    assert!(
        diff <= FVSLOPE_TOLERANCE * 10f64,
        "upstream minimal slope should match derived expectation (diff {})",
        diff
    );

    cleanup_output_path(&output_path);
}
