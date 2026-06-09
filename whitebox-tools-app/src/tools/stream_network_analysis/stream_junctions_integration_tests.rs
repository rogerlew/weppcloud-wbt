use super::*;
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

const CHNJNT_BACKGROUND: f64 = -32768.0;

fn temp_output_path(stem: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "stream_junctions_{}_{}_{}.tif",
        stem,
        process::id(),
        nanos
    ))
}

fn cleanup_output_path(path: &PathBuf) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(format!("{}.aux.xml", path.to_string_lossy()));
}

fn parse_row_col_from_fixture(readme_path: &PathBuf, marker: &str) -> (isize, isize) {
    let payload = fs::read_to_string(readme_path).expect("fixture README should be readable");
    for line in payload.lines() {
        if line.contains(marker) {
            let mut row: Option<isize> = None;
            let mut col: Option<isize> = None;
            for token in line.split(|c| c == ',' || c == ' ') {
                if let Some(rest) = token.strip_prefix("row=") {
                    row = Some(
                        rest.parse::<isize>()
                            .expect("fixture README row should parse as integer"),
                    );
                } else if let Some(rest) = token.strip_prefix("col=") {
                    col = Some(
                        rest.parse::<isize>()
                            .expect("fixture README col should parse as integer"),
                    );
                }
            }
            if let (Some(row), Some(col)) = (row, col) {
                return (row, col);
            }
        }
    }
    panic!("could not parse {} from fixture README", marker);
}

fn run_stream_junctions(
    fixture_root: &PathBuf,
    esri_style: bool,
) -> PathBuf {
    let d8_path = fixture_root.join("flovec.tif");
    let streams_path = fixture_root.join("netw0.tif");
    let output_path = temp_output_path(if esri_style {
        "esri_mode"
    } else {
        "default_mode"
    });
    let tool = StreamJunctionIdentifier::new();
    let mut args = vec![
        format!("--d8_pntr={}", d8_path.display()),
        format!("--streams={}", streams_path.display()),
        format!("--output={}", output_path.display()),
    ];
    if esri_style {
        args.push("--esri_pntr=true".to_string());
    }
    tool.run(args, "", false).expect("stream_junctions should execute");
    output_path
}

#[test]
fn blackwood_regression() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let fixture_root = repo_root.join("test_fixtures/blackwood_60_5");

    let d8_path = fixture_root.join("flovec.tif");
    let streams_path = fixture_root.join("netw0.tif");
    let reference_path = fixture_root.join("chnjnt.tif");
    let output_path = temp_output_path("blackwood_regression");

    let tool = StreamJunctionIdentifier::new();
    let args = vec![
        format!("--d8_pntr={}", d8_path.display()),
        format!("--streams={}", streams_path.display()),
        format!("--output={}", output_path.display()),
    ];
    tool.run(args, "", false).expect("stream_junctions should run on blackwood fixture");

    let output = Raster::new(&output_path.to_string_lossy(), "r")
        .expect("stream_junction output should open");
    let reference = Raster::new(&reference_path.to_string_lossy(), "r")
        .expect("reference chnjnt should open");
    let streams = Raster::new(&streams_path.to_string_lossy(), "r")
        .expect("streams input should open");

    assert_eq!(output.configs.rows, reference.configs.rows, "rows should match");
    assert_eq!(output.configs.columns, reference.configs.columns, "columns should match");

    let mut found_junction = false;
    let mut stream_cell_count = 0usize;
    for row in 0..output.configs.rows as isize {
        for col in 0..output.configs.columns as isize {
            let reference_value = reference[(row, col)];
            let output_value = output[(row, col)];
            let stream_value = streams[(row, col)];
            if stream_value > 0.0 {
                stream_cell_count += 1;
                assert_eq!(output_value, reference_value, "blackwood regression should match reference where valid");
                if output_value >= 2.0 {
                    found_junction = true;
                }
            } else {
                assert_eq!(
                    output_value,
                    CHNJNT_BACKGROUND,
                    "non-stream cells should use junction background"
                );
            }
        }
    }

    assert!(stream_cell_count > 0, "blackwood fixture should include stream cells");
    assert!(found_junction, "blackwood fixture should include real junctions");
    cleanup_output_path(&output_path);
}

#[test]
fn minimal_1pixel_structural() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let fixture_root = repo_root.join("test_fixtures/minimal_1pixel_stream");

    let streams_path = fixture_root.join("netw0.tif");
    let output_path = temp_output_path("minimal_1pixel_structural");
    let d8_path = fixture_root.join("flovec.tif");

    let tool = StreamJunctionIdentifier::new();
    let args = vec![
        format!("--d8_pntr={}", d8_path.display()),
        format!("--streams={}", streams_path.display()),
        format!("--output={}", output_path.display()),
        "--esri_pntr=true".to_string(),
    ];
    tool.run(args, "", false).expect("stream_junctions should run on 1-pixel fixture");

    let output = Raster::new(&output_path.to_string_lossy(), "r")
        .expect("stream_junction output should open");
    let streams = Raster::new(&streams_path.to_string_lossy(), "r")
        .expect("streams input should open");
    let background = CHNJNT_BACKGROUND;

    assert_eq!(output.configs.rows, streams.configs.rows, "output rows should match streams");
    assert_eq!(output.configs.columns, streams.configs.columns, "output columns should match streams");

    let mut stream_cells = 0usize;
    let mut non_background_cells = 0usize;
    for row in 0..output.configs.rows as isize {
        for col in 0..output.configs.columns as isize {
            let stream_value = streams[(row, col)];
            let output_value = output[(row, col)];
            if stream_value > 0.0 {
                stream_cells += 1;
                non_background_cells += 1;
                assert_eq!(
                    output_value,
                    0.0,
                    "1-pixel stream fixture should produce a single zero junction count"
                );
            } else {
                assert_eq!(
                    output_value,
                    background,
                    "1-pixel stream fixture should use background for non-stream cells"
                );
            }
        }
    }

    assert_eq!(stream_cells, 1, "1-pixel fixture should include exactly one stream cell");
    assert_eq!(non_background_cells, 1, "1-pixel fixture should expose exactly one non-background output value");
    cleanup_output_path(&output_path);
}

#[test]
fn minimal_2pixel_known_geometry() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let fixture_root = repo_root.join("test_fixtures/minimal_2pixel_stream");

    let readme_path = fixture_root.join("README.md");
    let streams_path = fixture_root.join("netw0.tif");
    let d8_path = fixture_root.join("flovec.tif");
    let output_path = temp_output_path("minimal_2pixel_known_geometry");
    let (upstream_row, upstream_col) = parse_row_col_from_fixture(&readme_path, "Upstream pixel");
    let (outlet_row, outlet_col) = parse_row_col_from_fixture(&readme_path, "Outlet pixel");

    let tool = StreamJunctionIdentifier::new();
    let args = vec![
        format!("--d8_pntr={}", d8_path.display()),
        format!("--streams={}", streams_path.display()),
        format!("--output={}", output_path.display()),
        "--esri_pntr=true".to_string(),
    ];
    tool.run(args, "", false).expect("stream_junctions should run on 2-pixel fixture");

    let output = Raster::new(&output_path.to_string_lossy(), "r")
        .expect("stream_junction output should open");
    let streams = Raster::new(&streams_path.to_string_lossy(), "r")
        .expect("streams input should open");
    let background = CHNJNT_BACKGROUND;

    assert_eq!(output.configs.rows, streams.configs.rows, "output rows should match streams");
    assert_eq!(output.configs.columns, streams.configs.columns, "output columns should match streams");

    let mut stream_cells = 0usize;
    let mut non_background_cells = 0usize;
    let mut zero_count = 0usize;
    let mut one_count = 0usize;

    for row in 0..output.configs.rows as isize {
        for col in 0..output.configs.columns as isize {
            let stream_value = streams[(row, col)];
            let output_value = output[(row, col)];
            if stream_value > 0.0 {
                stream_cells += 1;
                non_background_cells += 1;
                assert!(output_value < 2.0, "2-pixel fixture should not produce junctions");
                if output_value == 0.0 {
                    zero_count += 1;
                } else if output_value == 1.0 {
                    one_count += 1;
                }
            } else {
                assert_eq!(
                    output_value,
                    background,
                    "2-pixel fixture should use background for non-stream cells"
                );
            }
        }
    }

    assert_eq!(stream_cells, 2, "2-pixel fixture should contain two stream cells");
    assert_eq!(non_background_cells, 2, "2-pixel fixture should expose two non-background stream cells");
    assert_eq!(zero_count, 1, "2-pixel fixture should include one headwater");
    assert_eq!(one_count, 1, "2-pixel fixture should include one outlet count of 1");

    assert_eq!(
        output[(upstream_row, upstream_col)],
        0.0,
        "upstream pixel should be headwater with zero inflows"
    );
    assert_eq!(
        output[(outlet_row, outlet_col)],
        1.0,
        "outlet pixel should have one upstream neighbor"
    );

    cleanup_output_path(&output_path);
}

#[test]
fn blackwood_esri_differs() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let fixture_root = repo_root.join("test_fixtures/blackwood_60_5");

    let output_default = run_stream_junctions(&fixture_root, false);
    let output_esri = run_stream_junctions(&fixture_root, true);

    let default = Raster::new(&output_default.to_string_lossy(), "r")
        .expect("default stream_junction output should open");
    let esri = Raster::new(&output_esri.to_string_lossy(), "r")
        .expect("esri stream_junction output should open");

    assert_eq!(
        default.configs.rows,
        esri.configs.rows,
        "default and esri outputs should have same rows"
    );
    assert_eq!(
        default.configs.columns,
        esri.configs.columns,
        "default and esri outputs should have same columns"
    );

    let mut differing_valid_cells = 0usize;
    for row in 0..default.configs.rows as isize {
        for col in 0..default.configs.columns as isize {
            let default_value = default[(row, col)];
            let esri_value = esri[(row, col)];
            if default_value != CHNJNT_BACKGROUND && esri_value != CHNJNT_BACKGROUND {
                if default_value != esri_value {
                    differing_valid_cells += 1;
                }
            }
        }
    }

    assert!(
        differing_valid_cells > 0,
        "ESRI flow encoding should change at least one valid junction count"
    );

    cleanup_output_path(&output_default);
    cleanup_output_path(&output_esri);
}
