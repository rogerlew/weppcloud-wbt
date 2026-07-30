/*
This tool is part of the WhiteboxTools geospatial analysis library.

The conditioning kernel is a source-level translation of the FILDEP and RELIEF
numerical methods in USDA-ARS TOPAZ DEDNM 3.10, maintained at
https://github.com/rogerlew/topaz (revision recorded in repository docs).
J. Garbrecht and L. Martz are the authors of the original TOPAZ methods.
*/

use crate::tools::*;
use serde_json::json;
use std::env;
use std::fs::File;
use std::io::{BufWriter, Error, ErrorKind, Write};
use std::path;
use std::time::Instant;
use whitebox_raster::*;

const SCALE: f64 = 100_000.0;
const QUANTUM: i64 = 10_000; // TOPAZ first rounds source elevations to 0.1 z unit.
const SOURCE_REVISION: &str = "topaz@116607fc1185800ca78e387454ef1ccd3ffd73b4";

#[derive(Default, Clone, Debug)]
struct ConditioningStats {
    depressions: usize,
    flats: usize,
    obstruction_width_1: usize,
    obstruction_width_2: usize,
    filled_cells: usize,
    lowered_cells: usize,
    relief_cells: usize,
    max_fill: i64,
    max_cut: i64,
    max_relief: i64,
    fill_sum: i128,
    cut_sum: i128,
}

#[derive(Clone)]
struct Grid {
    rows: usize,
    cols: usize,
    z: Vec<i64>,
    valid: Vec<bool>,
}

impl Grid {
    fn idx(&self, row: usize, col: usize) -> usize {
        row * self.cols + col
    }

    fn neighbours(&self, row: usize, col: usize) -> Vec<(usize, usize)> {
        let mut cells = Vec::with_capacity(9);
        for dr in -1isize..=1 {
            for dc in -1isize..=1 {
                let rr = row as isize + dr;
                let cc = col as isize + dc;
                if rr >= 0 && cc >= 0 && rr < self.rows as isize && cc < self.cols as isize {
                    cells.push((rr as usize, cc as usize));
                }
            }
        }
        cells
    }
}

fn topaz_round(value: f64) -> Result<i64, Error> {
    if !value.is_finite() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Valid DEM cells must contain finite elevations.",
        ));
    }
    // DEDNM reads the decimal elevation into its default REAL (32-bit) before
    // multiplying and applying NINT. The f32 operation is observable for DEM
    // values immediately below a half-decimetre boundary.
    let decimetres = ((value as f32) * 10.0_f32).round() as f64;
    if decimetres < (i64::MIN / QUANTUM) as f64 || decimetres > (i64::MAX / QUANTUM) as f64 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "DEM elevation exceeds the TOPAZ integer scale.",
        ));
    }
    Ok(decimetres as i64 * QUANTUM)
}

#[derive(Clone, Copy)]
struct SearchWindow {
    top: usize,
    bottom: usize,
    left: usize,
    right: usize,
}

fn choose_obstruction(
    grid: &Grid,
    state: &[i16],
    window: SearchWindow,
    spill: i64,
    max_width: u8,
) -> Option<(i64, usize, Option<usize>)> {
    if max_width == 0 {
        return None;
    }
    let mut best: Option<(i64, f32, i64, usize, Option<usize>)> = None;
    for row in window.top..=window.bottom {
        for col in window.left..=window.right {
            let index = grid.idx(row, col);
            if state[index] < 99 || grid.z[index] != spill {
                continue;
            }
            let mut outside_drop = 0i64;
            let mut outside_dist = f32::INFINITY;
            for (rr, cc) in grid.neighbours(row, col) {
                let n = grid.idx(rr, cc);
                if !grid.valid[n] || state[n] >= 99 {
                    continue;
                }
                let drop = spill - grid.z[n];
                let dist = (((rr as isize - row as isize).pow(2)
                    + (cc as isize - col as isize).pow(2)) as f32)
                    .sqrt();
                if drop > outside_drop || (drop == outside_drop && dist < outside_dist) {
                    outside_drop = drop;
                    outside_dist = dist;
                }
            }
            if outside_drop <= 0 {
                continue;
            }

            let mut inside_drop = 0i64;
            let mut inside_dist = f32::INFINITY;
            let mut inside_cell = index;
            for (rr, cc) in grid.neighbours(row, col) {
                let n = grid.idx(rr, cc);
                if !grid.valid[n] || state[n] < 99 {
                    continue;
                }
                let drop = spill - grid.z[n];
                let d1 = (((rr as isize - row as isize).pow(2)
                    + (cc as isize - col as isize).pow(2)) as f32)
                    .sqrt();
                if drop >= 0 && (drop > inside_drop || (drop == inside_drop && d1 < inside_dist)) {
                    inside_drop = drop;
                    inside_dist = d1;
                    inside_cell = n;
                }
                if max_width < 2 || drop < 0 {
                    continue;
                }
                for (r2, c2) in grid.neighbours(rr, cc) {
                    let n2 = grid.idx(r2, c2);
                    if !grid.valid[n2] || state[n2] < 99 {
                        continue;
                    }
                    let drop2 = spill - grid.z[n2];
                    let d2 =
                        ((((rr as isize - row as isize).pow(2)
                            + (cc as isize - col as isize).pow(2))
                            as f64)
                            .sqrt()
                            + (((r2 as isize - rr as isize).pow(2)
                                + (c2 as isize - cc as isize).pow(2))
                                as f64)
                                .sqrt()) as f32;
                    if drop2 >= 0
                        && (drop2 > inside_drop || (drop2 == inside_drop && d2 < inside_dist))
                    {
                        inside_drop = drop2;
                        inside_dist = d2;
                        inside_cell = n;
                    }
                }
            }
            let cut = outside_drop.min(inside_drop);
            if cut <= 0 {
                continue;
            }
            let distance = outside_dist + inside_dist;
            let candidate = (
                cut,
                distance,
                outside_drop,
                index,
                if max_width == 2 {
                    Some(inside_cell)
                } else {
                    None
                },
            );
            let replace = match best {
                None => true,
                Some((best_cut, best_distance, _, _, _)) => {
                    cut > best_cut
                        || (cut == best_cut && distance < best_distance - 1.0e-5_f32)
                        || (cut == best_cut
                            && (distance - best_distance).abs() < 1.0e-5_f32
                            && outside_drop > best_cut)
                }
            };
            if replace {
                best = Some(candidate);
            }
        }
    }
    best.map(|(cut, _, _, outlet, inner)| (cut, outlet, inner))
}

fn fill_depressions(
    grid: &mut Grid,
    max_width: u8,
    stats: &mut ConditioningStats,
) -> Result<(), Error> {
    let mut state = vec![0i16; grid.z.len()];
    let mut visited = vec![false; grid.z.len()];
    if grid.rows < 3 || grid.cols < 3 {
        return Ok(());
    }
    for row in 1..grid.rows - 1 {
        for col in 1..grid.cols - 1 {
            let seed = grid.idx(row, col);
            if state[seed] > 0 || !grid.valid[seed] {
                continue;
            }
            let mut has_lower = false;
            let mut has_higher = false;
            for (rr, cc) in grid.neighbours(row, col) {
                let n = grid.idx(rr, cc);
                if !grid.valid[n] {
                    has_lower = true;
                    continue;
                }
                has_lower |= grid.z[n] < grid.z[seed];
                has_higher |= grid.z[n] > grid.z[seed];
            }
            if has_lower || !has_higher {
                continue;
            }

            state[seed] += 99;
            let mut window = SearchWindow {
                top: row.saturating_sub(5),
                bottom: (row + 5).min(grid.rows - 1),
                left: col.saturating_sub(5),
                right: (col + 5).min(grid.cols - 1),
            };
            let mut widened_for_zero_cut = false;
            let mut final_spill: i64;

            loop {
                loop {
                    let mut changed = false;
                    for r in window.top + 1..window.bottom {
                        for c in window.left + 1..window.right {
                            let index = grid.idx(r, c);
                            if state[index] < 99 || visited[index] {
                                continue;
                            }
                            for (rr, cc) in grid.neighbours(r, c) {
                                let n = grid.idx(rr, cc);
                                if grid.valid[n] && state[n] < 99 && grid.z[n] >= grid.z[index] {
                                    state[n] += 99;
                                    changed = true;
                                }
                            }
                            visited[index] = true;
                        }
                    }
                    if !changed {
                        break;
                    }
                }

                let mut minimum = i64::MAX;
                let mut spill = i64::MAX;
                let mut has_outlet = false;
                for r in window.top..=window.bottom {
                    for c in window.left..=window.right {
                        let index = grid.idx(r, c);
                        if state[index] < 99 {
                            continue;
                        }
                        minimum = minimum.min(grid.z[index]);
                        if grid.z[index] >= spill {
                            continue;
                        }
                        if r == 0 || c == 0 || r + 1 == grid.rows || c + 1 == grid.cols {
                            spill = grid.z[index];
                            has_outlet = true;
                            continue;
                        }
                        if grid.neighbours(r, c).iter().any(|&(rr, cc)| {
                            let n = grid.idx(rr, cc);
                            !grid.valid[n] || (state[n] < 99 && grid.z[n] < grid.z[index])
                        }) {
                            spill = grid.z[index];
                            has_outlet = true;
                        }
                    }
                }

                if !has_outlet {
                    let mut boundary_min = i64::MAX;
                    for r in window.top..=window.bottom {
                        boundary_min = boundary_min.min(grid.z[grid.idx(r, window.left)]);
                        boundary_min = boundary_min.min(grid.z[grid.idx(r, window.right)]);
                    }
                    for c in window.left..=window.right {
                        boundary_min = boundary_min.min(grid.z[grid.idx(window.top, c)]);
                        boundary_min = boundary_min.min(grid.z[grid.idx(window.bottom, c)]);
                    }
                    let mut expanded = false;
                    if window.left > 0
                        && (window.top..=window.bottom)
                            .any(|r| grid.z[grid.idx(r, window.left)] == boundary_min)
                    {
                        window.left = window.left.saturating_sub(5);
                        expanded = true;
                    }
                    if window.right + 1 < grid.cols
                        && (window.top..=window.bottom)
                            .any(|r| grid.z[grid.idx(r, window.right)] == boundary_min)
                    {
                        window.right = (window.right + 5).min(grid.cols - 1);
                        expanded = true;
                    }
                    if window.top > 0
                        && (window.left..=window.right)
                            .any(|c| grid.z[grid.idx(window.top, c)] == boundary_min)
                    {
                        window.top = window.top.saturating_sub(5);
                        expanded = true;
                    }
                    if window.bottom + 1 < grid.rows
                        && (window.left..=window.right)
                            .any(|c| grid.z[grid.idx(window.bottom, c)] == boundary_min)
                    {
                        window.bottom = (window.bottom + 5).min(grid.rows - 1);
                        expanded = true;
                    }
                    if expanded {
                        continue;
                    }
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "TOPAZ depression search found no outlet and could not expand its window.",
                    ));
                }

                let mut expanded = false;
                if window.left > 0
                    && (window.top..=window.bottom).any(|r| {
                        let i = grid.idx(r, window.left);
                        state[i] >= 99 && grid.z[i] < spill
                    })
                {
                    window.left = window.left.saturating_sub(5);
                    expanded = true;
                }
                if window.right + 1 < grid.cols
                    && (window.top..=window.bottom).any(|r| {
                        let i = grid.idx(r, window.right);
                        state[i] >= 99 && grid.z[i] < spill
                    })
                {
                    window.right = (window.right + 5).min(grid.cols - 1);
                    expanded = true;
                }
                if window.top > 0
                    && (window.left..=window.right).any(|c| {
                        let i = grid.idx(window.top, c);
                        state[i] >= 99 && grid.z[i] < spill
                    })
                {
                    window.top = window.top.saturating_sub(5);
                    expanded = true;
                }
                if window.bottom + 1 < grid.rows
                    && (window.left..=window.right).any(|c| {
                        let i = grid.idx(window.bottom, c);
                        state[i] >= 99 && grid.z[i] < spill
                    })
                {
                    window.bottom = (window.bottom + 5).min(grid.rows - 1);
                    expanded = true;
                }
                if expanded {
                    continue;
                }

                final_spill = spill;
                if max_width == 0 || minimum == spill {
                    break;
                }
                if let Some((cut, outlet, inside)) =
                    choose_obstruction(grid, &state, window, spill, max_width)
                {
                    final_spill -= cut;
                    grid.z[outlet] = final_spill;
                    if let Some(inner) = inside {
                        grid.z[inner] = final_spill;
                        stats.obstruction_width_2 += 1;
                    } else {
                        stats.obstruction_width_1 += 1;
                    }
                    break;
                }
                if max_width == 2 && !widened_for_zero_cut {
                    window.top = window.top.saturating_sub(1);
                    window.bottom = (window.bottom + 1).min(grid.rows - 1);
                    window.left = window.left.saturating_sub(1);
                    window.right = (window.right + 1).min(grid.cols - 1);
                    widened_for_zero_cut = true;
                    continue;
                }
                break;
            }

            let mut changed = false;
            for r in window.top..=window.bottom {
                for c in window.left..=window.right {
                    let index = grid.idx(r, c);
                    if state[index] < 99 {
                        continue;
                    }
                    visited[index] = false;
                    if grid.z[index] > final_spill {
                        state[index] -= 99;
                    } else {
                        changed |= grid.z[index] != final_spill;
                        grid.z[index] = final_spill;
                        state[index] = 1;
                    }
                }
            }
            if changed {
                stats.depressions += 1;
            }
        }
    }
    Ok(())
}

fn resolve_flats(grid: &mut Grid, stats: &mut ConditioningStats) {
    if grid.rows < 3 || grid.cols < 3 {
        return;
    }
    let count = grid.z.len();
    let mut status = vec![0i32; count];
    let mut relief = vec![0i64; count];
    let mut work = vec![0i64; count];

    for row in 1..grid.rows - 1 {
        for col in 1..grid.cols - 1 {
            let index = grid.idx(row, col);
            if !grid.valid[index] {
                status[index] = 2;
                continue;
            }
            status[index] = if grid.neighbours(row, col).iter().any(|&(rr, cc)| {
                let n = grid.idx(rr, cc);
                !grid.valid[n] || grid.z[n] < grid.z[index]
            }) {
                1
            } else {
                5
            };
        }
    }

    for row in 1..grid.rows - 1 {
        for col in 1..grid.cols - 1 {
            let seed = grid.idx(row, col);
            if status[seed] <= 3 {
                continue;
            }
            let elevation = grid.z[seed];
            let mut flat = vec![seed];
            status[seed] = 4;
            let mut cursor = 0;
            while cursor < flat.len() {
                let index = flat[cursor];
                cursor += 1;
                let r = index / grid.cols;
                let c = index % grid.cols;
                for (rr, cc) in grid.neighbours(r, c) {
                    let n = grid.idx(rr, cc);
                    if status[n] > 4 && grid.valid[n] && grid.z[n] == elevation {
                        status[n] = 4;
                        flat.push(n);
                    }
                }
                if status[index] > 3 {
                    status[index] = 3;
                }
            }
            stats.flats += 1;

            let mut layer = -1i64;
            for &index in &flat {
                let r = index / grid.cols;
                let c = index % grid.cols;
                if grid.neighbours(r, c).iter().any(|&(rr, cc)| {
                    let n = grid.idx(rr, cc);
                    grid.valid[n] && grid.z[n] > grid.z[index]
                }) {
                    relief[index] = layer;
                }
            }
            loop {
                let next = layer - 1;
                let mut changed = false;
                for &index in &flat {
                    if relief[index] != layer {
                        continue;
                    }
                    let r = index / grid.cols;
                    let c = index % grid.cols;
                    for (rr, cc) in grid.neighbours(r, c) {
                        let n = grid.idx(rr, cc);
                        if status[n] == 3 && grid.z[n] == elevation && relief[n] >= 0 {
                            relief[n] = next;
                            changed = true;
                        }
                    }
                }
                if !changed {
                    break;
                }
                layer = next;
            }
            let offset = layer.abs();
            for &index in &flat {
                if relief[index] < 0 {
                    relief[index] = (relief[index] + offset) * 2;
                }
            }
        }
    }

    for pass in 0..2 {
        let increment = if pass == 0 { 2 } else { 1 };
        work.copy_from_slice(&grid.z);
        loop {
            let mut changed = false;
            for row in 1..grid.rows - 1 {
                for col in 1..grid.cols - 1 {
                    let index = grid.idx(row, col);
                    if !grid.valid[index]
                        || (pass == 0 && status[index] <= 2)
                        || (pass == 1 && (status[index] == 1 || status[index] == 2))
                    {
                        work[index] = grid.z[index];
                        continue;
                    }
                    let has_lower = grid.neighbours(row, col).iter().any(|&(rr, cc)| {
                        let n = grid.idx(rr, cc);
                        !grid.valid[n] || grid.z[n] < grid.z[index]
                    });
                    if has_lower {
                        work[index] = grid.z[index];
                        status[index] = -1;
                    } else {
                        work[index] = grid.z[index] + increment;
                        changed = true;
                    }
                }
            }
            if !changed {
                grid.z.copy_from_slice(&work);
                break;
            }
            let mut next_changed = false;
            for row in 1..grid.rows - 1 {
                for col in 1..grid.cols - 1 {
                    let index = grid.idx(row, col);
                    if !grid.valid[index]
                        || (pass == 0 && status[index] <= 2)
                        || (pass == 1 && (status[index] == 1 || status[index] == 2))
                    {
                        work[index] = grid.z[index];
                        continue;
                    }
                    let has_lower = grid.neighbours(row, col).iter().any(|&(rr, cc)| {
                        let n = grid.idx(rr, cc);
                        !grid.valid[n] || work[n] < work[index]
                    });
                    if has_lower {
                        grid.z[index] = work[index];
                        status[index] = -1;
                    } else {
                        grid.z[index] = work[index] + increment;
                        next_changed = true;
                    }
                }
            }
            if !next_changed {
                break;
            }
        }
        if pass == 0 {
            for index in 0..count {
                if grid.valid[index] {
                    grid.z[index] += relief[index];
                }
            }
        }
    }
}

fn condition_topaz(
    input: &Grid,
    max_width: u8,
) -> Result<(Grid, Vec<i64>, Vec<i64>, ConditioningStats), Error> {
    let original = input.z.clone();
    let mut output = input.clone();
    let mut stats = ConditioningStats::default();
    fill_depressions(&mut output, max_width, &mut stats)?;
    let fildep = output.z.clone();
    resolve_flats(&mut output, &mut stats);

    for index in 0..output.z.len() {
        if !output.valid[index] {
            continue;
        }
        let fill_delta = fildep[index] - original[index];
        if fill_delta > 0 {
            stats.filled_cells += 1;
            stats.max_fill = stats.max_fill.max(fill_delta);
            stats.fill_sum += fill_delta as i128;
        } else if fill_delta < 0 {
            stats.lowered_cells += 1;
            stats.max_cut = stats.max_cut.max(-fill_delta);
            stats.cut_sum += (-fill_delta) as i128;
        }
        let relief_delta = output.z[index] - fildep[index];
        if relief_delta != 0 {
            stats.relief_cells += 1;
            stats.max_relief = stats.max_relief.max(relief_delta.abs());
        }
    }
    Ok((output, fildep, original, stats))
}

pub struct TopazConditionDem {
    name: String,
    description: String,
    toolbox: String,
    parameters: Vec<ToolParameter>,
    example_usage: String,
}

impl TopazConditionDem {
    pub fn new() -> TopazConditionDem {
        let name = "TopazConditionDem".to_string();
        let toolbox = "Hydrological Analysis".to_string();
        let description =
            "Conditions a DEM using TOPAZ-compatible FILDEP and RELIEF methods.".to_string();
        let parameters = vec![
            ToolParameter {
                name: "Input DEM File".to_owned(),
                flags: vec!["-i".to_owned(), "--dem".to_owned()],
                description: "Input raster DEM file.".to_owned(),
                parameter_type: ParameterType::ExistingFile(ParameterFileType::Raster),
                default_value: None,
                optional: false,
            },
            ToolParameter {
                name: "Output File".to_owned(),
                flags: vec!["-o".to_owned(), "--output".to_owned()],
                description: "Output conditioned DEM.".to_owned(),
                parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
                default_value: None,
                optional: false,
            },
            ToolParameter {
                name: "Maximum Obstruction Width".to_owned(),
                flags: vec!["--max_obstruction_width".to_owned()],
                description: "TOPAZ obstruction adjustment width: 0, 1, or 2 cells.".to_owned(),
                parameter_type: ParameterType::Integer,
                default_value: Some("2".to_string()),
                optional: true,
            },
            ToolParameter {
                name: "FILDEP Stage Raster".to_owned(),
                flags: vec!["--fildep".to_owned()],
                description: "Optional post-FILDEP, pre-RELIEF stage raster.".to_owned(),
                parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
                default_value: None,
                optional: true,
            },
            ToolParameter {
                name: "Delta Raster".to_owned(),
                flags: vec!["--delta".to_owned()],
                description: "Optional signed conditioned-minus-input raster.".to_owned(),
                parameter_type: ParameterType::NewFile(ParameterFileType::Raster),
                default_value: None,
                optional: true,
            },
            ToolParameter {
                name: "Diagnostics JSON".to_owned(),
                flags: vec!["--diagnostics".to_owned()],
                description: "Optional JSON diagnostics output.".to_owned(),
                parameter_type: ParameterType::NewFile(ParameterFileType::Text),
                default_value: None,
                optional: true,
            },
        ];
        let sep = path::MAIN_SEPARATOR.to_string();
        let mut exe = env::current_exe().unwrap();
        exe.pop();
        let usage = format!(
            ">>whitebox_tools -r={} --wd=\"*path*to*data*\" --dem=dem.tif -o=conditioned.tif",
            name
        )
        .replace("*", &sep);
        TopazConditionDem {
            name,
            description,
            toolbox,
            parameters,
            example_usage: usage,
        }
    }
}

impl WhiteboxTool for TopazConditionDem {
    fn get_source_file(&self) -> String {
        String::from(file!())
    }
    fn get_tool_name(&self) -> String {
        self.name.clone()
    }
    fn get_tool_description(&self) -> String {
        self.description.clone()
    }
    fn get_tool_parameters(&self) -> String {
        match serde_json::to_string(&self.parameters) {
            Ok(json) => format!("{{\"parameters\":{}}}", json),
            Err(error) => format!("{:?}", error),
        }
    }
    fn get_example_usage(&self) -> String {
        self.example_usage.clone()
    }
    fn get_toolbox(&self) -> String {
        self.toolbox.clone()
    }

    fn run<'a>(
        &self,
        args: Vec<String>,
        working_directory: &'a str,
        verbose: bool,
    ) -> Result<(), Error> {
        if args.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Tool run with no parameters.",
            ));
        }
        let mut dem = String::new();
        let mut output_file = String::new();
        let mut fildep_file = String::new();
        let mut delta_file = String::new();
        let mut diagnostics_file = String::new();
        let mut max_width = 2u8;
        let mut i = 0;
        while i < args.len() {
            let arg = args[i].replace(['"', '\''], "");
            let parts: Vec<&str> = arg.splitn(2, '=').collect();
            let flag = parts[0].to_lowercase().replace("--", "-");
            let value = if parts.len() == 2 {
                parts[1].to_string()
            } else if i + 1 < args.len() {
                i += 1;
                args[i].replace(['"', '\''], "")
            } else {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!("Missing value for {}", flag),
                ));
            };
            match flag.as_str() {
                "-i" | "-input" | "-dem" => dem = value,
                "-o" | "-output" => output_file = value,
                "-fildep" => fildep_file = value,
                "-max_obstruction_width" => {
                    max_width = value.parse::<u8>().map_err(|_| {
                        Error::new(ErrorKind::InvalidInput, "Invalid obstruction width.")
                    })?
                }
                "-delta" => delta_file = value,
                "-diagnostics" => diagnostics_file = value,
                _ => {}
            }
            i += 1;
        }
        if dem.is_empty() || output_file.is_empty() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "--dem and --output are required.",
            ));
        }
        if max_width > 2 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "--max_obstruction_width must be 0, 1, or 2.",
            ));
        }
        let sep = path::MAIN_SEPARATOR.to_string();
        for filename in [
            &mut dem,
            &mut output_file,
            &mut fildep_file,
            &mut delta_file,
            &mut diagnostics_file,
        ] {
            if !filename.is_empty() && !filename.contains(&sep) && !filename.contains('/') {
                *filename = format!("{}{}", working_directory, filename);
            }
        }

        let input = Raster::new(&dem, "r")?;
        let rows = input.configs.rows;
        let cols = input.configs.columns;
        let nodata = input.configs.nodata;
        let mut z = Vec::with_capacity(rows * cols);
        let mut valid = Vec::with_capacity(rows * cols);
        for row in 0..rows {
            for col in 0..cols {
                let value = input[(row as isize, col as isize)];
                let is_valid = value != nodata;
                valid.push(is_valid);
                z.push(if is_valid { topaz_round(value)? } else { 0 });
            }
        }
        let grid = Grid {
            rows,
            cols,
            z,
            valid,
        };
        let start = Instant::now();
        let (conditioned, fildep, original, stats) = condition_topaz(&grid, max_width)?;
        let elapsed = get_formatted_elapsed_time(start);

        let mut output = Raster::initialize_using_file(&output_file, &input);
        output.configs.data_type = DataType::F64;
        for row in 0..rows {
            let mut data = vec![nodata; cols];
            for col in 0..cols {
                let index = row * cols + col;
                if conditioned.valid[index] {
                    data[col] = conditioned.z[index] as f64 / SCALE;
                }
            }
            output.set_row_data(row as isize, data);
        }
        output.add_metadata_entry(format!("Created by whitebox_tools' {} tool", self.name));
        output.add_metadata_entry(format!("Input DEM file: {}", dem));
        output.add_metadata_entry(format!("Maximum obstruction width: {}", max_width));
        output.add_metadata_entry(format!("TOPAZ compatibility source: {}", SOURCE_REVISION));
        output.add_metadata_entry(format!("Elapsed Time (excluding I/O): {}", elapsed));
        output.write()?;

        if !fildep_file.is_empty() {
            let mut fildep_output = Raster::initialize_using_file(&fildep_file, &input);
            fildep_output.configs.data_type = DataType::F64;
            for row in 0..rows {
                let mut data = vec![nodata; cols];
                for col in 0..cols {
                    let index = row * cols + col;
                    if conditioned.valid[index] {
                        data[col] = fildep[index] as f64 / SCALE;
                    }
                }
                fildep_output.set_row_data(row as isize, data);
            }
            fildep_output.add_metadata_entry(
                "TOPAZ FILDEP stage: depression filling and obstruction adjustment".to_string(),
            );
            fildep_output.add_metadata_entry(format!("Maximum obstruction width: {}", max_width));
            fildep_output
                .add_metadata_entry(format!("TOPAZ compatibility source: {}", SOURCE_REVISION));
            fildep_output.write()?;
        }

        if !delta_file.is_empty() {
            let mut delta = Raster::initialize_using_file(&delta_file, &input);
            delta.configs.data_type = DataType::F64;
            for row in 0..rows {
                let mut data = vec![nodata; cols];
                for col in 0..cols {
                    let index = row * cols + col;
                    if conditioned.valid[index] {
                        data[col] = (conditioned.z[index] - original[index]) as f64 / SCALE;
                    }
                }
                delta.set_row_data(row as isize, data);
            }
            delta.add_metadata_entry("Signed delta: conditioned - TOPAZ-rounded input".to_string());
            delta.add_metadata_entry(format!("TOPAZ compatibility source: {}", SOURCE_REVISION));
            delta.write()?;
        }

        if !diagnostics_file.is_empty() {
            let cell_area = input.configs.resolution_x.abs() * input.configs.resolution_y.abs();
            let diagnostics = json!({
                "schema_version": 1,
                "tool": self.name,
                "source_revision": SOURCE_REVISION,
                "input": dem,
                "output": output_file,
                "fildep_output": if fildep_file.is_empty() { None } else { Some(&fildep_file) },
                "parameters": {"max_obstruction_width": max_width},
                "raster": {"rows": rows, "columns": cols, "nodata": nodata},
                "counts": {
                    "depressions": stats.depressions,
                    "flats": stats.flats,
                    "filled_cells": stats.filled_cells,
                    "lowered_cells": stats.lowered_cells,
                    "synthetic_relief_cells": stats.relief_cells,
                    "obstruction_adjustments_width_1": stats.obstruction_width_1,
                    "obstruction_adjustments_width_2": stats.obstruction_width_2
                },
                "delta_z_units": {
                    "maximum_fill": stats.max_fill as f64 / SCALE,
                    "maximum_cut": stats.max_cut as f64 / SCALE,
                    "maximum_synthetic_relief": stats.max_relief as f64 / SCALE
                },
                "volume_cubic_z_horizontal_units": {
                    "fill": stats.fill_sum as f64 / SCALE * cell_area,
                    "cut": stats.cut_sum as f64 / SCALE * cell_area,
                    "qualification": "Computed from projected raster cell area; caller must confirm compatible horizontal and vertical units."
                },
                "stage_counts": {
                    "fildep_values": fildep.len(),
                    "relief_values": conditioned.z.len()
                }
            });
            let file = File::create(&diagnostics_file)?;
            let mut writer = BufWriter::new(file);
            serde_json::to_writer_pretty(&mut writer, &diagnostics)
                .map_err(|error| Error::new(ErrorKind::Other, error.to_string()))?;
            writer.write_all(b"\n")?;
        }
        if verbose {
            println!("Elapsed Time (excluding I/O): {}", elapsed);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grid(values: &[&[f64]]) -> Grid {
        let rows = values.len();
        let cols = values[0].len();
        Grid {
            rows,
            cols,
            z: values
                .iter()
                .flat_map(|row| row.iter().map(|value| topaz_round(*value).unwrap()))
                .collect(),
            valid: vec![true; rows * cols],
        }
    }

    #[test]
    fn fills_single_cell_pit_and_resolves_flat() {
        let input = grid(&[
            &[10., 10., 10., 10., 10.],
            &[10., 8., 8., 8., 10.],
            &[9., 8., 1., 8., 10.],
            &[10., 8., 8., 8., 10.],
            &[10., 10., 10., 10., 10.],
        ]);
        let (output, fildep, _, _) = condition_topaz(&input, 0).unwrap();
        assert!(fildep[12] >= topaz_round(8.).unwrap());
        assert!(output.z[12] >= fildep[12]);
    }

    #[test]
    fn expands_search_window_to_find_spill() {
        let mut values = vec![vec![9.0; 17]; 17];
        for row in values.iter_mut().take(14).skip(3) {
            for value in row.iter_mut().take(14).skip(3) {
                *value = 5.0;
            }
        }
        values[8][8] = 1.0;
        values[2][8] = 4.0;
        values[1][8] = 3.0;
        values[0][8] = 2.0;
        let rows: Vec<&[f64]> = values.iter().map(Vec::as_slice).collect();
        let input = grid(&rows);
        let (_, fildep, _, _) = condition_topaz(&input, 0).unwrap();
        assert_eq!(fildep[input.idx(8, 8)], topaz_round(5.0).unwrap());
    }

    #[test]
    fn rejects_non_finite_values() {
        assert!(topaz_round(f64::NAN).is_err());
        assert!(topaz_round(f64::INFINITY).is_err());
    }

    #[test]
    fn matches_topaz_f32_half_decimetre_rounding() {
        assert_eq!(topaz_round(775.3499755859375).unwrap(), 77_540_000);
    }

    #[test]
    fn preserves_nodata_mask() {
        let mut input = grid(&[&[5., 5., 5.], &[5., 1., 5.], &[5., 5., 5.]]);
        input.valid[4] = false;
        let (output, _, _, _) = condition_topaz(&input, 2).unwrap();
        assert!(!output.valid[4]);
    }

    #[test]
    fn treats_nodata_as_an_open_lower_boundary_during_relief() {
        let mut input = grid(&[&[5., 5., 5.], &[5., 7., 5.], &[5., 5., 5.]]);
        input.valid.fill(false);
        input.valid[4] = true;
        let original = input.z[4];
        let mut stats = ConditioningStats::default();

        resolve_flats(&mut input, &mut stats);

        assert_eq!(input.z[4], original);
    }

    #[test]
    fn does_not_fill_a_cell_that_drains_to_nodata() {
        let mut input = grid(&[
            &[9., 9., 9., 9., 9.],
            &[9., 8., 8., 8., 9.],
            &[9., 8., 1., 8., 9.],
            &[9., 8., 8., 8., 9.],
            &[9., 9., 9., 9., 9.],
        ]);
        let nodata = input.idx(2, 1);
        input.valid[nodata] = false;
        let original = input.z[input.idx(2, 2)];
        let mut stats = ConditioningStats::default();

        fill_depressions(&mut input, 2, &mut stats).unwrap();

        assert_eq!(input.z[input.idx(2, 2)], original);
    }
}
