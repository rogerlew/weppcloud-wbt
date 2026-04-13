use super::iterative_first_order_link_prune_topology::{
    D8PointerScheme, GridCell, TopologyClass, TopologyKernel,
};
use std::collections::HashSet;
use std::io::{Error, ErrorKind};

#[derive(Debug, Clone)]
pub(crate) struct PhaseAInputs {
    pub(crate) rows: isize,
    pub(crate) columns: isize,
    pub(crate) pointers: Vec<u8>,
    pub(crate) pointer_scheme: D8PointerScheme,
    pub(crate) upstream_area_cells: Vec<f64>,
    pub(crate) active_mask: Vec<bool>,
    pub(crate) local_csa_cells: Vec<f64>,
    pub(crate) min_active_csa_cells: f64,
    pub(crate) epsilon: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PhaseAPassTrace {
    pub(crate) scanned_sources: Vec<GridCell>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PhaseAResult {
    pub(crate) stream_mask: Vec<bool>,
    pub(crate) topology: Vec<TopologyClass>,
    pub(crate) pass_traces: Vec<PhaseAPassTrace>,
}

pub(crate) fn run_phase_a_qualification(inputs: &PhaseAInputs) -> Result<PhaseAResult, Error> {
    validate_inputs(inputs)?;

    let kernel = TopologyKernel::new(
        inputs.rows,
        inputs.columns,
        inputs.pointers.clone(),
        inputs.pointer_scheme,
    )?;

    let mut stream_mask = build_provisional_stream_mask(inputs);
    if !stream_mask.iter().any(|active| *active) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "No channels remain after initial qualification thresholding.",
        ));
    }

    let mut trace = PhaseAPassTrace {
        scanned_sources: Vec::new(),
    };
    for row in 0..inputs.rows {
        for col in 0..inputs.columns {
            let source = GridCell::new(row, col);
            let source_idx = index_of(inputs.rows, inputs.columns, source)?;
            if !stream_mask[source_idx] {
                continue;
            }

            let source_class = live_topology_class(&kernel, &stream_mask, source)?;
            if !matches!(
                source_class,
                TopologyClass::Head | TopologyClass::TerminalHead
            ) {
                continue;
            }

            trace.scanned_sources.push(source);
            let _ = qualify_source_walk(
                &kernel,
                &mut stream_mask,
                source,
                &inputs.upstream_area_cells,
                &inputs.local_csa_cells,
                inputs.rows,
                inputs.columns,
                inputs.epsilon,
            )?;
        }
    }
    let pass_traces = vec![trace];

    if !stream_mask.iter().any(|active| *active) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "No channels remain after source-area qualification.",
        ));
    }

    let topology = kernel.classify_topology(&stream_mask)?;
    Ok(PhaseAResult {
        stream_mask,
        topology,
        pass_traces,
    })
}

fn validate_inputs(inputs: &PhaseAInputs) -> Result<(), Error> {
    if inputs.rows <= 0 || inputs.columns <= 0 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Phase A requires positive dimensions (rows={}, columns={})",
                inputs.rows, inputs.columns
            ),
        ));
    }
    if inputs.epsilon < 0.0 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Phase A epsilon must be non-negative, got {}",
                inputs.epsilon
            ),
        ));
    }
    if !inputs.min_active_csa_cells.is_finite() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Phase A minimum active CSA must be finite, got {}",
                inputs.min_active_csa_cells
            ),
        ));
    }

    let expected_len = (inputs.rows * inputs.columns) as usize;
    for (name, len) in [
        ("pointers", inputs.pointers.len()),
        ("upstream_area_cells", inputs.upstream_area_cells.len()),
        ("active_mask", inputs.active_mask.len()),
        ("local_csa_cells", inputs.local_csa_cells.len()),
    ] {
        if len != expected_len {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Phase A {} length mismatch (expected {}, got {})",
                    name, expected_len, len
                ),
            ));
        }
    }

    for idx in 0..expected_len {
        if !inputs.active_mask[idx] {
            continue;
        }
        if !inputs.upstream_area_cells[idx].is_finite() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Phase A upstream area must be finite at index {} (got {})",
                    idx, inputs.upstream_area_cells[idx]
                ),
            ));
        }
        if !inputs.local_csa_cells[idx].is_finite() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Phase A local CSA must be finite at index {} (got {})",
                    idx, inputs.local_csa_cells[idx]
                ),
            ));
        }
    }

    Ok(())
}

fn build_provisional_stream_mask(inputs: &PhaseAInputs) -> Vec<bool> {
    let mut stream_mask = vec![false; inputs.active_mask.len()];
    for idx in 0..stream_mask.len() {
        if !inputs.active_mask[idx] {
            continue;
        }
        if inputs.upstream_area_cells[idx] >= inputs.min_active_csa_cells - inputs.epsilon {
            stream_mask[idx] = true;
        }
    }
    stream_mask
}

fn qualify_source_walk(
    kernel: &TopologyKernel,
    stream_mask: &mut [bool],
    source: GridCell,
    upstream_area_cells: &[f64],
    local_csa_cells: &[f64],
    rows: isize,
    columns: isize,
    epsilon: f64,
) -> Result<bool, Error> {
    let mut changed = false;
    let mut current = source;
    let mut visited = HashSet::new();

    loop {
        let current_idx = index_of(rows, columns, current)?;
        if !stream_mask[current_idx] {
            break;
        }
        if !visited.insert(current) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Cycle detected during Phase A source walk at ({}, {})",
                    current.row, current.col
                ),
            ));
        }

        if meets_local_csa(current_idx, upstream_area_cells, local_csa_cells, epsilon) {
            break;
        }

        stream_mask[current_idx] = false;
        changed = true;

        let receiver = match kernel.downstream_neighbor(current)? {
            Some(cell) => cell,
            None => break,
        };
        let receiver_idx = index_of(rows, columns, receiver)?;
        if !stream_mask[receiver_idx] {
            break;
        }

        let receiver_class = live_topology_class(kernel, stream_mask, receiver)?;
        if receiver_class == TopologyClass::Junction {
            break;
        }
        if receiver_class == TopologyClass::TerminalJunction {
            let receiver_inflow = kernel.inflow_count(receiver, stream_mask)?;
            if receiver_inflow > 1 {
                break;
            }
            if meets_local_csa(receiver_idx, upstream_area_cells, local_csa_cells, epsilon) {
                break;
            }
            stream_mask[receiver_idx] = false;
            changed = true;
            current = receiver;
            continue;
        }
        if receiver_class == TopologyClass::TerminalHead {
            if meets_local_csa(receiver_idx, upstream_area_cells, local_csa_cells, epsilon) {
                break;
            }
            stream_mask[receiver_idx] = false;
            changed = true;
            current = receiver;
            continue;
        }

        current = receiver;
    }

    Ok(changed)
}

fn meets_local_csa(
    idx: usize,
    upstream_area_cells: &[f64],
    local_csa_cells: &[f64],
    epsilon: f64,
) -> bool {
    upstream_area_cells[idx] >= local_csa_cells[idx] - epsilon
}

fn live_topology_class(
    kernel: &TopologyKernel,
    stream_mask: &[bool],
    cell: GridCell,
) -> Result<TopologyClass, Error> {
    let inflow_count = kernel.inflow_count(cell, stream_mask)?;
    let has_downstream = kernel
        .downstream_stream_neighbor(cell, stream_mask)?
        .is_some();

    Ok(if has_downstream {
        match inflow_count {
            0 => TopologyClass::Head,
            1 => TopologyClass::Mid,
            _ => TopologyClass::Junction,
        }
    } else {
        match inflow_count {
            0 => TopologyClass::TerminalHead,
            _ => TopologyClass::TerminalJunction,
        }
    })
}

fn index_of(rows: isize, columns: isize, cell: GridCell) -> Result<usize, Error> {
    if cell.row < 0 || cell.row >= rows || cell.col < 0 || cell.col >= columns {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Cell ({}, {}) is out of bounds for {}x{} grid",
                cell.row, cell.col, rows, columns
            ),
        ));
    }
    Ok((cell.row * columns + cell.col) as usize)
}
