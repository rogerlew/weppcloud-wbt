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
        "raise_roads_integration_{}_{}_{}.tif",
        stem,
        process::id(),
        nanos
    ))
}

fn cleanup_output_path(path: &PathBuf) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(format!("{}.aux.xml", path.to_string_lossy()));
}

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve")
        .join("test_fixtures/raise_roads_exogamous_shavenlane")
}

fn load_inputs() -> (PathBuf, PathBuf, Raster) {
    let fixture_path = fixture_root();
    let dem_path = fixture_path.join("dem_clip.tif");
    let roads_path = fixture_path.join("roads.geojson");
    let dem = Raster::new(&dem_path.to_string_lossy(), "r")
        .expect("fixture dem should open");
    (dem_path, roads_path, dem)
}

fn run_raise_roads(output_stem: &str, args: &[String]) -> PathBuf {
    let output = temp_output_path(output_stem);
    let tool = RaiseRoads::new();
    let mut run_args = vec![
        format!("--output={}", output.to_string_lossy()),
    ];
    run_args.extend_from_slice(args);
    tool.run(run_args, "", false)
        .expect("raise_roads integration should run");
    output
}

fn assert_no_lowering_and_no_mask_drift(input: &Raster, output: &Raster) -> usize {
    let mut raised_count = 0usize;
    let input_nodata = input.configs.nodata;
    let output_nodata = output.configs.nodata;

    assert_eq!(
        output.configs.rows,
        input.configs.rows,
        "output rows should match input"
    );
    assert_eq!(
        output.configs.columns,
        input.configs.columns,
        "output columns should match input"
    );

    for row in 0..input.configs.rows as isize {
        for col in 0..input.configs.columns as isize {
            let input_value = input[(row, col)];
            let output_value = output[(row, col)];

            if input_value == input_nodata {
                assert_eq!(
                    output_value, input_nodata,
                    "nodata cells should remain nodata at ({}, {})",
                    row,
                    col
                );
                continue;
            }

            assert!(
                output_value != output_nodata,
                "valid input cells should not become nodata at ({}, {})",
                row,
                col
            );
            assert!(
                output_value >= input_value,
                "road conditioning should never lower terrain at ({}, {})",
                row,
                col
            );
            if output_value > input_value {
                raised_count += 1;
            }
        }
    }

    raised_count
}

fn any_valid_difference(a: &Raster, b: &Raster) -> bool {
    assert_eq!(
        a.configs.rows,
        b.configs.rows,
        "rasters should have identical row counts before comparison"
    );
    assert_eq!(
        a.configs.columns,
        b.configs.columns,
        "rasters should have identical column counts before comparison"
    );

    let a_nodata = a.configs.nodata;
    let b_nodata = b.configs.nodata;
    for row in 0..a.configs.rows as isize {
        for col in 0..a.configs.columns as isize {
            let a_value = a[(row, col)];
            let b_value = b[(row, col)];
            if a_value == a_nodata || b_value == b_nodata {
                continue;
            }
            if (a_value - b_value).abs() > 0.0 {
                return true;
            }
        }
    }

    false
}

#[test]
fn profile_relative_raises_without_lowering() {
    let (dem_path, roads_path, dem) = load_inputs();

    let args = vec![
        format!("--dem={}", dem_path.to_string_lossy()),
        format!(
            "--roads={}",
            roads_path.to_string_lossy()
        ),
        "--strategy=profile_relative".to_string(),
    ];
    let output_path = run_raise_roads("profile_relative", &args);
    assert!(
        output_path.exists(),
        "integration output should be written for profile_relative"
    );

    let output = Raster::new(&output_path.to_string_lossy(), "r")
        .expect("profile_relative output should open");

    assert_eq!(
        output.configs.rows,
        dem.configs.rows,
        "profile_relative output rows should match input"
    );
    assert_eq!(
        output.configs.columns,
        dem.configs.columns,
        "profile_relative output columns should match input"
    );

    let raised_count = assert_no_lowering_and_no_mask_drift(&dem, &output);
    assert!(
        raised_count > 0,
        "profile_relative should modify at least one cell after reprojection"
    );

    cleanup_output_path(&output_path);
}

#[test]
fn constant_and_cross_section_no_lowering() {
    let (dem_path, roads_path, dem) = load_inputs();

    let constant_output = temp_output_path("constant");
    let cross_output = temp_output_path("cross_section");

    let tool = RaiseRoads::new();

    let constant_args = vec![
        format!("--dem={}", dem_path.to_string_lossy()),
        format!(
            "--roads={}",
            roads_path.to_string_lossy()
        ),
        "--strategy=constant".to_string(),
        "--height=3.0".to_string(),
        format!("--output={}", constant_output.to_string_lossy()),
    ];
    tool.run(constant_args, "", false)
        .expect("constant raise_roads integration should run");

    let cross_args = vec![
        format!("--dem={}", dem_path.to_string_lossy()),
        format!(
            "--roads={}",
            roads_path.to_string_lossy()
        ),
        "--strategy=cross_section".to_string(),
        format!("--output={}", cross_output.to_string_lossy()),
    ];
    tool.run(cross_args, "", false)
        .expect("cross_section raise_roads integration should run");

    assert!(
        constant_output.exists(),
        "constant output should be written"
    );
    assert!(
        cross_output.exists(),
        "cross_section output should be written"
    );

    let constant_raster = Raster::new(&constant_output.to_string_lossy(), "r")
        .expect("constant output raster should open");
    let cross_raster = Raster::new(&cross_output.to_string_lossy(), "r")
        .expect("cross_section output raster should open");

    let constant_raised = assert_no_lowering_and_no_mask_drift(&dem, &constant_raster);
    let cross_raised = assert_no_lowering_and_no_mask_drift(&dem, &cross_raster);
    assert!(
        constant_raised > 0,
        "constant strategy should raise at least one cell"
    );
    assert!(
        cross_raised > 0,
        "cross_section strategy should raise at least one cell"
    );

    cleanup_output_path(&constant_output);
    cleanup_output_path(&cross_output);
}

#[test]
fn strategies_produce_distinct_outputs() {
    let (dem_path, roads_path, dem) = load_inputs();
    let tool = RaiseRoads::new();

    let profile_output = temp_output_path("profile_relative_distinct");
    let constant_output = temp_output_path("constant_distinct");
    let cross_output = temp_output_path("cross_section_distinct");

    let profile_args = vec![
        format!("--dem={}", dem_path.to_string_lossy()),
        format!(
            "--roads={}",
            roads_path.to_string_lossy()
        ),
        "--strategy=profile_relative".to_string(),
        format!("--output={}", profile_output.to_string_lossy()),
    ];
    tool.run(profile_args, "", false)
        .expect("profile_relative raise_roads integration should run");
    let constant_args = vec![
        format!("--dem={}", dem_path.to_string_lossy()),
        format!(
            "--roads={}",
            roads_path.to_string_lossy()
        ),
        "--strategy=constant".to_string(),
        "--height=3.0".to_string(),
        format!("--output={}", constant_output.to_string_lossy()),
    ];
    tool.run(constant_args, "", false)
        .expect("constant raise_roads integration should run");
    let cross_args = vec![
        format!("--dem={}", dem_path.to_string_lossy()),
        format!(
            "--roads={}",
            roads_path.to_string_lossy()
        ),
        "--strategy=cross_section".to_string(),
        format!("--output={}", cross_output.to_string_lossy()),
    ];
    tool.run(cross_args, "", false)
        .expect("cross_section raise_roads integration should run");

    let profile = Raster::new(&profile_output.to_string_lossy(), "r")
        .expect("profile_relative output raster should open");
    let constant = Raster::new(&constant_output.to_string_lossy(), "r")
        .expect("constant output raster should open");
    let cross = Raster::new(&cross_output.to_string_lossy(), "r")
        .expect("cross_section output raster should open");

    assert_eq!(profile.configs.nodata, dem.configs.nodata);
    assert_no_lowering_and_no_mask_drift(&dem, &profile);
    assert_no_lowering_and_no_mask_drift(&dem, &constant);
    assert_no_lowering_and_no_mask_drift(&dem, &cross);

    assert!(
        any_valid_difference(&profile, &constant),
        "profile_relative and constant outputs should differ"
    );
    assert!(
        any_valid_difference(&profile, &cross),
        "profile_relative and cross_section outputs should differ"
    );
    assert!(
        any_valid_difference(&constant, &cross),
        "constant and cross_section outputs should differ"
    );

    cleanup_output_path(&profile_output);
    cleanup_output_path(&constant_output);
    cleanup_output_path(&cross_output);
}
