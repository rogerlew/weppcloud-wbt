use super::iterative_first_order_link_prune_phase_b::{
    process_receiver_groups_for_single_pass, run_phase_b_pruning, PhaseBInputs,
};
use super::iterative_first_order_link_prune_topology::{
    D8PointerScheme, FirstOrderLink, GridCell, ReceiverCandidateGroup, TopologyClass,
    TopologyKernel,
};
use std::io::ErrorKind;

fn index(rows: isize, columns: isize, row: isize, col: isize) -> usize {
    assert!(row >= 0 && row < rows);
    assert!(col >= 0 && col < columns);
    (row * columns + col) as usize
}

fn phase_b_inputs(
    rows: isize,
    columns: isize,
    pointers: Vec<u8>,
    initial_stream_mask: Vec<bool>,
    local_mscl_m: Vec<f64>,
    fail_if_only_channel_pruned: bool,
) -> PhaseBInputs {
    phase_b_inputs_with_scheme(
        rows,
        columns,
        pointers,
        initial_stream_mask,
        local_mscl_m,
        fail_if_only_channel_pruned,
        D8PointerScheme::Whitebox,
    )
}

fn phase_b_inputs_with_scheme(
    rows: isize,
    columns: isize,
    pointers: Vec<u8>,
    initial_stream_mask: Vec<bool>,
    local_mscl_m: Vec<f64>,
    fail_if_only_channel_pruned: bool,
    pointer_scheme: D8PointerScheme,
) -> PhaseBInputs {
    let expected_len = (rows * columns) as usize;
    assert_eq!(pointers.len(), expected_len);
    assert_eq!(initial_stream_mask.len(), expected_len);
    assert_eq!(local_mscl_m.len(), expected_len);

    PhaseBInputs {
        rows,
        columns,
        pointers,
        pointer_scheme,
        initial_stream_mask,
        local_mscl_m,
        epsilon: 1e-5,
        fail_if_only_channel_pruned,
        max_junctions: None,
        cell_size_x: 1.0,
        cell_size_y: 1.0,
    }
}

fn phase_b_inputs_with_cell_size(
    rows: isize,
    columns: isize,
    pointers: Vec<u8>,
    initial_stream_mask: Vec<bool>,
    local_mscl_m: Vec<f64>,
    fail_if_only_channel_pruned: bool,
    cell_size_x: f64,
    cell_size_y: f64,
) -> PhaseBInputs {
    let mut inputs = phase_b_inputs(
        rows,
        columns,
        pointers,
        initial_stream_mask,
        local_mscl_m,
        fail_if_only_channel_pruned,
    );
    inputs.cell_size_x = cell_size_x;
    inputs.cell_size_y = cell_size_y;
    inputs
}

fn chained_tributary_inputs(outlet_mscl: f64) -> PhaseBInputs {
    let rows = 3;
    let columns = 3;
    let mut pointers = vec![2u8; (rows * columns) as usize];
    pointers[index(rows, columns, 0, 0)] = 4; // (0,0) SE -> (1,1)
    pointers[index(rows, columns, 1, 0)] = 2; // (1,0) E -> (1,1)
    pointers[index(rows, columns, 1, 1)] = 8; // (1,1) S -> (2,1)
    pointers[index(rows, columns, 2, 1)] = 8; // (2,1) S -> off-grid

    let mut stream_mask = vec![false; pointers.len()];
    stream_mask[index(rows, columns, 0, 0)] = true;
    stream_mask[index(rows, columns, 1, 0)] = true;
    stream_mask[index(rows, columns, 1, 1)] = true;
    stream_mask[index(rows, columns, 2, 1)] = true;

    let mut local_mscl_m = vec![0.0; pointers.len()];
    local_mscl_m[index(rows, columns, 1, 1)] = 1.1;
    local_mscl_m[index(rows, columns, 2, 1)] = outlet_mscl;

    phase_b_inputs(rows, columns, pointers, stream_mask, local_mscl_m, false)
}

#[test]
fn iterative_first_order_link_prune_phase_b_prunes_adjacent_links_across_degeneration_repasses() {
    let inputs = chained_tributary_inputs(3.0);
    let result = run_phase_b_pruning(&inputs).expect("phase B should succeed");

    assert_eq!(result.pass_traces.len(), 2);
    assert_eq!(
        result.pass_traces[0].receiver_order,
        vec![GridCell::new(1, 1)]
    );
    assert_eq!(
        result.pass_traces[0].pruned_sources,
        vec![GridCell::new(1, 0)]
    );
    assert!(result.pass_traces[0].degeneration_flag);

    assert_eq!(
        result.pass_traces[1].receiver_order,
        vec![GridCell::new(2, 1)]
    );
    assert_eq!(
        result.pass_traces[1].pruned_sources,
        vec![GridCell::new(0, 0)]
    );
    assert!(!result.pass_traces[1].degeneration_flag);

    assert!(!result.stream_mask[index(3, 3, 0, 0)]);
    assert!(!result.stream_mask[index(3, 3, 1, 0)]);
    assert!(!result.stream_mask[index(3, 3, 1, 1)]);
    assert!(result.stream_mask[index(3, 3, 2, 1)]);
    assert_eq!(
        result.topology[index(3, 3, 2, 1)],
        TopologyClass::TerminalHead
    );
}

#[test]
fn iterative_first_order_link_prune_phase_b_receiver_transition_preserves_receiver_cell() {
    let inputs = chained_tributary_inputs(0.0);
    let result = run_phase_b_pruning(&inputs).expect("phase B should succeed");

    assert_eq!(result.pass_traces.len(), 2);
    assert_eq!(
        result.pass_traces[0].pruned_sources,
        vec![GridCell::new(1, 0)]
    );
    assert!(result.pass_traces[0].degeneration_flag);
    assert!(result.pass_traces[1].pruned_sources.is_empty());
    assert!(!result.pass_traces[1].degeneration_flag);

    assert!(result.stream_mask[index(3, 3, 0, 0)]);
    assert!(!result.stream_mask[index(3, 3, 1, 0)]);
    assert!(result.stream_mask[index(3, 3, 1, 1)]);
    assert!(result.stream_mask[index(3, 3, 2, 1)]);
    assert_eq!(result.topology[index(3, 3, 1, 1)], TopologyClass::Mid);
}

#[test]
fn iterative_first_order_link_prune_phase_b_only_channel_guard_raises_failure() {
    let rows = 1;
    let columns = 4;
    let pointers = vec![2u8, 2u8, 2u8, 2u8];
    let stream_mask = vec![true, true, true, true];
    let local_mscl_m = vec![0.0, 0.0, 0.0, 4.0];
    let inputs = phase_b_inputs(rows, columns, pointers, stream_mask, local_mscl_m, true);

    let err = run_phase_b_pruning(&inputs).expect_err("only-channel guard should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err
        .to_string()
        .contains("Only-channel prune guard violated"));
}

#[test]
fn iterative_first_order_link_prune_phase_b_only_channel_guard_can_be_disabled() {
    let rows = 1;
    let columns = 4;
    let pointers = vec![2u8, 2u8, 2u8, 2u8];
    let stream_mask = vec![true, true, true, true];
    let local_mscl_m = vec![0.0, 0.0, 0.0, 4.0];
    let inputs = phase_b_inputs(rows, columns, pointers, stream_mask, local_mscl_m, false);

    let result = run_phase_b_pruning(&inputs).expect("phase B should succeed");
    assert_eq!(result.pass_traces.len(), 1);
    assert_eq!(
        result.pass_traces[0].pruned_sources,
        vec![GridCell::new(0, 0)]
    );
    assert!(!result.pass_traces[0].degeneration_flag);

    assert_eq!(result.stream_mask, vec![false, false, false, true]);
    assert_eq!(
        result.topology[index(1, 4, 0, 3)],
        TopologyClass::TerminalHead
    );
}

#[test]
fn iterative_first_order_link_prune_phase_b_esri_pointer_mode_prunes_short_link() {
    let rows = 1;
    let columns = 4;
    let pointers = vec![1u8, 1u8, 1u8, 1u8];
    let stream_mask = vec![true, true, true, true];
    let local_mscl_m = vec![0.0, 0.0, 0.0, 4.0];
    let inputs = phase_b_inputs_with_scheme(
        rows,
        columns,
        pointers,
        stream_mask,
        local_mscl_m,
        false,
        D8PointerScheme::Esri,
    );

    let result = run_phase_b_pruning(&inputs).expect("phase B should succeed in ESRI mode");
    assert_eq!(result.pass_traces.len(), 1);
    assert_eq!(
        result.pass_traces[0].receiver_order,
        vec![GridCell::new(0, 3)]
    );
    assert_eq!(
        result.pass_traces[0].pruned_sources,
        vec![GridCell::new(0, 0)]
    );
    assert!(!result.pass_traces[0].degeneration_flag);
    assert_eq!(result.stream_mask, vec![false, false, false, true]);
    assert_eq!(
        result.topology[index(rows, columns, 0, 3)],
        TopologyClass::TerminalHead
    );
}

#[test]
fn iterative_first_order_link_prune_phase_b_self_receiver_prunes_immediately_without_repass() {
    let rows = 1;
    let columns = 2;
    let pointers = vec![32u8, 2u8];
    let stream_mask = vec![true, true];
    let local_mscl_m = vec![1.0, 0.0];
    let inputs = phase_b_inputs(rows, columns, pointers, stream_mask, local_mscl_m, false);

    let result = run_phase_b_pruning(&inputs).expect("phase B should succeed");
    assert_eq!(result.pass_traces.len(), 1);
    assert_eq!(
        result.pass_traces[0].receiver_order,
        vec![GridCell::new(0, 0), GridCell::new(0, 1)]
    );
    assert_eq!(
        result.pass_traces[0].pruned_sources,
        vec![GridCell::new(0, 0)]
    );
    assert!(!result.pass_traces[0].degeneration_flag);
    assert_eq!(result.stream_mask, vec![false, true]);
}

#[test]
fn iterative_first_order_link_prune_phase_b_fails_when_no_channels_exist_on_entry() {
    let inputs = phase_b_inputs(1, 1, vec![2u8], vec![false], vec![1.0], false);
    let err = run_phase_b_pruning(&inputs).expect_err("phase B should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err
        .to_string()
        .contains("No channels remain at first-order-link pruning stage"));
}

#[test]
fn iterative_first_order_link_prune_phase_b_rejects_non_finite_epsilon() {
    let mut inputs = phase_b_inputs(1, 1, vec![2u8], vec![true], vec![1.0], false);
    inputs.epsilon = f64::NAN;

    let err = run_phase_b_pruning(&inputs).expect_err("phase B should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("non-negative and finite"));
}

#[test]
fn iterative_first_order_link_prune_phase_b_rejects_non_finite_cell_size() {
    let mut inputs = phase_b_inputs(1, 1, vec![2u8], vec![true], vec![1.0], false);
    inputs.cell_size_x = f64::INFINITY;

    let err = run_phase_b_pruning(&inputs).expect_err("phase B should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("positive finite cell sizes"));
}

#[test]
fn iterative_first_order_link_prune_phase_b_mscl_threshold_is_not_scaled_by_cell_size() {
    let rows = 1;
    let columns = 2;
    let pointers = vec![2u8, 2u8];
    let stream_mask = vec![true, true];
    let local_mscl_m = vec![0.0, 2.0];
    let inputs = phase_b_inputs_with_cell_size(
        rows,
        columns,
        pointers,
        stream_mask,
        local_mscl_m,
        false,
        10.0,
        10.0,
    );

    let result = run_phase_b_pruning(&inputs).expect("phase B should succeed");
    assert_eq!(result.pass_traces.len(), 1);
    assert!(result.pass_traces[0].pruned_sources.is_empty());
    assert_eq!(result.stream_mask, vec![true, true]);
}

#[test]
fn iterative_first_order_link_prune_phase_b_prunes_only_one_candidate_per_receiver_group_per_pass()
{
    let rows = 3;
    let columns = 3;
    let mut pointers = vec![2u8; (rows * columns) as usize];
    pointers[index(rows, columns, 0, 0)] = 4; // (0,0) SE -> (1,1)
    pointers[index(rows, columns, 0, 1)] = 8; // (0,1) S -> (1,1)
    pointers[index(rows, columns, 1, 0)] = 2; // (1,0) E -> (1,1)
    pointers[index(rows, columns, 1, 1)] = 8; // (1,1) S -> (2,1)
    pointers[index(rows, columns, 2, 1)] = 8; // (2,1) S -> off-grid

    let mut stream_mask = vec![false; pointers.len()];
    stream_mask[index(rows, columns, 0, 0)] = true;
    stream_mask[index(rows, columns, 0, 1)] = true;
    stream_mask[index(rows, columns, 1, 0)] = true;
    stream_mask[index(rows, columns, 1, 1)] = true;
    stream_mask[index(rows, columns, 2, 1)] = true;

    let mut local_mscl_m = vec![0.0; pointers.len()];
    local_mscl_m[index(rows, columns, 1, 1)] = 2.0;
    let inputs = phase_b_inputs(rows, columns, pointers, stream_mask, local_mscl_m, false);

    let result = run_phase_b_pruning(&inputs).expect("phase B should succeed");
    assert_eq!(result.pass_traces.len(), 1);
    assert_eq!(
        result.pass_traces[0].receiver_order,
        vec![GridCell::new(1, 1)]
    );
    assert_eq!(
        result.pass_traces[0].pruned_sources,
        vec![GridCell::new(0, 1)]
    );
    assert!(!result.pass_traces[0].degeneration_flag);

    // Two additional short candidates remain because Phase B does not fallback to the next
    // candidate in the same receiver group after pruning one selected candidate.
    assert!(result.stream_mask[index(rows, columns, 0, 0)]);
    assert!(!result.stream_mask[index(rows, columns, 0, 1)]);
    assert!(result.stream_mask[index(rows, columns, 1, 0)]);
}

#[test]
fn iterative_first_order_link_prune_phase_b_skips_stale_later_group_without_fallback() {
    let rows = 1;
    let columns = 5;
    let pointers = vec![2u8; (rows * columns) as usize];
    let kernel = TopologyKernel::new(rows, columns, pointers, D8PointerScheme::Whitebox).unwrap();

    let mut stream_mask = vec![true; (rows * columns) as usize];
    let mut local_mscl_m = vec![0.0; (rows * columns) as usize];
    local_mscl_m[index(rows, columns, 0, 2)] = 10.0;
    local_mscl_m[index(rows, columns, 0, 4)] = 10.0;

    let receiver_groups = vec![
        ReceiverCandidateGroup {
            receiver: GridCell::new(0, 2),
            candidates: vec![FirstOrderLink {
                source: GridCell::new(0, 0),
                receiver: GridCell::new(0, 2),
                path: vec![
                    GridCell::new(0, 0),
                    GridCell::new(0, 1),
                    GridCell::new(0, 2),
                ],
                length_m: 2.0,
                encounter_order: 0,
            }],
        },
        ReceiverCandidateGroup {
            receiver: GridCell::new(0, 4),
            candidates: vec![
                FirstOrderLink {
                    source: GridCell::new(0, 1),
                    receiver: GridCell::new(0, 4),
                    path: vec![
                        GridCell::new(0, 1),
                        GridCell::new(0, 2),
                        GridCell::new(0, 3),
                        GridCell::new(0, 4),
                    ],
                    length_m: 2.0,
                    encounter_order: 1,
                },
                FirstOrderLink {
                    source: GridCell::new(0, 3),
                    receiver: GridCell::new(0, 4),
                    path: vec![GridCell::new(0, 3), GridCell::new(0, 4)],
                    length_m: 3.0,
                    encounter_order: 2,
                },
            ],
        },
    ];

    let mut active_count = stream_mask.iter().filter(|active| **active).count();
    let pass_trace = process_receiver_groups_for_single_pass(
        &kernel,
        &mut stream_mask,
        &receiver_groups,
        3,
        &local_mscl_m,
        1e-5,
        false,
        None,
        rows,
        columns,
        &mut active_count,
    )
    .expect("single-pass receiver processing should succeed");

    assert_eq!(
        pass_trace.receiver_order,
        vec![GridCell::new(0, 2), GridCell::new(0, 4)]
    );
    assert_eq!(pass_trace.pruned_sources, vec![GridCell::new(0, 0)]);
    assert!(!pass_trace.degeneration_flag);

    // First group pruning removes cells (0,0) and (0,1), making the later group's selected
    // candidate stale. Phase B must skip that receiver group without falling back to the next
    // candidate in the same group.
    assert_eq!(
        stream_mask,
        vec![
            false, // pruned source of first receiver group
            false, // pruned interior cell of first receiver group
            true,  // preserved receiver of first group
            true,  // would be pruned by fallback candidate if fallback occurred
            true
        ]
    );
}

#[test]
fn iterative_first_order_link_prune_phase_b_max_junctions_three_prunes_deterministically() {
    let rows = 5;
    let columns = 5;
    let mut pointers = vec![2u8; (rows * columns) as usize];
    let receiver = GridCell::new(2, 2);

    // Four incoming first-order links into one receiver.
    pointers[index(rows, columns, 1, 1)] = 4; // (1,1) SE -> (2,2)
    pointers[index(rows, columns, 1, 2)] = 8; // (1,2) S -> (2,2)
    pointers[index(rows, columns, 2, 1)] = 2; // (2,1) E -> (2,2)
    pointers[index(rows, columns, 3, 2)] = 128; // (3,2) N -> (2,2)

    let mut stream_mask = vec![false; pointers.len()];
    stream_mask[index(rows, columns, 1, 1)] = true;
    stream_mask[index(rows, columns, 1, 2)] = true;
    stream_mask[index(rows, columns, 2, 1)] = true;
    stream_mask[index(rows, columns, 2, 2)] = true;
    stream_mask[index(rows, columns, 3, 2)] = true;

    let local_mscl_m = vec![0.0; pointers.len()];
    let mut inputs = phase_b_inputs(rows, columns, pointers, stream_mask, local_mscl_m, false);
    inputs.max_junctions = Some(3);

    let first = run_phase_b_pruning(&inputs).expect("phase B should succeed");
    let second = run_phase_b_pruning(&inputs).expect("phase B rerun should be deterministic");

    assert_eq!(first.pass_traces, second.pass_traces);
    assert_eq!(first.stream_mask, second.stream_mask);
    assert_eq!(first.pass_traces.len(), 1);
    assert_eq!(first.pass_traces[0].receiver_order, vec![receiver]);
    assert_eq!(
        first.pass_traces[0].pruned_sources,
        vec![GridCell::new(1, 2)]
    );
    assert!(!first.pass_traces[0].degeneration_flag);
    assert!(!first.stream_mask[index(rows, columns, 1, 2)]);
    assert!(first.stream_mask[index(rows, columns, 1, 1)]);
    assert!(first.stream_mask[index(rows, columns, 2, 1)]);
    assert!(first.stream_mask[index(rows, columns, 3, 2)]);
}

#[test]
fn iterative_first_order_link_prune_phase_b_rejects_source_less_cycle_network() {
    let rows = 2;
    let columns = 2;
    let mut pointers = vec![2u8; (rows * columns) as usize];
    pointers[index(rows, columns, 0, 0)] = 2; // (0,0) E -> (0,1)
    pointers[index(rows, columns, 0, 1)] = 8; // (0,1) S -> (1,1)
    pointers[index(rows, columns, 1, 1)] = 32; // (1,1) W -> (1,0)
    pointers[index(rows, columns, 1, 0)] = 128; // (1,0) N -> (0,0)

    let stream_mask = vec![true; pointers.len()];
    let local_mscl_m = vec![0.0; pointers.len()];
    let inputs = phase_b_inputs(rows, columns, pointers, stream_mask, local_mscl_m, false);

    let err = run_phase_b_pruning(&inputs).expect_err("source-less cycle should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("Cycle detected"));
    assert!(err.to_string().contains("HEAD/TERMINAL_HEAD"));
}

#[test]
fn iterative_first_order_link_prune_phase_b_rejects_mixed_component_with_disconnected_cycle() {
    let rows = 2;
    let columns = 4;
    let mut pointers = vec![2u8; (rows * columns) as usize];

    // Valid terminal-head component at (0,0).
    pointers[index(rows, columns, 0, 0)] = 32; // W -> off-grid

    // Disconnected 2x2 directed cycle over (0,2),(0,3),(1,3),(1,2).
    pointers[index(rows, columns, 0, 2)] = 2; // E -> (0,3)
    pointers[index(rows, columns, 0, 3)] = 8; // S -> (1,3)
    pointers[index(rows, columns, 1, 3)] = 32; // W -> (1,2)
    pointers[index(rows, columns, 1, 2)] = 128; // N -> (0,2)

    let mut stream_mask = vec![false; pointers.len()];
    stream_mask[index(rows, columns, 0, 0)] = true;
    stream_mask[index(rows, columns, 0, 2)] = true;
    stream_mask[index(rows, columns, 0, 3)] = true;
    stream_mask[index(rows, columns, 1, 2)] = true;
    stream_mask[index(rows, columns, 1, 3)] = true;

    let local_mscl_m = vec![0.0; pointers.len()];
    let inputs = phase_b_inputs(rows, columns, pointers, stream_mask, local_mscl_m, false);

    let err = run_phase_b_pruning(&inputs).expect_err("mixed disconnected cycle should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("Cycle detected"));
    assert!(err.to_string().contains("(0, 2)"));
}
