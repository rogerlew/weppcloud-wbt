use crate::geotiff::read_geotiff_dimensions;
use crate::DataType;
use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;
use std::fs::File;
use std::io::{BufReader, Error, ErrorKind};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq)]
pub struct VrtDataset {
    pub raster_x_size: usize,
    pub raster_y_size: usize,
    pub srs: Option<String>,
    pub geo_transform: Option<[f64; 6]>,
    pub raster_band: VrtRasterBand,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VrtRasterBand {
    pub data_type: Option<DataType>,
    pub band: u32,
    pub simple_source: VrtSimpleSource,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VrtSimpleSource {
    pub source_filename: String,
    pub relative_to_vrt: bool,
    pub source_band: u32,
    pub src_rect: VrtRect,
    pub dst_rect: VrtRect,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub struct VrtRect {
    pub x_off: isize,
    pub y_off: isize,
    pub x_size: isize,
    pub y_size: isize,
}

impl VrtRect {
    fn new(x_off: isize, y_off: isize, x_size: isize, y_size: isize) -> Result<Self, Error> {
        if x_off < 0 || y_off < 0 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Rect offsets must be >= 0.",
            ));
        }
        if x_size <= 0 || y_size <= 0 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Rect sizes must be > 0.",
            ));
        }
        Ok(VrtRect {
            x_off,
            y_off,
            x_size,
            y_size,
        })
    }
}

impl VrtSimpleSource {
    pub fn resolve_source_path(&self, vrt_path: &Path) -> PathBuf {
        if self.relative_to_vrt {
            if let Some(parent) = vrt_path.parent() {
                return parent.join(&self.source_filename);
            }
        }
        PathBuf::from(&self.source_filename)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
enum TextTarget {
    SourceFilename,
    SourceBand,
    GeoTransform,
    Srs,
}

pub fn read_vrt<P: AsRef<Path>>(path: P) -> Result<VrtDataset, Error> {
    let path = path.as_ref();
    let file = File::open(path)?;
    let mut reader = Reader::from_reader(BufReader::new(file));
    reader.trim_text(true);

    let mut buf = Vec::new();

    let mut raster_x_size: Option<usize> = None;
    let mut raster_y_size: Option<usize> = None;
    let mut srs: Option<String> = None;
    let mut geo_transform: Option<[f64; 6]> = None;
    let mut geo_transform_text: Option<String> = None;

    let mut band: Option<u32> = None;
    let mut data_type: Option<DataType> = None;
    let mut simple_source_seen = 0usize;

    let mut source_filename: Option<String> = None;
    let mut relative_to_vrt = false;
    let mut source_band: Option<u32> = None;
    let mut src_rect: Option<VrtRect> = None;
    let mut dst_rect: Option<VrtRect> = None;

    let mut text_target: Option<TextTarget> = None;

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                handle_start(
                    &e,
                    false,
                    &mut raster_x_size,
                    &mut raster_y_size,
                    &mut srs,
                    &mut geo_transform_text,
                    &mut band,
                    &mut data_type,
                    &mut simple_source_seen,
                    &mut source_filename,
                    &mut relative_to_vrt,
                    &mut src_rect,
                    &mut dst_rect,
                    &mut text_target,
                )?;
            }
            Ok(Event::Empty(e)) => {
                handle_start(
                    &e,
                    true,
                    &mut raster_x_size,
                    &mut raster_y_size,
                    &mut srs,
                    &mut geo_transform_text,
                    &mut band,
                    &mut data_type,
                    &mut simple_source_seen,
                    &mut source_filename,
                    &mut relative_to_vrt,
                    &mut src_rect,
                    &mut dst_rect,
                    &mut text_target,
                )?;
            }
            Ok(Event::Text(e)) => {
                if let Some(target) = text_target {
                    let text = e
                        .unescape()
                        .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid XML text."))?
                        .trim()
                        .to_string();
                    if text.is_empty() {
                        continue;
                    }
                    match target {
                        TextTarget::SourceFilename => {
                            source_filename = Some(text);
                        }
                        TextTarget::SourceBand => {
                            source_band = Some(parse_u32(&text, "SourceBand")?);
                        }
                        TextTarget::GeoTransform => {
                            let entry = geo_transform_text.get_or_insert_with(String::new);
                            entry.push_str(&text);
                        }
                        TextTarget::Srs => {
                            let entry = srs.get_or_insert_with(String::new);
                            entry.push_str(&text);
                        }
                    }
                }
            }
            Ok(Event::End(e)) => {
                let qname = e.name();
                let name = qname.as_ref();
                if let Some(target) = text_target {
                    let target_name: &[u8] = match target {
                        TextTarget::SourceFilename => b"SourceFilename",
                        TextTarget::SourceBand => b"SourceBand",
                        TextTarget::GeoTransform => b"GeoTransform",
                        TextTarget::Srs => b"SRS",
                    };
                    if name == target_name {
                        text_target = None;
                    }
                }
                if name == b"GeoTransform" {
                    if let Some(text) = geo_transform_text.take() {
                        geo_transform = Some(parse_geotransform(&text)?);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Err(_) => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Failed to parse VRT XML.",
                ))
            }
            _ => {}
        }
        buf.clear();
    }

    let raster_x_size = raster_x_size.ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidData,
            "VRTDataset rasterXSize is missing.",
        )
    })?;
    let raster_y_size = raster_y_size.ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidData,
            "VRTDataset rasterYSize is missing.",
        )
    })?;

    if simple_source_seen == 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "VRT SimpleSource element is missing.",
        ));
    }

    let source_filename = source_filename.ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidData,
            "VRT SourceFilename is missing.",
        )
    })?;

    let source_band = source_band.unwrap_or(1);
    if source_band != 1 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "Only SourceBand=1 is supported.",
        ));
    }

    let mut rects_missing = false;
    let (src_rect, dst_rect) = match (src_rect, dst_rect) {
        (Some(src), Some(dst)) => (src, dst),
        (None, None) => {
            rects_missing = true;
            let x_size = isize::try_from(raster_x_size).map_err(|_| {
                Error::new(
                    ErrorKind::InvalidData,
                    "VRT rasterXSize is too large.",
                )
            })?;
            let y_size = isize::try_from(raster_y_size).map_err(|_| {
                Error::new(
                    ErrorKind::InvalidData,
                    "VRT rasterYSize is too large.",
                )
            })?;
            let full_rect = VrtRect::new(0, 0, x_size, y_size)?;
            (full_rect, full_rect)
        }
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "VRT SrcRect and DstRect must both be present or both omitted.",
            ))
        }
    };

    if dst_rect.x_off != 0 || dst_rect.y_off != 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "DstRect offsets must be 0.",
        ));
    }
    if src_rect.x_size != dst_rect.x_size || src_rect.y_size != dst_rect.y_size {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "SrcRect and DstRect sizes must match.",
        ));
    }
    if raster_x_size != src_rect.x_size as usize
        || raster_y_size != src_rect.y_size as usize
    {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "VRT raster size must match SrcRect size.",
        ));
    }

    let band = band.unwrap_or(1);
    if band != 1 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "Only band=1 is supported.",
        ));
    }

    let simple_source = VrtSimpleSource {
        source_filename,
        relative_to_vrt,
        source_band,
        src_rect,
        dst_rect,
    };

    let source_path = simple_source.resolve_source_path(path);
    let (source_columns, source_rows) = read_geotiff_dimensions(
        &source_path.to_string_lossy().to_string(),
    )?;
    let x_off_u = usize::try_from(simple_source.src_rect.x_off).map_err(|_| {
        Error::new(ErrorKind::InvalidData, "Invalid SrcRect offset.")
    })?;
    let y_off_u = usize::try_from(simple_source.src_rect.y_off).map_err(|_| {
        Error::new(ErrorKind::InvalidData, "Invalid SrcRect offset.")
    })?;
    let x_size_u = usize::try_from(simple_source.src_rect.x_size).map_err(|_| {
        Error::new(ErrorKind::InvalidData, "Invalid SrcRect size.")
    })?;
    let y_size_u = usize::try_from(simple_source.src_rect.y_size).map_err(|_| {
        Error::new(ErrorKind::InvalidData, "Invalid SrcRect size.")
    })?;
    let x_end = x_off_u
        .checked_add(x_size_u)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "SrcRect exceeds source bounds."))?;
    let y_end = y_off_u
        .checked_add(y_size_u)
        .ok_or_else(|| Error::new(ErrorKind::InvalidData, "SrcRect exceeds source bounds."))?;
    if x_end > source_columns || y_end > source_rows {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "SrcRect exceeds source bounds.",
        ));
    }
    if rects_missing && (raster_x_size != source_columns || raster_y_size != source_rows) {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "VRT without SrcRect/DstRect must match source dimensions.",
        ));
    }

    Ok(VrtDataset {
        raster_x_size,
        raster_y_size,
        srs,
        geo_transform,
        raster_band: VrtRasterBand {
            data_type,
            band,
            simple_source,
        },
    })
}

fn handle_start(
    e: &BytesStart<'_>,
    is_empty: bool,
    raster_x_size: &mut Option<usize>,
    raster_y_size: &mut Option<usize>,
    srs: &mut Option<String>,
    geo_transform_text: &mut Option<String>,
    band: &mut Option<u32>,
    data_type: &mut Option<DataType>,
    simple_source_seen: &mut usize,
    source_filename: &mut Option<String>,
    relative_to_vrt: &mut bool,
    src_rect: &mut Option<VrtRect>,
    dst_rect: &mut Option<VrtRect>,
    text_target: &mut Option<TextTarget>,
) -> Result<(), Error> {
    match e.name().as_ref() {
        b"VRTDataset" => {
            *raster_x_size = Some(parse_usize_attr(e, b"rasterXSize")?);
            *raster_y_size = Some(parse_usize_attr(e, b"rasterYSize")?);
        }
        b"VRTRasterBand" => {
            let band_value = parse_optional_u32_attr(e, b"band")?.unwrap_or(1);
            if band_value != 1 {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Only VRTRasterBand band=1 is supported.",
                ));
            }
            if band.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Only a single VRTRasterBand is supported.",
                ));
            }
            *band = Some(band_value);
            if let Some(value) = parse_optional_string_attr(e, b"dataType")? {
                *data_type = Some(parse_data_type(&value)?);
            }
        }
        b"SimpleSource" => {
            if parse_optional_string_attr(e, b"resampling")?.is_some() {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "SimpleSource resampling is not supported.",
                ));
            }
            *simple_source_seen += 1;
            if *simple_source_seen > 1 {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Only one SimpleSource is supported.",
                ));
            }
        }
        b"ComplexSource"
        | b"AveragedSource"
        | b"NoDataFromMaskSource"
        | b"KernelFilteredSource"
        | b"ArraySource" => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Unsupported VRT source type.",
            ));
        }
        b"MaskBand" => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "MaskBand is not supported.",
            ));
        }
        b"SourceFilename" => {
            *relative_to_vrt = match parse_optional_string_attr(e, b"relativeToVRT")? {
                Some(value) => parse_bool_attr(&value, "relativeToVRT")?,
                None => false,
            };
            if !is_empty {
                *text_target = Some(TextTarget::SourceFilename);
            } else {
                *source_filename = Some(String::new());
            }
        }
        b"SourceBand" => {
            if !is_empty {
                *text_target = Some(TextTarget::SourceBand);
            }
        }
        b"SrcRect" => {
            *src_rect = Some(parse_rect(e, "SrcRect")?);
        }
        b"DstRect" => {
            *dst_rect = Some(parse_rect(e, "DstRect")?);
        }
        b"SRS" => {
            if !is_empty {
                *text_target = Some(TextTarget::Srs);
            } else {
                *srs = Some(String::new());
            }
        }
        b"GeoTransform" => {
            if !is_empty {
                *text_target = Some(TextTarget::GeoTransform);
            } else {
                *geo_transform_text = Some(String::new());
            }
        }
        _ => {}
    }

    Ok(())
}

fn parse_rect(e: &BytesStart<'_>, name: &str) -> Result<VrtRect, Error> {
    let x_off = parse_isize_attr(e, b"xOff", name)?;
    let y_off = parse_isize_attr(e, b"yOff", name)?;
    let x_size = parse_isize_attr(e, b"xSize", name)?;
    let y_size = parse_isize_attr(e, b"ySize", name)?;
    VrtRect::new(x_off, y_off, x_size, y_size)
}

fn parse_geotransform(text: &str) -> Result<[f64; 6], Error> {
    let parts: Vec<_> = text
        .split(',')
        .map(|part| part.trim())
        .filter(|part| !part.is_empty())
        .collect();
    if parts.len() != 6 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            "GeoTransform must contain six values.",
        ));
    }
    let mut values = [0.0f64; 6];
    for (idx, part) in parts.iter().enumerate() {
        values[idx] = part
            .parse::<f64>()
            .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid GeoTransform value."))?;
    }
    Ok(values)
}

fn parse_data_type(value: &str) -> Result<DataType, Error> {
    let normalized = value.trim().to_ascii_lowercase();
    let data_type = match normalized.as_str() {
        "byte" | "uint8" => DataType::U8,
        "int8" => DataType::I8,
        "uint16" => DataType::U16,
        "int16" => DataType::I16,
        "uint32" => DataType::U32,
        "int32" => DataType::I32,
        "uint64" => DataType::U64,
        "int64" => DataType::I64,
        "float32" => DataType::F32,
        "float64" => DataType::F64,
        _ => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "Unsupported VRT dataType.",
            ))
        }
    };
    Ok(data_type)
}

fn parse_usize_attr(e: &BytesStart<'_>, key: &[u8]) -> Result<usize, Error> {
    let value = parse_string_attr(e, key)?;
    value
        .parse::<usize>()
        .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid integer attribute."))
}

fn parse_isize_attr(e: &BytesStart<'_>, key: &[u8], context: &str) -> Result<isize, Error> {
    let value = parse_string_attr(e, key)?;
    let parsed = value
        .parse::<isize>()
        .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid rect attribute."))?;
    if parsed < 0 {
        return Err(Error::new(
            ErrorKind::InvalidData,
            format!("{} contains a negative value.", context),
        ));
    }
    Ok(parsed)
}

fn parse_u32(value: &str, context: &str) -> Result<u32, Error> {
    value
        .parse::<u32>()
        .map_err(|_| Error::new(ErrorKind::InvalidData, format!("Invalid {} value.", context)))
}

fn parse_optional_u32_attr(e: &BytesStart<'_>, key: &[u8]) -> Result<Option<u32>, Error> {
    match parse_optional_string_attr(e, key)? {
        Some(value) => Ok(Some(parse_u32(&value, "attribute")?)),
        None => Ok(None),
    }
}

fn parse_string_attr(e: &BytesStart<'_>, key: &[u8]) -> Result<String, Error> {
    parse_optional_string_attr(e, key)?.ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidData,
            "Missing required XML attribute.",
        )
    })
}

fn parse_optional_string_attr(
    e: &BytesStart<'_>,
    key: &[u8],
) -> Result<Option<String>, Error> {
    for attr in e.attributes().with_checks(false) {
        let attr = attr.map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid XML attribute."))?;
        if attr.key.as_ref() == key {
            let value = attr
                .unescape_value()
                .map_err(|_| Error::new(ErrorKind::InvalidData, "Invalid XML attribute."))?
                .to_string();
            return Ok(Some(value));
        }
    }
    Ok(None)
}

fn parse_bool_attr(value: &str, context: &str) -> Result<bool, Error> {
    match value.trim() {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Invalid {} value.", context),
        )),
    }
}
