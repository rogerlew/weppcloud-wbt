use std::path::PathBuf;

use whitebox_raster::vrt::read_vrt;
use whitebox_raster::DataType;

fn data_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("vrt_test_data")
}

fn data_path(rel: &str) -> PathBuf {
    data_root().join(rel)
}

fn assert_invalid(rel: &str) {
    let path = data_path(rel);
    let result = read_vrt(&path);
    assert!(result.is_err(), "Expected error for {:?}", path);
}

#[test]
fn test_parse_valid_vrt() {
    let vrt_path = data_path("vrt/crop_center_100x100.vrt");
    let dataset = read_vrt(&vrt_path).expect("Expected VRT parse to succeed.");

    assert_eq!(dataset.raster_x_size, 100);
    assert_eq!(dataset.raster_y_size, 100);
    assert!(dataset.srs.is_some());
    assert!(dataset.geo_transform.is_some());

    let band = dataset.raster_band;
    assert_eq!(band.band, 1);
    assert_eq!(band.data_type, Some(DataType::F64));

    let source = band.simple_source;
    assert!(!source.relative_to_vrt);
    assert_eq!(source.source_band, 1);
    assert_eq!(source.src_rect.x_off, 200);
    assert_eq!(source.src_rect.y_off, 200);
    assert_eq!(source.src_rect.x_size, 100);
    assert_eq!(source.src_rect.y_size, 100);
    assert_eq!(source.dst_rect.x_off, 0);
    assert_eq!(source.dst_rect.y_off, 0);
}

#[test]
fn test_parse_relative_path_vrt() {
    let vrt_path = data_path("vrt/crop_relative_path.vrt");
    let dataset = read_vrt(&vrt_path).expect("Expected VRT parse to succeed.");
    assert!(dataset.raster_band.simple_source.relative_to_vrt);
    assert!(
        dataset
            .raster_band
            .simple_source
            .source_filename
            .starts_with(".."),
        "Expected relative source filename"
    );
}

#[test]
fn test_parse_float32_vrt() {
    let vrt_path = data_path("vrt/crop_float32_50x50.vrt");
    let dataset = read_vrt(&vrt_path).expect("Expected VRT parse to succeed.");
    assert_eq!(dataset.raster_band.data_type, Some(DataType::F32));
}

#[test]
fn test_parse_fullsize_no_rect() {
    let vrt_path = data_path("vrt/fullsize_no_rect.vrt");
    let dataset = read_vrt(&vrt_path).expect("Expected VRT parse to succeed.");

    assert_eq!(dataset.raster_x_size, 100);
    assert_eq!(dataset.raster_y_size, 100);

    let source = dataset.raster_band.simple_source;
    assert_eq!(source.src_rect.x_off, 0);
    assert_eq!(source.src_rect.y_off, 0);
    assert_eq!(source.src_rect.x_size, 100);
    assert_eq!(source.src_rect.y_size, 100);
    assert_eq!(source.dst_rect.x_off, 0);
    assert_eq!(source.dst_rect.y_off, 0);
    assert_eq!(source.dst_rect.x_size, 100);
    assert_eq!(source.dst_rect.y_size, 100);
}

#[test]
fn test_invalid_band_not_one() {
    assert_invalid("invalid_vrt/band_not_one.vrt");
}

#[test]
fn test_invalid_complex_source() {
    assert_invalid("invalid_vrt/complex_source.vrt");
}

#[test]
fn test_invalid_multiple_sources() {
    assert_invalid("invalid_vrt/multiple_sources.vrt");
}

#[test]
fn test_invalid_negative_offset() {
    assert_invalid("invalid_vrt/negative_offset.vrt");
}

#[test]
fn test_invalid_nonzero_dstrect_offset() {
    assert_invalid("invalid_vrt/nonzero_dstrect_offset.vrt");
}

#[test]
fn test_invalid_size_mismatch() {
    assert_invalid("invalid_vrt/size_mismatch.vrt");
}

#[test]
fn test_invalid_vrt_size_mismatch() {
    assert_invalid("invalid_vrt/vrt_size_mismatch.vrt");
}

#[test]
fn test_invalid_missing_dstrect() {
    assert_invalid("invalid_vrt/missing_dstrect.vrt");
}

#[test]
fn test_invalid_missing_srcrect() {
    assert_invalid("invalid_vrt/missing_srcrect.vrt");
}

#[test]
fn test_invalid_window_out_of_bounds() {
    assert_invalid("invalid_vrt/window_out_of_bounds.vrt");
}
