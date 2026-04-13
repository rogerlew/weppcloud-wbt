use super::iterative_first_order_link_prune_topology::{
    group_links_by_receiver_discovery_order, select_shortest_link_strict_epsilon, D8PointerScheme,
    FirstOrderLink, GridCell, ReceiverCandidateGroup, TopologyClass, TopologyKernel,
};
use std::io::{Error, ErrorKind};

#[derive(Debug, Clone)]
pub(crate) struct PhaseBInputs {
    pub(crate) rows: isize,
    pub(crate) columns: isize,
    pub(crate) pointers: Vec<u8>,
    pub(crate) pointer_scheme: D8PointerScheme,
    pub(crate) initial_stream_mask: Vec<bool>,
    pub(crate) local_mscl_m: Vec<f64>,
    pub(crate) epsilon: f64,
    pub(crate) fail_if_only_channel_pruned: bool,
    pub(crate) cell_size_x: f64,
    pub(crate) cell_size_y: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PhaseBPassTrace {
    pub(crate) discovered_link_count: usize,
    pub(crate) receiver_order: Vec<GridCell>,
    pub(crate) pruned_sources: Vec<GridCell>,
    pub(crate) degeneration_flag: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PhaseBResult {
    pub(crate) stream_mask: Vec<bool>,
    pub(crate) topology: Vec<TopologyClass>,
    pub(crate) pass_traces: Vec<PhaseBPassTrace>,
}

pub(crate) fn run_phase_b_pruning(inputs: &PhaseBInputs) -> Result<PhaseBResult, Error> {
    validate_inputs(inputs)?;

    let kernel = TopologyKernel::new(
        inputs.rows,
        inputs.columns,
        inputs.pointers.clone(),
        inputs.pointer_scheme,
    )?;

    let mut stream_mask = inputs.initial_stream_mask.clone();
    if !stream_mask.iter().any(|active| *active) {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "No channels remain at first-order-link pruning stage.",
        ));
    }

    let mut pass_traces = Vec::new();
    loop {
        if let Some(cycle_cell) = kernel.find_cycle_cell_in_active_network(&stream_mask)? {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Cycle detected during first-order-link pruning at ({}, {}): active component has no resolvable HEAD/TERMINAL_HEAD traversal.",
                    cycle_cell.row, cycle_cell.col
                ),
            ));
        }

        let links = kernel.discover_first_order_links_row_major(
            &stream_mask,
            inputs.cell_size_x,
            inputs.cell_size_y,
        )?;
        if links.is_empty() && stream_mask.iter().any(|active| *active) {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                "Cycle detected during first-order-link pruning: active network has no discoverable HEAD/TERMINAL_HEAD sources.",
            ));
        }
        let receiver_groups = group_links_by_receiver_discovery_order(&links);
        let pass_trace = process_receiver_groups_for_single_pass(
            &kernel,
            &mut stream_mask,
            &receiver_groups,
            links.len(),
            &inputs.local_mscl_m,
            inputs.epsilon,
            inputs.fail_if_only_channel_pruned,
            inputs.rows,
            inputs.columns,
        )?;
        let degeneration_flag = pass_trace.degeneration_flag;
        pass_traces.push(pass_trace);

        if degeneration_flag {
            let _ = kernel.classify_topology(&stream_mask)?;
            continue;
        }
        break;
    }

    let topology = kernel.classify_topology(&stream_mask)?;
    Ok(PhaseBResult {
        stream_mask,
        topology,
        pass_traces,
    })
}

fn validate_inputs(inputs: &PhaseBInputs) -> Result<(), Error> {
    if inputs.rows <= 0 || inputs.columns <= 0 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Phase B requires positive dimensions (rows={}, columns={})",
                inputs.rows, inputs.columns
            ),
        ));
    }
    if !inputs.epsilon.is_finite() || inputs.epsilon < 0.0 {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Phase B epsilon must be non-negative and finite, got {}",
                inputs.epsilon
            ),
        ));
    }
    if !inputs.cell_size_x.is_finite()
        || !inputs.cell_size_y.is_finite()
        || inputs.cell_size_x <= 0.0
        || inputs.cell_size_y <= 0.0
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Phase B requires positive finite cell sizes (x={}, y={})",
                inputs.cell_size_x, inputs.cell_size_y
            ),
        ));
    }

    let expected_len = (inputs.rows * inputs.columns) as usize;
    for (name, len) in [
        ("pointers", inputs.pointers.len()),
        ("initial_stream_mask", inputs.initial_stream_mask.len()),
        ("local_mscl_m", inputs.local_mscl_m.len()),
    ] {
        if len != expected_len {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Phase B {} length mismatch (expected {}, got {})",
                    name, expected_len, len
                ),
            ));
        }
    }

    for idx in 0..expected_len {
        if !inputs.local_mscl_m[idx].is_finite() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Phase B local MSCL threshold must be finite at index {} (got {})",
                    idx, inputs.local_mscl_m[idx]
                ),
            ));
        }
    }

    Ok(())
}

fn prune_link_immediately(
    stream_mask: &mut [bool],
    candidate: &FirstOrderLink,
    rows: isize,
    columns: isize,
) -> Result<(), Error> {
    if candidate.path.is_empty() {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            "Cannot prune empty first-order-link candidate path.",
        ));
    }

    if candidate.source == candidate.receiver {
        let idx = index_of(rows, columns, candidate.source)?;
        stream_mask[idx] = false;
        return Ok(());
    }

    for cell in candidate.path.iter().take(candidate.path.len() - 1) {
        let idx = index_of(rows, columns, *cell)?;
        stream_mask[idx] = false;
    }
    Ok(())
}

pub(crate) fn process_receiver_groups_for_single_pass(
    kernel: &TopologyKernel,
    stream_mask: &mut [bool],
    receiver_groups: &[ReceiverCandidateGroup],
    discovered_link_count: usize,
    local_mscl_m: &[f64],
    epsilon: f64,
    fail_if_only_channel_pruned: bool,
    rows: isize,
    columns: isize,
) -> Result<PhaseBPassTrace, Error> {
    let mut degeneration_flag = false;
    let mut pass_trace = PhaseBPassTrace {
        discovered_link_count,
        receiver_order: Vec::new(),
        pruned_sources: Vec::new(),
        degeneration_flag: false,
    };

    for group in receiver_groups {
        pass_trace.receiver_order.push(group.receiver);
        let receiver_candidate_count = group.candidates.len();
        let selected = match select_shortest_link_strict_epsilon(&group.candidates, epsilon) {
            Some(candidate) => candidate,
            None => continue,
        };

        if !kernel.candidate_is_still_alive(stream_mask, selected)? {
            continue;
        }

        let receiver_idx = index_of(rows, columns, group.receiver)?;
        if !stream_mask[receiver_idx] {
            continue;
        }

        let local_mscl = local_mscl_m[receiver_idx];
        if selected.length_m < local_mscl - epsilon {
            if fail_if_only_channel_pruned
                && discovered_link_count == 1
                && receiver_candidate_count == 1
            {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "Only-channel prune guard violated at receiver ({}, {}).",
                        group.receiver.row, group.receiver.col
                    ),
                ));
            }

            let pre_inflow = if stream_mask[receiver_idx] {
                kernel.inflow_count(group.receiver, stream_mask)? as usize
            } else {
                0
            };

            prune_link_immediately(stream_mask, selected, rows, columns)?;
            pass_trace.pruned_sources.push(selected.source);

            if !stream_mask.iter().any(|active| *active) {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    "No channels remain during first-order-link pruning.",
                ));
            }

            let post_inflow = if stream_mask[receiver_idx] {
                kernel.inflow_count(group.receiver, stream_mask)? as usize
            } else {
                0
            };
            if pre_inflow >= 2 && post_inflow <= 1 {
                degeneration_flag = true;
            }
        }
    }

    pass_trace.degeneration_flag = degeneration_flag;
    Ok(pass_trace)
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
