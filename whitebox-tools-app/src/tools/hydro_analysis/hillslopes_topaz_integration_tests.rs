use super::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;
use std::process;
use std::time::{SystemTime, UNIX_EPOCH};

const EXPECTED_HILLSLOPE_ID_OFFSET: i32 = 3;

#[derive(Debug, PartialEq)]
struct NetwRow {
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
    std::env::temp_dir().join(format!("hillslopes_topaz_{}_{}_{}.tif", stem, process::id(), nanos))
}

fn parse_netw_tsv(path: &PathBuf) -> Vec<NetwRow> {
    let payload = fs::read_to_string(path).expect("netw.tsv should be readable");
    let mut lines = payload
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty());

    let header = lines
        .next()
        .expect("netw.tsv should include a header row");
    let columns: Vec<&str> = header.split('\t').collect();
    assert_eq!(
        columns.len(),
        17,
        "netw.tsv should include 17 columns"
    );
    assert_eq!(columns[1], "topaz_id");
    assert_eq!(columns[14], "areaup");
    assert_eq!(columns[15], "is_headwater");
    assert_eq!(columns[16], "is_outlet");

    lines
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(
                fields.len(),
                columns.len(),
                "netw.tsv row should include all 17 columns"
            );
            NetwRow {
                topaz_id: fields[1]
                    .parse::<i32>()
                    .expect("topaz_id field should parse as i32"),
                areaup: fields[14]
                    .parse::<f64>()
                    .expect("areaup field should parse as f64"),
                is_headwater: parse_bool(fields[15]),
                is_outlet: parse_bool(fields[16]),
            }
        })
        .collect()
}

fn parse_bool(text: &str) -> bool {
    match text {
        "true" => true,
        "false" => false,
        _ => panic!("invalid boolean value in netw.tsv: {}", text),
    }
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

#[test]
fn hillslopes_topaz_integration_minimal_1pixel_stream_outputs_expected_hillslopes_and_structure() {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .canonicalize()
        .expect("repo root should resolve");
    let fixture_root = repo_root.join("test_fixtures/minimal_1pixel_stream");

    let dem_path = fixture_root.join("relief.tif");
    let d8_path = fixture_root.join("flovec.tif");
    let stream_path = fixture_root.join("netw0.tif");
    let pour_pts_path = fixture_root.join("outlet.geojson");
    let watershed_path = fixture_root.join("bound.tif");
    let chnjnt_path = fixture_root.join("chnjnt.tif");
    let order_path = fixture_root.join("strahler.tif");
    let reference_subwta_path = fixture_root.join("subwta.tif");
    let reference_netw_path = fixture_root.join("netw.tsv");
    let output_subwta = temp_output_path("minimal_1pixel_subwta");
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
        .expect("hillslopes_topaz minimal fixture integration should run");

    assert!(
        output_subwta.exists(),
        "integration output raster should exist"
    );
    assert!(output_netw.exists(), "integration netw.tsv should exist");

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

    let netw_rows = parse_netw_tsv(&output_netw);
    let reference_netw_rows = parse_netw_tsv(&reference_netw_path);
    assert_eq!(netw_rows.len(), 1, "minimal fixture should emit one channel row");
    assert_eq!(netw_rows[0].topaz_id, 24, "minimal fixture link topaz id should be 24");
    assert!(netw_rows[0].is_headwater, "minimal fixture channel should be headwater");
    assert!(netw_rows[0].is_outlet, "minimal fixture channel should be outlet");
    assert!(
        (netw_rows[0].areaup - 0.0).abs() < 1e-9,
        "minimal fixture areaup should be zero"
    );
    assert_eq!(reference_netw_rows.len(), 1, "fixture baseline should include one channel row");
    assert_eq!(
        reference_netw_rows, netw_rows,
        "generated netw.tsv should match fixture baseline for minimal stream"
    );

    let output_counts = raster_label_counts_by_id(&output, output_nodata);
    let expected_counts = raster_label_counts_by_id(&reference_subwta, reference_nodata);
    let expected_ids: HashSet<i64> = expected_counts.keys().cloned().collect();
    let output_ids: HashSet<i64> = output_counts.keys().cloned().collect();
    assert_eq!(
        output_ids,
        expected_ids,
        "generated subwta ids should match fixture id set"
    );

    let mut channel_ids: HashSet<i64> = HashSet::new();
    let mut stream_cells = 0usize;
    let mut stream_labeled_cells = 0usize;
    for row in 0..stream.configs.rows as isize {
        for col in 0..stream.configs.columns as isize {
            if stream[(row, col)] != stream_nodata && stream[(row, col)] > 0.0 {
                stream_cells += 1;
                let out_id = output[(row, col)];
                if out_id != output_nodata {
                    stream_labeled_cells += 1;
                }
            }
        }
    }
    channel_ids.insert(i64::from(netw_rows[0].topaz_id));
    assert_eq!(
        stream_labeled_cells,
        stream_cells,
        "all stream cells should be labeled"
    );

    for row in 0..output.configs.rows as isize {
        for col in 0..output.configs.columns as isize {
            let watershed_value = watershed[(row, col)];
            if watershed_value == watershed_nodata || watershed_value <= 0.0 {
                continue;
            }

            let output_value = output[(row, col)];
            if output_value != output_nodata && output_value != output_value.round() {
                panic!(
                    "raster value at ({}, {}) is not an integer label: {}",
                    row, col, output_value
                );
            }
            if output_value == output_nodata {
                continue;
            }

            let output_id = output_value as i64;
            if stream[(row, col)] != stream_nodata && stream[(row, col)] > 0.0 {
                assert!(
                    channel_ids.contains(&output_id),
                    "stream-labeled cells should be channel ids"
                );
            }
        }
    }

    assert_eq!(
        output_counts.len(),
        2,
        "minimal fixture should emit one channel id and one hillslope id"
    );
    assert!(
        output_counts.contains_key(&(i64::from(
            netw_rows[0].topaz_id - EXPECTED_HILLSLOPE_ID_OFFSET
        ))),
        "minimal fixture should include the expected source-hillslope id"
    );

    cleanup_output_path(&output_subwta);
    cleanup_output_path(&output_netw);
}
