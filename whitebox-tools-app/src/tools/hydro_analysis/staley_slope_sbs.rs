// Independently derived Horn gradient and three-state intersection. License: MIT.
use crate::tools::*;
use serde_json::json;
use std::collections::BTreeMap;
use std::fs::{self, File};
use std::io::{Error, ErrorKind, Write};
use std::path::{Path, PathBuf};
use whitebox_raster::{DataType, PhotometricInterpretation, Raster, RasterConfigs};

fn invalid(message: impl Into<String>) -> Error {
    Error::new(ErrorKind::InvalidInput, message.into())
}
const MAX_CELLS: usize = 10_000_000;
const NODATA: f64 = -32768.;

pub struct StaleySlopeSbs {
    parameters: Vec<ToolParameter>,
}
impl StaleySlopeSbs {
    pub fn new() -> Self {
        let parameters = [
            "dem",
            "sbs",
            "mask",
            "output_dir",
            "elevation_units",
            "sbs_classes",
        ]
        .iter()
        .map(|flag| ToolParameter {
            name: flag.to_string(),
            flags: vec![format!("--{}", flag)],
            description: match *flag {
                "sbs_classes" => "Four distinct integer codes: unburned,low,moderate,high",
                "output_dir" => "Fresh output directory; existing parent required",
                "elevation_units" => "Required meter declaration: m",
                _ => "Aligned uncompressed single-band GeoTIFF",
            }
            .into(),
            parameter_type: if ["dem", "sbs", "mask"].contains(flag) {
                ParameterType::ExistingFile(ParameterFileType::Raster)
            } else {
                ParameterType::String
            },
            default_value: None,
            optional: false,
        })
        .collect();
        Self { parameters }
    }
}

fn parse(args: &[String]) -> Result<BTreeMap<String, String>, Error> {
    let mut result = BTreeMap::new();
    let mut i = 0;
    while i < args.len() {
        let (flag, inline) = args[i]
            .split_once('=')
            .map(|(a, b)| (a, Some(b)))
            .unwrap_or((&args[i], None));
        let key = flag.trim_start_matches('-').to_lowercase();
        if ![
            "dem",
            "sbs",
            "mask",
            "output_dir",
            "elevation_units",
            "sbs_classes",
        ]
        .contains(&key.as_str())
        {
            return Err(invalid(format!("Unknown argument: {}", flag)));
        }
        let value = match inline {
            Some(v) => v,
            None => {
                i += 1;
                args.get(i)
                    .ok_or_else(|| invalid("Missing argument value"))?
            }
        };
        let value = if value.len() >= 2
            && ((value.starts_with('\'') && value.ends_with('\''))
                || (value.starts_with('"') && value.ends_with('"')))
        {
            &value[1..value.len() - 1]
        } else {
            value
        };
        if value.is_empty() || value.starts_with("--") || result.insert(key, value.into()).is_some()
        {
            return Err(invalid("Empty or duplicate argument"));
        }
        i += 1;
    }
    if result.len() != 6 || result.get("elevation_units").map(String::as_str) != Some("m") {
        return Err(invalid(
            "Required: --dem --sbs --mask --output_dir --elevation_units=m --sbs_classes",
        ));
    }
    Ok(result)
}
fn path(name: &str, wd: &str) -> PathBuf {
    let p = Path::new(name);
    if p.is_absolute() {
        p.into()
    } else {
        Path::new(if wd.is_empty() { "." } else { wd }).join(p)
    }
}
fn missing(value: f64, nodata: f64) -> bool {
    value == nodata || (value.is_nan() && nodata.is_nan())
}

fn meters(units: &str) -> bool {
    matches!(
        units.to_lowercase().as_str(),
        "m" | "meter" | "meters" | "metre" | "metres" | "linear_meter"
    )
}

fn validate_grid(c: &RasterConfigs) -> Result<(), Error> {
    if !((32601..=32660).contains(&c.epsg_code) || (32701..=32760).contains(&c.epsg_code)) {
        return Err(invalid(
            "Only WGS84 UTM meter grids (EPSG:32601-32660/32701-32760) are supported.",
        ));
    }
    let point_pixels = c
        .geo_key_directory
        .get(4..)
        .unwrap_or(&[])
        .chunks_exact(4)
        .any(|key| key[0] == 1025 && (key[1] != 0 || key[2] != 1 || key[3] != 1));
    if c.bands != 1
        || point_pixels
        || !c.pixel_is_area
        || c.model_transformation.iter().any(|v| *v != 0.0)
    {
        return Err(invalid(
            "Require a single-band, north-up pixel-area grid without a transformation matrix.",
        ));
    }
    if c.xy_units != "not specified" && !meters(&c.xy_units) {
        return Err(invalid("Horizontal units must be meters."));
    }
    let source_scale = c
        .source_pixel_scale
        .ok_or_else(|| invalid("Original GeoTIFF scale unavailable."))?;
    if source_scale[0] <= 0.0 || source_scale[1] <= 0.0 {
        return Err(invalid(
            "Missing or nonpositive original GeoTIFF pixel scale.",
        ));
    }
    if c.rows == 0
        || c.columns == 0
        || ![
            c.north,
            c.south,
            c.east,
            c.west,
            c.resolution_x,
            c.resolution_y,
        ]
        .iter()
        .all(|x| x.is_finite())
        || c.resolution_x <= 0.0
        || c.resolution_y <= 0.0
        || c.north < c.south
        || c.east < c.west
    {
        return Err(invalid("Invalid raster extent, size or spacing."));
    }
    // The owned GeoTIFF reader stores the last pixel origin as east/south.
    // Require the conventional single top-left tiepoint, avoiding its general
    // tiepoint-offset path; compare input grids using the same representation.
    if c.model_tiepoint.len() != 6 || c.model_tiepoint[0] != 0.0 || c.model_tiepoint[1] != 0.0 {
        return Err(invalid("Require a single top-left GeoTIFF tiepoint."));
    }
    let width = c.resolution_x * c.columns.saturating_sub(1) as f64;
    let height = c.resolution_y * c.rows.saturating_sub(1) as f64;
    if ((c.east - c.west) - width).abs() > 1e-8 * c.resolution_x
        || ((c.north - c.south) - height).abs() > 1e-8 * c.resolution_y
    {
        return Err(invalid("Raster extent does not agree with cell spacing."));
    }
    Ok(())
}

// Bound the first TIFF IFD before invoking the owned decoder. Only the small
// uncompressed strip subset needed by this local interface is accepted.
fn preflight(path: &Path) -> Result<String, Error> {
    let meta = fs::metadata(path)?;
    if !meta.is_file() || meta.len() > 512 * 1024 * 1024 {
        return Err(invalid("Require regular GeoTIFF <=512 MiB"));
    }
    let ext = path
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ext != "tif" && ext != "tiff" {
        return Err(invalid("Require .tif/.tiff"));
    }
    let b = fs::read(path)?;
    let le = match b.get(..2) {
        Some(b"II") => true,
        Some(b"MM") => false,
        _ => return Err(invalid("Invalid TIFF byte order")),
    };
    let number = |offset: usize, width: usize| -> Result<u64, Error> {
        let bytes = b
            .get(
                offset
                    ..offset
                        .checked_add(width)
                        .ok_or_else(|| invalid("TIFF offset overflow"))?,
            )
            .ok_or_else(|| invalid("Truncated TIFF"))?;
        let mut value = 0u64;
        for i in 0..width {
            value = (value << 8) | bytes[if le { width - 1 - i } else { i }] as u64;
        }
        Ok(value)
    };
    if number(2, 2)? != 42 {
        return Err(invalid("Only classic TIFF supported"));
    }
    let offset = number(4, 4)? as usize;
    let count = number(offset, 2)? as usize;
    if count == 0 || count > 256 {
        return Err(invalid("Invalid TIFF tag count"));
    }
    let mut tags = BTreeMap::<u64, Vec<u64>>::new();
    let mut metadata_bytes = 0usize;
    let mut declared_nodata = false;
    let mut nodata_value = 0f64;
    let mut locations = BTreeMap::new();
    for i in 0..count {
        let pos = offset + 2 + i * 12;
        let tag = number(pos, 2)?;
        let kind = number(pos + 2, 2)?;
        let n = number(pos + 4, 4)? as usize;
        let width = match kind {
            1 | 2 | 6 | 7 => 1,
            3 | 8 => 2,
            4 | 9 | 11 => 4,
            5 | 10 | 12 => 8,
            _ => return Err(invalid("Unsupported TIFF tag type")),
        };
        let size = n
            .checked_mul(width)
            .ok_or_else(|| invalid("TIFF size overflow"))?;
        metadata_bytes = metadata_bytes
            .checked_add(size)
            .ok_or_else(|| invalid("Metadata size overflow"))?;
        if metadata_bytes > 64 * 1024 * 1024 || size > 16 * 1024 * 1024 || n == 0 {
            return Err(invalid("Invalid TIFF tag size"));
        }
        let start = if size <= 4 {
            pos + 8
        } else {
            number(pos + 8, 4)? as usize
        };
        if start
            .checked_add(size)
            .filter(|end| *end <= b.len())
            .is_none()
        {
            return Err(invalid("TIFF tag extends past EOF"));
        }
        locations.insert(tag, (start, size, kind, n));
        match tag {
            33550 if kind != 12 || n != 3 => {
                return Err(invalid("Require three DOUBLE pixel scales"))
            }
            33922 if kind != 12 || n != 6 => {
                return Err(invalid("Require one six-DOUBLE tiepoint"))
            }
            34264 => return Err(invalid("Transformation matrices unsupported")),
            34735 if kind != 3 || n < 4 => return Err(invalid("Invalid GeoKey directory")),
            34736 if kind != 12 => return Err(invalid("Invalid GeoDouble parameters")),
            34737 if kind != 2 => return Err(invalid("Invalid GeoASCII parameters")),
            _ => (),
        }
        if tag == 42113 {
            if kind != 2 {
                return Err(invalid("NoData tag must be ASCII"));
            }
            let text = std::str::from_utf8(&b[start..start + size])
                .map_err(|_| invalid("Invalid NoData text"))?;
            let value = text
                .trim_end_matches('\0')
                .trim()
                .parse::<f64>()
                .map_err(|_| invalid("Invalid NoData number"))?;
            if !value.is_finite() {
                return Err(invalid(
                    "Require finite NoData; legacy reader normalizes nonfinite sentinels",
                ));
            }
            nodata_value = value;
            declared_nodata = true;
        }
        if tags.contains_key(&tag) {
            return Err(invalid("Duplicate TIFF tag"));
        }
        // Metadata is checked for bounds but only scalar integer layout tags
        // are decoded here; georeferencing remains the owned reader's job.
        let wanted = [
            256, 257, 258, 259, 262, 273, 277, 278, 279, 284, 317, 322, 323, 324, 325, 339,
        ];
        let values = if wanted.contains(&tag) {
            if ![3, 4].contains(&kind) {
                return Err(invalid("Invalid TIFF layout tag type"));
            }
            (0..n)
                .map(|j| number(start + j * width, width))
                .collect::<Result<Vec<_>, _>>()?
        } else {
            vec![]
        };
        tags.insert(tag, values);
    }
    if let Some(&(start, _, _, n)) = locations.get(&34735) {
        let keys = number(start + 6, 2)? as usize;
        if keys > 256 || n < 4 + 4 * keys {
            return Err(invalid("Invalid or excessive GeoKey count"));
        }
        for i in 0..keys {
            let entry = start + 8 + i * 8;
            let location = number(entry + 2, 2)?;
            let count = number(entry + 4, 2)? as usize;
            let offset = number(entry + 6, 2)? as usize;
            if location == 0 {
                if count != 1 {
                    return Err(invalid("Invalid inline GeoKey count"));
                }
            } else {
                if ![34735, 34736, 34737].contains(&location) {
                    return Err(invalid("Unsupported GeoKey reference"));
                }
                let (_, _, _, length) = locations
                    .get(&location)
                    .ok_or_else(|| invalid("Missing GeoKey parameter tag"))?;
                if count == 0
                    || offset
                        .checked_add(count)
                        .filter(|end| end <= length)
                        .is_none()
                {
                    return Err(invalid("GeoKey reference outside parameter tag"));
                }
            }
        }
    }
    if !declared_nodata {
        return Err(invalid(
            "Explicit GDAL_NODATA tag required; prepare a raster with declared NoData",
        ));
    }
    if number(offset + 2 + count * 12, 4)? != 0 {
        return Err(invalid("Multiple TIFF images unsupported"));
    }
    let scalar = |tag: u64, default: Option<u64>| -> Result<u64, Error> {
        match tags.get(&tag) {
            Some(v) if v.len() == 1 => Ok(v[0]),
            None => default.ok_or_else(|| invalid("Missing TIFF layout tag")),
            _ => Err(invalid("Non-scalar TIFF layout tag")),
        }
    };
    let cols = scalar(256, None)? as usize;
    let rows = scalar(257, None)? as usize;
    if rows == 0 || cols == 0 || rows.checked_mul(cols).filter(|n| *n <= MAX_CELLS).is_none() {
        return Err(invalid("Raster exceeds 10 million cells or is empty"));
    }
    let bits = scalar(258, None)?;
    let format = scalar(339, None)?;
    if bits == 32 && format == 3 && (nodata_value as f32).is_infinite() {
        return Err(invalid("NoData is not representable as finite Float32"));
    }

    if !((format == 1 || format == 2) && [8, 16, 32].contains(&bits)
        || format == 3 && [32, 64].contains(&bits))
    {
        return Err(invalid("Unsupported scalar raster dtype"));
    }
    if scalar(259, Some(1))? != 1
        || scalar(277, Some(1))? != 1
        || ![1, 2].contains(&scalar(284, Some(1))?)
        || scalar(317, Some(1))? != 1
        || scalar(262, None)? != 1
        || [322, 323, 324, 325].iter().any(|t| tags.contains_key(t))
    {
        return Err(invalid(
            "Require uncompressed single-band grayscale strips without predictor or palette; prepare numeric SBS without its display color table",
        ));
    }
    let strip_rows = scalar(278, None)? as usize;
    if strip_rows == 0 {
        return Err(invalid("Invalid rows per strip"));
    }
    let offsets = tags
        .get(&273)
        .ok_or_else(|| invalid("Missing strip offsets"))?;
    let sizes = tags
        .get(&279)
        .ok_or_else(|| invalid("Missing strip sizes"))?;
    let strips = (rows - 1) / strip_rows + 1;
    if offsets.len() != strips || sizes.len() != strips {
        return Err(invalid("Strip count mismatch"));
    }
    for i in 0..strips {
        let expected = strip_rows.min(rows - i * strip_rows) * cols * (bits as usize / 8);
        if sizes[i] != expected as u64
            || offsets[i]
                .checked_add(sizes[i])
                .filter(|end| *end <= b.len() as u64)
                .is_none()
        {
            return Err(invalid("Invalid strip byte range"));
        }
    }
    let hash = b.iter().fold(0xcbf29ce484222325u64, |h, x| {
        (h ^ *x as u64).wrapping_mul(0x100000001b3)
    });
    Ok(format!("{:016x}", hash))
}

fn horn(z: [f64; 9], dx: f64) -> Result<f64, Error> {
    // Difference pairs reduce cancellation from a large constant elevation.
    let gx = ((z[2] - z[0]) + 2. * (z[5] - z[3]) + (z[8] - z[6])) / (8. * dx);
    let gy = ((z[6] - z[0]) + 2. * (z[7] - z[1]) + (z[8] - z[2])) / (8. * dx);
    let gradient = gx.hypot(gy);
    if !gradient.is_finite() {
        return Err(invalid("Gradient overflow"));
    }
    Ok(gradient)
}
fn intersection(steep: Option<bool>, burned: Option<bool>) -> u8 {
    match (steep, burned) {
        (Some(false), _) | (_, Some(false)) => 0,
        (Some(true), Some(true)) => 1,
        _ => 2,
    }
}

impl WhiteboxTool for StaleySlopeSbs {
    fn get_source_file(&self) -> String {
        file!().into()
    }
    fn get_tool_name(&self) -> String {
        "StaleySlopeSbs".into()
    }
    fn get_tool_description(&self) -> String {
        "Horn surface slope and uncertainty-preserving watershed SBS intersection.".into()
    }
    fn get_tool_parameters(&self) -> String {
        json!({"parameters":self.parameters}).to_string()
    }
    fn get_example_usage(&self) -> String {
        "whitebox_tools -r=StaleySlopeSbs --dem=raw.tif --sbs=sbs.tif --mask=bound.tif --output_dir=result --elevation_units=m --sbs_classes=0,1,2,3".into()
    }
    fn get_toolbox(&self) -> String {
        "Hydrological Analysis".into()
    }
    fn run<'a>(&self, args: Vec<String>, wd: &'a str, verbose: bool) -> Result<(), Error> {
        let args = parse(&args)?;
        let codes = args["sbs_classes"]
            .split(',')
            .map(|s| {
                s.parse::<i32>()
                    .map_err(|_| invalid("SBS codes must be four distinct i32 integers"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if codes.len() != 4
            || codes
                .iter()
                .enumerate()
                .any(|(i, v)| codes[..i].contains(v))
        {
            return Err(invalid("SBS codes must be four distinct integers"));
        }
        let destination = path(&args["output_dir"], wd);
        let parent = destination
            .parent()
            .ok_or_else(|| invalid("Output parent required"))?
            .canonicalize()?;
        let destination = parent.join(
            destination
                .file_name()
                .ok_or_else(|| invalid("Output name required"))?,
        );
        match fs::symlink_metadata(&destination) {
            Ok(_) => return Err(invalid("Output directory already exists")),
            Err(e) if e.kind() == ErrorKind::NotFound => (),
            Err(e) => return Err(e),
        }
        let mut rasters = Vec::new();
        let mut sources = BTreeMap::new();
        for key in ["dem", "sbs", "mask"] {
            let p = path(&args[key], wd).canonicalize()?;
            let fingerprint = preflight(&p)?;
            // Deliberate decoder boundary: legacy owned GeoTIFF metadata parsing
            // can panic on malformed keys. Preflight bounds allocations/layout;
            // convert decoder panics to an explicit failed-input contract.
            let name = p.to_str().ok_or_else(|| invalid("Non-UTF8 input path"))?;
            let r = std::panic::catch_unwind(|| Raster::new(name, "r")).map_err(|_| {
                invalid(format!(
                    "GeoTIFF decoder rejected malformed metadata: {}",
                    p.display()
                ))
            })??;
            if !r.configs.nodata.is_finite() {
                return Err(invalid("Decoded NoData must be finite"));
            }
            validate_grid(&r.configs)?;
            if r.configs.resolution_x != r.configs.resolution_y {
                return Err(invalid("Require square cells"));
            }
            sources.insert(
                key,
                json!({"path":p,"bytes":fs::metadata(&p)?.len(),"fnv1a64":fingerprint}),
            );
            rasters.push(r);
        }
        let dem = &rasters[0];
        let sbs = &rasters[1];
        let mask = &rasters[2];
        let c = &dem.configs;
        if c.z_units != "not specified" && !meters(&c.z_units) {
            return Err(invalid("DEM vertical units contradict meters declaration"));
        }
        for r in &rasters[1..] {
            let p = &r.configs;
            if c.rows != p.rows
                || c.columns != p.columns
                || c.epsg_code != p.epsg_code
                || c.source_pixel_scale != p.source_pixel_scale
                || c.model_tiepoint != p.model_tiepoint
            {
                return Err(invalid("All grids must align exactly"));
            }
        }
        if codes.iter().any(|v| *v as f64 == sbs.configs.nodata) {
            return Err(invalid("SBS class collides with NoData"));
        }
        let rows = c.rows;
        let cols = c.columns;
        let mut outputs = Vec::new();
        for (i, name) in ["slope.tif", "intersection.tif", "support.tif"]
            .iter()
            .enumerate()
        {
            let mut out = Raster::initialize_using_file(
                destination
                    .join(name)
                    .to_str()
                    .ok_or_else(|| invalid("Non-UTF8 output path"))?,
                dem,
            );
            out.configs.data_type = if i == 0 { DataType::F64 } else { DataType::U8 };
            out.configs.nodata = if i == 0 { NODATA } else { 255. };
            out.configs.geo_key_directory.clear();
            out.configs.geo_double_params.clear();
            out.configs.geo_ascii_params.clear();
            out.configs.z_units = "not specified".into();
            out.configs.photometric_interp = PhotometricInterpretation::Continuous;
            out.reinitialize_values(out.configs.nodata);
            out.configs.minimum = 0.;
            out.configs.maximum = if i == 0 {
                90.
            } else if i == 1 {
                2.
            } else {
                3.
            };
            out.add_metadata_entry("StaleySlopeSbs: raw meter DEM; Horn 3x3; strict nine-valid-cell edges; threshold gradient >= tan(23 degrees)".into());
            outputs.push(out);
        }
        let mut counts = BTreeMap::<&str, u64>::new();
        for key in [
            "basin",
            "slope_valid",
            "sbs_valid",
            "jointly_valid",
            "steep",
            "sbs_unburned",
            "sbs_low",
            "sbs_moderate",
            "sbs_high",
            "intersection_true",
            "intersection_false",
            "intersection_unknown",
        ] {
            counts.insert(key, 0);
        }
        // Validate the entire input grid before any products can be published.
        for row in 0..rows as isize {
            for col in 0..cols as isize {
                for r in &rasters {
                    let v = r.get_value(row, col);
                    if !missing(v, r.configs.nodata) && !v.is_finite() {
                        return Err(invalid("Nonfinite non-NoData input value"));
                    }
                }
                let v = sbs.get_value(row, col);
                if !missing(v, sbs.configs.nodata) && !codes.iter().any(|code| *code as f64 == v) {
                    return Err(invalid(format!(
                        "Unrecognized SBS class {} at {},{}",
                        v, row, col
                    )));
                }
            }
        }
        let threshold = 23f64.to_radians().tan();
        for row in 0..rows as isize {
            for col in 0..cols as isize {
                let mut z = [0.; 9];
                let mut valid =
                    row > 0 && col > 0 && row + 1 < rows as isize && col + 1 < cols as isize;
                if valid {
                    for dr in -1..=1 {
                        for dc in -1..=1 {
                            let v = dem.get_value(row + dr, col + dc);
                            z[((dr + 1) * 3 + dc + 1) as usize] = v;
                            valid &= !missing(v, c.nodata);
                        }
                    }
                }
                let gradient = if valid {
                    Some(horn(z, c.resolution_x)?)
                } else {
                    None
                };
                if let Some(g) = gradient {
                    outputs[0].set_value(row, col, g.atan().to_degrees());
                }
                let m = mask.get_value(row, col);
                if missing(m, mask.configs.nodata) || m <= 0. {
                    continue;
                }
                *counts.get_mut("basin").unwrap() += 1;
                let v = sbs.get_value(row, col);
                let class = if missing(v, sbs.configs.nodata) {
                    None
                } else {
                    codes.iter().position(|code| *code as f64 == v)
                };
                let steep = gradient.map(|g| g >= threshold);
                for (key, yes) in [
                    ("slope_valid", valid),
                    ("sbs_valid", class.is_some()),
                    ("jointly_valid", valid && class.is_some()),
                    ("steep", steep == Some(true)),
                ] {
                    if yes {
                        *counts.get_mut(key).unwrap() += 1;
                    }
                }
                if let Some(k) = class {
                    *counts
                        .get_mut(["sbs_unburned", "sbs_low", "sbs_moderate", "sbs_high"][k])
                        .unwrap() += 1;
                }
                let state = intersection(steep, class.map(|k| k >= 2));
                *counts
                    .get_mut(
                        [
                            "intersection_false",
                            "intersection_true",
                            "intersection_unknown",
                        ][state as usize],
                    )
                    .unwrap() += 1;
                outputs[1].set_value(row, col, state as f64);
                outputs[2].set_value(
                    row,
                    col,
                    (u8::from(valid) + 2 * u8::from(class.is_some())) as f64,
                );
            }
        }
        let n = counts["basin"];
        if n == 0 {
            return Err(invalid("Empty watershed"));
        }
        let area = c.resolution_x * c.resolution_y;
        if !(area * n as f64).is_finite() || area <= 0. {
            return Err(invalid("Invalid basin area"));
        }
        let y = counts["intersection_true"];
        let u = counts["intersection_unknown"];
        let lower = y as f64 / n as f64;
        let upper = (y + u) as f64 / n as f64;
        let areas: BTreeMap<_, _> = counts.iter().map(|(k, v)| (*k, *v as f64 * area)).collect();
        let summary = json!({"schema_version":1,"status":"complete","tool":"StaleySlopeSbs","tool_version":env!("CARGO_PKG_VERSION"),
            "parameters":{"algorithm":"Horn 3x3","dem_source":"raw","edges":"nine-valid-cells","threshold_degrees":23,"elevation_units":"m","sbs_classes":codes},
            "sources":sources,"grid":{"epsg":c.epsg_code,"rows":rows,"columns":cols,"resolution_m":c.resolution_x,"west":c.west,"north":c.north},
            "counts":counts,"areas_m2":areas,"T":if u==0 {Some(lower)} else {None},"T_lower":lower,"T_upper":upper});
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(&destination)?;
        // Failure leaves the exclusively reserved directory incomplete. Summary
        // is the last product and completion marker; never replace old outputs.
        for out in &mut outputs {
            whitebox_raster::geotiff::write_geotiff(out)?;
        }
        let bytes = serde_json::to_vec_pretty(&summary)?;
        let mut file = File::options()
            .write(true)
            .create_new(true)
            .open(destination.join("summary.json"))?;
        file.write_all(&bytes)?;
        file.flush()?;
        if verbose {
            println!(
                "StaleySlopeSbs complete: {} watershed cells, {} unresolved",
                n, u
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn planes_and_threshold() {
        for degrees in [0f64, 22.999999, 23., 23.000001, 45.] {
            for az in [0f64, 30., 45., 90., 135., 230.] {
                let g = degrees.to_radians().tan();
                let x = g * az.to_radians().cos();
                let y = g * az.to_radians().sin();
                let mut z = [0.; 9];
                for r in 0..3 {
                    for c in 0..3 {
                        z[r * 3 + c] = (c as f64 - 1.) * x * 10. + (r as f64 - 1.) * y * 10.;
                    }
                }
                let actual = horn(z, 10.).unwrap();
                assert!((actual.atan().to_degrees() - degrees).abs() < 1e-12);
                if degrees != 23. {
                    assert_eq!(actual >= 23f64.to_radians().tan(), degrees > 23.);
                }
            }
        }
        let t = 23f64.to_radians().tan();
        assert_eq!(horn([-t, 0., t, -t, 0., t, -t, 0., t], 1.).unwrap(), t);
    }
    #[test]
    fn full_truth_table() {
        for (a, b, want) in [
            (None, None, 2),
            (None, Some(false), 0),
            (None, Some(true), 2),
            (Some(false), None, 0),
            (Some(true), None, 2),
            (Some(false), Some(false), 0),
            (Some(false), Some(true), 0),
            (Some(true), Some(false), 0),
            (Some(true), Some(true), 1),
        ] {
            assert_eq!(intersection(a, b), want);
        }
    }
    #[test]
    fn ridge_pit_and_overflow() {
        assert_eq!(horn([0., 1., 0., 0., 1., 0., 0., 1., 0.], 1.).unwrap(), 0.);
        assert_eq!(
            horn([1., 1., 1., 1., -10., 1., 1., 1., 1.], 1.).unwrap(),
            0.
        );
        assert!(horn([-f64::MAX, 0., f64::MAX, 0., 0., 0., 0., 0., 0.], 1.).is_err());
    }
    #[test]
    fn bare_relative_paths_use_current_directory() {
        assert_eq!(path("result", ""), Path::new(".").join("result"));
        assert_eq!(path("result", "").parent(), Some(Path::new(".")));
    }
    #[test]
    fn parser_errors() {
        for a in [vec!["--dem"], vec!["--x=y"], vec!["--dem=x", "--dem=y"]] {
            assert!(parse(&a.iter().map(|v| v.to_string()).collect::<Vec<_>>()).is_err());
        }
    }
}
