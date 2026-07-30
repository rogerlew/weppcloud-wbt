use serde_json::{json, Value};
use std::fs::{self, File};
use std::io::{BufWriter, Error, ErrorKind, Write};
use std::path::{Path, PathBuf};
use whitebox_raster::Raster;

fn basename(path: &str) -> Result<String, Error> {
    Path::new(path)
        .file_name()
        .and_then(|value| value.to_str())
        .map(str::to_owned)
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "Invalid diagnostics raster basename."))
}

pub fn validate_operation_id(value: &str) -> Result<(), Error> {
    if value.len() == 32
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(Error::new(
            ErrorKind::InvalidInput,
            "--diagnostics_id must be exactly 32 lowercase hexadecimal characters.",
        ))
    }
}

pub fn write(
    diagnostics_file: &str,
    operation_id: &str,
    tool: &str,
    input_name: &str,
    output_name: &str,
    input: &Raster,
    output: &Raster,
    conditioning: Value,
    parameters: Value,
) -> Result<(), Error> {
    if diagnostics_file.is_empty() {
        return Ok(());
    }
    validate_operation_id(operation_id)?;

    let rows = input.configs.rows as isize;
    let columns = input.configs.columns as isize;
    let nodata = input.configs.nodata;
    let cell_area = input.configs.resolution_x.abs() * input.configs.resolution_y.abs();
    let mut valid_cell_count = 0usize;
    let mut raised_cell_count = 0usize;
    let mut lowered_cell_count = 0usize;
    let mut maximum_raise = 0f64;
    let mut maximum_cut = 0f64;
    let mut fill_sum = 0f64;
    let mut cut_sum = 0f64;
    for row in 0..rows {
        for column in 0..columns {
            let source = input.get_value(row, column);
            if source == nodata || !source.is_finite() {
                continue;
            }
            let conditioned = output.get_value(row, column);
            if !conditioned.is_finite() {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "Conditioned raster contains a non-finite value.",
                ));
            }
            valid_cell_count += 1;
            let delta = conditioned - source;
            if delta > 0.0 {
                raised_cell_count += 1;
                maximum_raise = maximum_raise.max(delta);
                fill_sum += delta;
            } else if delta < 0.0 {
                lowered_cell_count += 1;
                let cut = -delta;
                maximum_cut = maximum_cut.max(cut);
                cut_sum += cut;
            }
        }
    }

    let document = json!({
        "schema_version": 1,
        "tool": tool,
        "status": "success",
        "operation_id": operation_id,
        "input_name": basename(input_name)?,
        "output_name": basename(output_name)?,
        "units": {
            "elevation": "m",
            "horizontal": "m",
            "area": "m2",
            "volume": "m3"
        },
        "terrain_change": {
            "valid_cell_count": valid_cell_count,
            "raised_cell_count": raised_cell_count,
            "lowered_cell_count": lowered_cell_count,
            "raised_area": raised_cell_count as f64 * cell_area,
            "lowered_area": lowered_cell_count as f64 * cell_area,
            "maximum_raise": maximum_raise,
            "maximum_cut": maximum_cut,
            "fill_volume": fill_sum * cell_area,
            "cut_volume": cut_sum * cell_area
        },
        "conditioning": conditioning,
        "parameters": parameters
    });

    let target = PathBuf::from(diagnostics_file);
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let temp = parent.join(format!(".conditioning-diagnostics-{}.tmp", operation_id));
    let file = File::create(&temp)?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer_pretty(&mut writer, &document)
        .map_err(|error| Error::new(ErrorKind::Other, error.to_string()))?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    writer.get_ref().sync_all()?;
    drop(writer);
    if let Err(error) = fs::rename(&temp, &target) {
        let _ = fs::remove_file(&temp);
        return Err(error);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_operation_id;

    #[test]
    fn operation_id_requires_lowercase_hex() {
        assert!(validate_operation_id("0123456789abcdef0123456789abcdef").is_ok());
        assert!(validate_operation_id("0123456789ABCDEF0123456789ABCDEF").is_err());
        assert!(validate_operation_id("short").is_err());
    }
}
