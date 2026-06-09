use super::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

/// Hillslope suffix to channel-ID offset rule:
/// top=+3, left=+2, right=+1.
const EXPECTED_HILLSLOPE_TOP_OFFSET: i64 = 3;
const EXPECTED_HILLSLOPE_LEFT_OFFSET: i64 = 2;
const EXPECTED_HILLSLOPE_RIGHT_OFFSET: i64 = 1;
const EXPECTED_NETW_COLUMNS: [&str; 17] = [
    "id",
    "topaz_id",
    "ds_x",
    "ds_y",
    "us_x",
    "us_y",
    "inflow0_id",
    "inflow1_id",
    "inflow2_id",
    "length_m",
    "ds_z",
    "us_z",
    "drop_m",
    "order",
    "areaup",
    "is_headwater",
    "is_outlet",
];

/// TOPAZ hillslope convention:
/// A channel ID ends with `4` and its hillslope classes are encoded as suffixes:
/// `1` for top, `2` for left, and `3` for right.
/// Therefore a hillslope ID maps back to its channel by adding a fixed offset:
/// +3 (top), +2 (left), +1 (right).
const TOPAZ_STREAM_SUFFIX: i64 = 4;
const TOPAZ_TOP_HILLSLOPE_SUFFIX: i64 = 1;
const TOPAZ_LEFT_HILLSLOPE_SUFFIX: i64 = 2;
const TOPAZ_RIGHT_HILLSLOPE_SUFFIX: i64 = 3;
const EXPECTED_HILLSLOPE_CLASS_SUFFIX: [i64; 3] = [
    TOPAZ_TOP_HILLSLOPE_SUFFIX,
    TOPAZ_LEFT_HILLSLOPE_SUFFIX,
    TOPAZ_RIGHT_HILLSLOPE_SUFFIX,
];
#[derive(Debug, PartialEq)]
struct NetwRow {
    fields: Vec<String>,
    topaz_id: i32,
    areaup: f64,
    is_headwater: bool,
    is_outlet: bool,
}

fn temp_output_path(stem: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "hillslopes_topaz_{}_{}_{}.tif",
        stem,
        process::id(),
        nanos
    ))
}

fn parse_bool(text: &str) -> bool {
    match text {
        "true" => true,
        "false" => false,
        _ => panic!("invalid boolean value in netw.tsv: {}", text),
    }
}

fn parse_netw_tsv(path: &PathBuf) -> Vec<NetwRow> {
    let payload = fs::read_to_string(path).expect("netw.tsv should be readable");
    let mut lines = payload
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());

    let header = lines.next().expect("netw.tsv should include a header row");
    let columns: Vec<&str> = header.split('\t').collect();
    assert_eq!(
        columns.len(),
        EXPECTED_NETW_COLUMNS.len(),
        "netw.tsv should include 17 columns"
    );
    for (position, (actual, expected)) in columns
        .iter()
        .zip(EXPECTED_NETW_COLUMNS.iter())
        .enumerate()
    {
        assert_eq!(actual, expected, "netw.tsv header mismatch at position {}", position);
    }

    lines
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(
                fields.len(),
                columns.len(),
                "netw.tsv row should include all 17 columns"
            );

            let parsed_int_columns = [0usize, 2, 3, 4, 5, 6, 7, 8, 13];
            for index in parsed_int_columns {
                fields[index]
                    .parse::<i64>()
                    .expect("integer field should parse in netw.tsv");
            }

            let parsed_float_columns = [9usize, 10, 11, 12, 14];
            for index in parsed_float_columns {
                fields[index]
                    .parse::<f64>()
                    .expect("float field should parse in netw.tsv");
            }

            let parsed_bool_columns = [15usize, 16usize];
            for index in parsed_bool_columns {
                parse_bool(fields[index]);
            }

            NetwRow {
                fields: fields.iter().map(|value| value.to_string()).collect(),
                topaz_id: fields[1].parse::<i32>().expect("topaz_id should parse as i32"),
                areaup: fields[14].parse::<f64>().expect("areaup should parse as f64"),
                is_headwater: parse_bool(fields[15]),
                is_outlet: parse_bool(fields[16]),
            }
        })
        .collect()
}

fn cleanup_output_path(path: &PathBuf) {
    let _ = fs::remove_file(path);
    let _ = fs::remove_file(format!("{}.aux.xml", path.to_string_lossy()));
}

fn raster_label_counts_by_id(raster: &Raster, nodata: f64) -> HashMap<i64, usize> {
    let mut counts = HashMap::new();
    for row in 0..raster.configs.rows as isize {
        for col in 0..raster.configs.columns as isize {
            let value = raster[(row, col)];
            if value == nodata {
                continue;
            }
            assert!(
                (value - value.round()).abs() < 1e-6,
                "label at ({}, {}) should be integer-valued: {}",
                row,
                col,
                value
            );
            let value = value as i64;
            *counts.entry(value).or_insert(0) += 1;
        }
    }
    counts
}

fn expected_parent_channel_id(hillslope_id: i64) -> Option<i64> {
    match hillslope_id.rem_euclid(10) {
        TOPAZ_TOP_HILLSLOPE_SUFFIX => Some(hillslope_id + EXPECTED_HILLSLOPE_TOP_OFFSET),
        TOPAZ_LEFT_HILLSLOPE_SUFFIX => Some(hillslope_id + EXPECTED_HILLSLOPE_LEFT_OFFSET),
        TOPAZ_RIGHT_HILLSLOPE_SUFFIX => Some(hillslope_id + EXPECTED_HILLSLOPE_RIGHT_OFFSET),
        _ => None,
    }
}

#[test]
fn hillslopes_topaz_integration_fixture_regression() {
    // Regression lock: stable, representative multi-cell topology fixture.
    // Avoiding the synthetic minimal fixtures, which can drift from the historical
    // sidecar labeling expectations without changing the tool behavior.
    const FIXTURE_NAME: &str = "blackwood_60_5";
    const FIXTURE_SUFFIXES: &[i64] = &EXPECTED_HILLSLOPE_CLASS_SUFFIX;

    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let fixture_root = repo_root.join(format!("test_fixtures/{}", FIXTURE_NAME));

    let dem_path = fixture_root.join("relief.tif");
    let d8_path = fixture_root.join("flovec.tif");
    let stream_path = fixture_root.join("netw0.tif");
    let pour_pts_path = fixture_root.join("outlet.geojson");
    let watershed_path = fixture_root.join("bound.tif");
    let chnjnt_path = fixture_root.join("chnjnt.tif");
    let order_path = fixture_root.join("strahler.tif");
    let reference_subwta_path = fixture_root.join("subwta.tif");
    let reference_netw_path = fixture_root.join("netw.tsv");
    let output_subwta = temp_output_path(&format!("{}_subwta", FIXTURE_NAME));
    let output_netw = output_subwta.with_extension("netw.tsv");

    let tool = HillslopesTopaz::new();
    let args = vec![
        format!("--dem={}", dem_path.display()),
        format!("--d8_pntr={}", d8_path.display()),
        format!("--streams={}", stream_path.display()),
        format!("--pour_pts={}", pour_pts_path.display()),
        format!("--watershed={}", watershed_path.display()),
        format!("--chnjnt={}", chnjnt_path.display()),
        format!("--order={}", order_path.display()),
        format!("--subwta={}", output_subwta.display()),
        format!("--netw={}", output_netw.display()),
    ];
    tool.run(args, "", false)
        .expect("hillslopes_topaz fixture integration should run");

    assert!(
        output_subwta.exists(),
        "integration output raster should exist for {}",
        FIXTURE_NAME
    );
    assert!(output_netw.exists(), "integration netw.tsv should exist for {}", FIXTURE_NAME);

    let output = Raster::new(&output_subwta.to_string_lossy(), "r")
        .expect("output subwta raster should open");
    let watershed = Raster::new(&watershed_path.to_string_lossy(), "r")
        .expect("watershed raster should open");
    let stream = Raster::new(&stream_path.to_string_lossy(), "r")
        .expect("stream raster should open");
    let reference_subwta = Raster::new(&reference_subwta_path.to_string_lossy(), "r")
        .expect("reference subwta raster should open");

    let output_nodata = output.configs.nodata;
    let watershed_nodata = watershed.configs.nodata;
    let stream_nodata = stream.configs.nodata;
    let reference_nodata = reference_subwta.configs.nodata;

    assert_eq!(
        output.configs.rows,
        reference_subwta.configs.rows,
        "output and reference should have matching row counts for {}",
        FIXTURE_NAME
    );
    assert_eq!(
        output.configs.columns,
        reference_subwta.configs.columns,
        "output and reference should have matching column counts for {}",
        FIXTURE_NAME
    );

    let netw_rows = parse_netw_tsv(&output_netw);
    let reference_netw_rows = parse_netw_tsv(&reference_netw_path);
    assert!(
        !reference_netw_rows.is_empty(),
        "fixture {} should include at least one netw row",
        FIXTURE_NAME
    );
    assert_eq!(
        netw_rows,
        reference_netw_rows,
        "generated netw.tsv should match fixture baseline for {}",
        FIXTURE_NAME
    );

    let output_counts = raster_label_counts_by_id(&output, output_nodata);
    let expected_counts = raster_label_counts_by_id(&reference_subwta, reference_nodata);
    assert_eq!(
        output_counts,
        expected_counts,
        "generated raster should match baseline pixel counts by ID for {}",
        FIXTURE_NAME
    );

    let channel_ids: HashSet<i64> = netw_rows.iter().map(|row| i64::from(row.topaz_id)).collect();
    let mut observed_stream_cells = 0usize;
    let mut expected_stream_cells = 0usize;
    let mut observed_channel_ids = HashSet::new();
    let mut observed_hillslope_suffixes = HashSet::new();

    for row in 0..output.configs.rows as isize {
        for col in 0..output.configs.columns as isize {
            let output_in_watershed = watershed[(row, col)] != watershed_nodata
                && watershed[(row, col)] > 0.0;
            let on_stream = stream[(row, col)] != stream_nodata && stream[(row, col)] > 0.0;
            if output_in_watershed && on_stream {
                expected_stream_cells += 1;
            }
        }
    }

    for row in 0..output.configs.rows as isize {
        for col in 0..output.configs.columns as isize {
            let output_value = output[(row, col)];
            let output_in_watershed = watershed[(row, col)] != watershed_nodata
                && watershed[(row, col)] > 0.0;
            let on_stream = stream[(row, col)] != stream_nodata && stream[(row, col)] > 0.0;

            if !output_in_watershed {
                assert!(
                    output_value == output_nodata,
                    "non-watershed cells should remain nodata for {} at ({}, {})",
                    FIXTURE_NAME,
                    row,
                    col
                );
                continue;
            }

            if on_stream {
                observed_stream_cells += 1;
                assert_ne!(
                    output_value,
                    output_nodata,
                    "stream cells in the watershed should be labeled for {} at ({}, {})",
                    FIXTURE_NAME,
                    row,
                    col
                );
                let output_id = output_value as i64;
                assert_eq!(
                    output_id.rem_euclid(10),
                    TOPAZ_STREAM_SUFFIX,
                    "stream cell should carry channel id (ending in 4) for {} at ({}, {})",
                    FIXTURE_NAME,
                    row,
                    col
                );
                assert!(
                    channel_ids.contains(&output_id),
                    "stream cell label should be declared in baseline netw.tsv for {} at ({}, {})",
                    FIXTURE_NAME,
                    row,
                    col
                );
                observed_channel_ids.insert(output_id);
                continue;
            }

            assert_ne!(
                output_value,
                output_nodata,
                "all watershed cells should be assigned a hillslope ID for {} at ({}, {})",
                FIXTURE_NAME,
                row,
                col
            );
            let output_id = output_value as i64;

            let output_suffix = output_id.rem_euclid(10);
            assert!(
                EXPECTED_HILLSLOPE_CLASS_SUFFIX.contains(&output_suffix),
                "non-stream hillslope should be top/left/right for {} at ({}, {})",
                FIXTURE_NAME,
                row,
                col
            );
            observed_hillslope_suffixes.insert(output_suffix);
            let parent = expected_parent_channel_id(output_id).expect(
                "non-stream hillslope id should map to a channel-derived topaz class",
            );
            assert!(
                channel_ids.contains(&parent),
                "hillslope id {} should map to channel id {} in {}",
                output_id,
                parent,
                FIXTURE_NAME
            );
        }
    }

    assert!(
        observed_stream_cells > 0,
        "fixture {} should include at least one stream cell",
        FIXTURE_NAME
    );
    assert_eq!(
        observed_stream_cells,
        expected_stream_cells,
        "all stream cells should be labeled for {}",
        FIXTURE_NAME
    );
    for channel_id in &channel_ids {
        assert!(
            observed_channel_ids.contains(channel_id),
            "each output link id should be present on a stream cell in {}: {}",
            FIXTURE_NAME,
            channel_id
        );
    }
    for required_suffix in FIXTURE_SUFFIXES {
        assert!(
            observed_hillslope_suffixes.contains(required_suffix),
            "fixture {} should include hillslope suffix {}",
            FIXTURE_NAME,
            required_suffix
        );
    }

    let expected_label_count = expected_counts.values().sum::<usize>();
    assert!(
        !expected_counts.is_empty() && expected_label_count > 0,
        "fixture {} should contain at least one raster label",
        FIXTURE_NAME
    );

    cleanup_output_path(&output_subwta);
    cleanup_output_path(&output_netw);
}
