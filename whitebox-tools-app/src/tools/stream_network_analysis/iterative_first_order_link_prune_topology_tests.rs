use super::iterative_first_order_link_prune_topology::{
    group_links_by_receiver_discovery_order, select_shortest_link_strict_epsilon, D8PointerScheme,
    GridCell, TopologyClass, TopologyKernel,
};
use std::io::ErrorKind;

fn index(rows: isize, columns: isize, row: isize, col: isize) -> usize {
    assert!(row >= 0 && row < rows);
    assert!(col >= 0 && col < columns);
    (row * columns + col) as usize
}

fn synthetic_kernel_with_terminal_head() -> (TopologyKernel, Vec<bool>) {
    let rows = 3;
    let columns = 3;

    // Defaults use eastward flow for non-stream cells.
    let mut pointers = vec![2u8; (rows * columns) as usize];
    pointers[index(rows, columns, 0, 0)] = 4; // SE -> (1,1)
    pointers[index(rows, columns, 0, 2)] = 16; // SW -> (1,1)
    pointers[index(rows, columns, 1, 1)] = 8; // S -> (2,1)
    pointers[index(rows, columns, 2, 1)] = 8; // S -> off-grid
    pointers[index(rows, columns, 2, 2)] = 2; // E -> off-grid

    let mut stream_mask = vec![false; pointers.len()];
    stream_mask[index(rows, columns, 0, 0)] = true;
    stream_mask[index(rows, columns, 0, 2)] = true;
    stream_mask[index(rows, columns, 1, 1)] = true;
    stream_mask[index(rows, columns, 2, 1)] = true;
    stream_mask[index(rows, columns, 2, 2)] = true;

    (
        TopologyKernel::new(rows, columns, pointers, D8PointerScheme::Whitebox)
            .expect("synthetic kernel should be valid"),
        stream_mask,
    )
}

#[test]
fn iterative_first_order_link_prune_topology_decodes_whitebox_and_esri_pointers() {
    let whitebox_kernel = TopologyKernel::new(3, 3, vec![2u8; 9], D8PointerScheme::Whitebox)
        .expect("whitebox kernel should build");
    let esri_kernel =
        TopologyKernel::new(3, 3, vec![1u8; 9], D8PointerScheme::Esri).expect("esri kernel");

    assert_eq!(whitebox_kernel.decode_pointer_index(2).unwrap(), 1);
    assert_eq!(esri_kernel.decode_pointer_index(1).unwrap(), 1);

    let mut whitebox_stream = vec![false; 9];
    whitebox_stream[index(3, 3, 1, 0)] = true;
    whitebox_stream[index(3, 3, 1, 1)] = true;
    whitebox_stream[index(3, 3, 1, 2)] = true;
    assert_eq!(
        whitebox_kernel
            .downstream_stream_neighbor(GridCell::new(1, 1), &whitebox_stream)
            .unwrap(),
        Some(GridCell::new(1, 2))
    );
    let upstream_whitebox = whitebox_kernel
        .upstream_stream_neighbors(GridCell::new(1, 1), &whitebox_stream)
        .unwrap();
    assert_eq!(upstream_whitebox, vec![GridCell::new(1, 0)]);

    let mut esri_stream = vec![false; 9];
    esri_stream[index(3, 3, 1, 0)] = true;
    esri_stream[index(3, 3, 1, 1)] = true;
    esri_stream[index(3, 3, 1, 2)] = true;
    assert_eq!(
        esri_kernel
            .downstream_stream_neighbor(GridCell::new(1, 1), &esri_stream)
            .unwrap(),
        Some(GridCell::new(1, 2))
    );
    let upstream_esri = esri_kernel
        .upstream_stream_neighbors(GridCell::new(1, 1), &esri_stream)
        .unwrap();
    assert_eq!(upstream_esri, vec![GridCell::new(1, 0)]);
}

#[test]
fn iterative_first_order_link_prune_topology_classifies_inflow_and_state_correctly() {
    let (kernel, stream_mask) = synthetic_kernel_with_terminal_head();
    let inflows = kernel.compute_inflow_counts(&stream_mask).unwrap();
    let classes = kernel.classify_topology(&stream_mask).unwrap();

    assert_eq!(inflows[index(3, 3, 0, 0)], 0);
    assert_eq!(inflows[index(3, 3, 0, 2)], 0);
    assert_eq!(inflows[index(3, 3, 1, 1)], 2);
    assert_eq!(inflows[index(3, 3, 2, 1)], 1);
    assert_eq!(inflows[index(3, 3, 2, 2)], 0);

    assert_eq!(classes[index(3, 3, 0, 0)], TopologyClass::Head);
    assert_eq!(classes[index(3, 3, 0, 2)], TopologyClass::Head);
    assert_eq!(classes[index(3, 3, 1, 1)], TopologyClass::Junction);
    assert_eq!(classes[index(3, 3, 2, 1)], TopologyClass::TerminalJunction);
    assert_eq!(classes[index(3, 3, 2, 2)], TopologyClass::TerminalHead);
}

#[test]
fn iterative_first_order_link_prune_topology_discovers_first_order_links_deterministically() {
    let (kernel, stream_mask) = synthetic_kernel_with_terminal_head();
    let links = kernel
        .discover_first_order_links_row_major(&stream_mask, 1.0, 1.0)
        .unwrap();

    assert_eq!(links.len(), 3);
    assert_eq!(links[0].encounter_order, 0);
    assert_eq!(links[1].encounter_order, 1);
    assert_eq!(links[2].encounter_order, 2);
    assert_eq!(links[0].source, GridCell::new(0, 0));
    assert_eq!(links[1].source, GridCell::new(0, 2));
    assert_eq!(links[2].source, GridCell::new(2, 2));

    assert_eq!(links[0].receiver, GridCell::new(1, 1));
    assert_eq!(links[1].receiver, GridCell::new(1, 1));
    assert_eq!(links[2].receiver, GridCell::new(2, 2));

    let receiver_groups = group_links_by_receiver_discovery_order(&links);
    assert_eq!(receiver_groups.len(), 2);
    assert_eq!(receiver_groups[0].receiver, GridCell::new(1, 1));
    assert_eq!(receiver_groups[0].candidates.len(), 2);
    assert_eq!(receiver_groups[1].receiver, GridCell::new(2, 2));
    assert_eq!(receiver_groups[1].candidates.len(), 1);
}

#[test]
fn iterative_first_order_link_prune_topology_tie_breaks_by_first_encounter_under_epsilon() {
    let (kernel, stream_mask) = synthetic_kernel_with_terminal_head();
    let links = kernel
        .discover_first_order_links_row_major(&stream_mask, 1.0, 1.0)
        .unwrap();
    let receiver_groups = group_links_by_receiver_discovery_order(&links);
    let branch_group = &receiver_groups[0].candidates;

    let best = select_shortest_link_strict_epsilon(branch_group, 1e-5).unwrap();
    assert_eq!(best.source, GridCell::new(0, 0));

    let mut strict_improvement = branch_group.clone();
    strict_improvement[1].length_m = strict_improvement[0].length_m - 2e-5;
    let best = select_shortest_link_strict_epsilon(&strict_improvement, 1e-5).unwrap();
    assert_eq!(best.source, GridCell::new(0, 2));

    let mut epsilon_tie = branch_group.clone();
    epsilon_tie[1].length_m = epsilon_tie[0].length_m - 5e-6;
    let best = select_shortest_link_strict_epsilon(&epsilon_tie, 1e-5).unwrap();
    assert_eq!(best.source, GridCell::new(0, 0));
}

#[test]
fn iterative_first_order_link_prune_topology_stale_candidate_checks_require_live_path() {
    let rows = 1;
    let columns = 4;
    let pointers = vec![2u8, 2u8, 2u8, 2u8];
    let kernel = TopologyKernel::new(rows, columns, pointers, D8PointerScheme::Whitebox).unwrap();
    let stream_mask = vec![true, true, true, true];
    let links = kernel
        .discover_first_order_links_row_major(&stream_mask, 1.0, 1.0)
        .unwrap();
    assert_eq!(links.len(), 1);
    let candidate = &links[0];

    assert!(kernel
        .candidate_is_still_alive(&stream_mask, candidate)
        .unwrap());

    let mut missing_middle = stream_mask.clone();
    missing_middle[index(rows, columns, 0, 2)] = false;
    assert!(!kernel
        .candidate_is_still_alive(&missing_middle, candidate)
        .unwrap());

    let mut missing_source = stream_mask.clone();
    missing_source[index(rows, columns, 0, 0)] = false;
    assert!(!kernel
        .candidate_is_still_alive(&missing_source, candidate)
        .unwrap());
}

#[test]
fn iterative_first_order_link_prune_topology_decodes_all_d8_codes_and_rejects_invalid() {
    let whitebox = TopologyKernel::new(1, 1, vec![1u8], D8PointerScheme::Whitebox).unwrap();
    let esri = TopologyKernel::new(1, 1, vec![1u8], D8PointerScheme::Esri).unwrap();

    for (pointer_value, expected_direction) in [
        (1u8, 0usize),
        (2, 1),
        (4, 2),
        (8, 3),
        (16, 4),
        (32, 5),
        (64, 6),
        (128, 7),
    ] {
        assert_eq!(
            whitebox.decode_pointer_index(pointer_value).unwrap(),
            expected_direction
        );
    }

    for (pointer_value, expected_direction) in [
        (128u8, 0usize),
        (1, 1),
        (2, 2),
        (4, 3),
        (8, 4),
        (16, 5),
        (32, 6),
        (64, 7),
    ] {
        assert_eq!(
            esri.decode_pointer_index(pointer_value).unwrap(),
            expected_direction
        );
    }

    for invalid in [0u8, 3u8, 255u8] {
        let err = whitebox
            .decode_pointer_index(invalid)
            .expect_err("invalid pointer value should fail");
        assert_eq!(err.kind(), ErrorKind::InvalidInput);
    }
}

#[test]
fn iterative_first_order_link_prune_topology_classifies_mid_and_nonstream() {
    let rows = 1;
    let columns = 4;
    let pointers = vec![2u8, 2u8, 2u8, 2u8];
    let stream_mask = vec![true, true, true, false];
    let kernel = TopologyKernel::new(rows, columns, pointers, D8PointerScheme::Whitebox).unwrap();
    let classes = kernel.classify_topology(&stream_mask).unwrap();

    assert_eq!(classes[index(rows, columns, 0, 0)], TopologyClass::Head);
    assert_eq!(classes[index(rows, columns, 0, 1)], TopologyClass::Mid);
    assert_eq!(
        classes[index(rows, columns, 0, 2)],
        TopologyClass::TerminalJunction
    );
    assert_eq!(
        classes[index(rows, columns, 0, 3)],
        TopologyClass::NonStream
    );
}

#[test]
fn iterative_first_order_link_prune_topology_link_stops_at_terminal_junction_receiver() {
    let rows = 1;
    let columns = 4;
    let pointers = vec![2u8, 2u8, 2u8, 2u8];
    let stream_mask = vec![true, true, true, false];
    let kernel = TopologyKernel::new(rows, columns, pointers, D8PointerScheme::Whitebox).unwrap();
    let topology = kernel.classify_topology(&stream_mask).unwrap();

    let link = kernel
        .first_order_link_from_source(GridCell::new(0, 0), &stream_mask, &topology, 1.0, 1.0)
        .unwrap()
        .expect("head source should produce a link");

    assert_eq!(link.source, GridCell::new(0, 0));
    assert_eq!(link.receiver, GridCell::new(0, 2));
    assert_eq!(
        link.path,
        vec![
            GridCell::new(0, 0),
            GridCell::new(0, 1),
            GridCell::new(0, 2)
        ]
    );
    assert!((link.length_m - 2.0).abs() < 1e-12);
}

#[test]
fn iterative_first_order_link_prune_topology_tie_break_exact_epsilon_keeps_first() {
    let (kernel, stream_mask) = synthetic_kernel_with_terminal_head();
    let links = kernel
        .discover_first_order_links_row_major(&stream_mask, 1.0, 1.0)
        .unwrap();
    let receiver_groups = group_links_by_receiver_discovery_order(&links);
    let mut branch_group = receiver_groups[0].candidates.clone();
    let epsilon = 1e-5;

    branch_group[1].length_m = branch_group[0].length_m - epsilon;
    let best = select_shortest_link_strict_epsilon(&branch_group, epsilon).unwrap();
    assert_eq!(best.source, GridCell::new(0, 0));
}

#[test]
fn iterative_first_order_link_prune_topology_negative_epsilon_does_not_flip_selection() {
    let (kernel, stream_mask) = synthetic_kernel_with_terminal_head();
    let links = kernel
        .discover_first_order_links_row_major(&stream_mask, 1.0, 1.0)
        .unwrap();
    let receiver_groups = group_links_by_receiver_discovery_order(&links);
    let mut branch_group = receiver_groups[0].candidates.clone();
    branch_group[1].length_m = branch_group[0].length_m + 0.25;

    let best = select_shortest_link_strict_epsilon(&branch_group, -1.0).unwrap();
    assert_eq!(best.source, GridCell::new(0, 0));
}

#[test]
fn iterative_first_order_link_prune_topology_discovery_repeatable_across_runs() {
    let (kernel, stream_mask) = synthetic_kernel_with_terminal_head();
    let first = kernel
        .discover_first_order_links_row_major(&stream_mask, 1.0, 1.0)
        .unwrap();
    let second = kernel
        .discover_first_order_links_row_major(&stream_mask, 1.0, 1.0)
        .unwrap();

    assert_eq!(first, second);
}

#[test]
fn iterative_first_order_link_prune_topology_stale_candidate_detects_rewired_downstream() {
    let rows = 1;
    let columns = 4;
    let base_pointers = vec![2u8, 2u8, 2u8, 2u8];
    let stream_mask = vec![true, true, true, true];
    let base_kernel =
        TopologyKernel::new(rows, columns, base_pointers, D8PointerScheme::Whitebox).unwrap();
    let mut candidates = base_kernel
        .discover_first_order_links_row_major(&stream_mask, 1.0, 1.0)
        .unwrap();
    let candidate = candidates.remove(0);

    let rewired_pointers = vec![2u8, 8u8, 2u8, 2u8];
    let rewired_kernel =
        TopologyKernel::new(rows, columns, rewired_pointers, D8PointerScheme::Whitebox).unwrap();
    assert!(!rewired_kernel
        .candidate_is_still_alive(&stream_mask, &candidate)
        .unwrap());

    let mut receiver_removed = stream_mask.clone();
    receiver_removed[index(rows, columns, 0, 3)] = false;
    assert!(!base_kernel
        .candidate_is_still_alive(&receiver_removed, &candidate)
        .unwrap());
}

#[test]
fn iterative_first_order_link_prune_topology_terminal_head_uses_half_cell_length() {
    let (kernel, stream_mask) = synthetic_kernel_with_terminal_head();
    let links = kernel
        .discover_first_order_links_row_major(&stream_mask, 3.0, 4.0)
        .unwrap();
    let terminal_head = links
        .iter()
        .find(|link| link.source == GridCell::new(2, 2))
        .expect("terminal-head link should exist");

    assert_eq!(terminal_head.source, terminal_head.receiver);
    assert!((terminal_head.length_m - 1.5).abs() < 1e-12);
}

#[test]
fn iterative_first_order_link_prune_topology_parallel_inflow_counts_match_manual_reference() {
    let rows = 1024;
    let columns = 8;

    let mut pointers = vec![2u8; (rows * columns) as usize];
    for row in 0..rows {
        pointers[index(rows, columns, row, columns - 1)] = 8; // last column drains south
    }
    pointers[index(rows, columns, rows - 1, columns - 1)] = 2; // outlet off-grid

    let kernel = TopologyKernel::new(rows, columns, pointers, D8PointerScheme::Whitebox).unwrap();
    let stream_mask = vec![true; (rows * columns) as usize];

    let computed = kernel
        .compute_inflow_counts_with_forced_threads_for_tests(&stream_mask, 2)
        .unwrap();
    let mut manual = vec![0u8; computed.len()];
    for row in 0..rows {
        for col in 0..columns {
            let cell = GridCell::new(row, col);
            let idx = index(rows, columns, row, col);
            let mut inflow = 0u8;
            for n in 0..8 {
                let neighbor = GridCell::new(
                    row + [-1, 0, 1, 1, 1, 0, -1, -1][n],
                    col + [1, 1, 1, 0, -1, -1, -1, 0][n],
                );
                if !kernel.is_in_bounds(neighbor) {
                    continue;
                }
                if let Some(downstream) = kernel.downstream_neighbor(neighbor).unwrap() {
                    if downstream == cell {
                        inflow += 1;
                    }
                }
            }
            manual[idx] = inflow;
        }
    }

    assert_eq!(computed, manual);
    assert_eq!(
        computed,
        kernel.compute_inflow_counts(&stream_mask).unwrap()
    );
}

#[test]
fn iterative_first_order_link_prune_topology_parallel_inflow_counts_propagate_worker_pointer_errors(
) {
    let rows = 1024;
    let columns = 2;
    let mut pointers = vec![2u8; (rows * columns) as usize];
    pointers[index(rows, columns, 512, 0)] = 0; // invalid D8 code in active domain

    let kernel = TopologyKernel::new(rows, columns, pointers, D8PointerScheme::Whitebox).unwrap();
    let stream_mask = vec![true; (rows * columns) as usize];

    let err = kernel
        .compute_inflow_counts_with_forced_threads_for_tests(&stream_mask, 2)
        .expect_err("threaded worker path should propagate invalid pointer errors");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("Invalid D8 pointer value 0"));
}

#[test]
fn iterative_first_order_link_prune_topology_default_inflow_counts_honor_max_procs_contract() {
    let rows = 1024;
    let columns = 8;

    let mut pointers = vec![2u8; (rows * columns) as usize];
    for row in 0..rows {
        pointers[index(rows, columns, row, columns - 1)] = 8; // last column drains south
    }
    pointers[index(rows, columns, rows - 1, columns - 1)] = 2; // outlet off-grid

    let kernel = TopologyKernel::new(rows, columns, pointers, D8PointerScheme::Whitebox).unwrap();
    let stream_mask = vec![true; (rows * columns) as usize];

    let default_counts = kernel.compute_inflow_counts(&stream_mask).unwrap();
    let max_proc_serial = kernel
        .compute_inflow_counts_with_max_procs_for_tests(&stream_mask, 1)
        .unwrap();
    let max_proc_threaded = kernel
        .compute_inflow_counts_with_max_procs_for_tests(&stream_mask, 2)
        .unwrap();

    let mut manual = vec![0u8; default_counts.len()];
    for row in 0..rows {
        for col in 0..columns {
            let cell = GridCell::new(row, col);
            let idx = index(rows, columns, row, col);
            let mut inflow = 0u8;
            for n in 0..8 {
                let neighbor = GridCell::new(
                    row + [-1, 0, 1, 1, 1, 0, -1, -1][n],
                    col + [1, 1, 1, 0, -1, -1, -1, 0][n],
                );
                if !kernel.is_in_bounds(neighbor) {
                    continue;
                }
                if let Some(downstream) = kernel.downstream_neighbor(neighbor).unwrap() {
                    if downstream == cell {
                        inflow += 1;
                    }
                }
            }
            manual[idx] = inflow;
        }
    }

    assert_eq!(default_counts, manual);
    assert_eq!(max_proc_serial, manual);
    assert_eq!(max_proc_threaded, manual);
}

#[test]
fn iterative_first_order_link_prune_topology_inflow_count_rejects_mask_geometry_mismatch() {
    let kernel = TopologyKernel::new(1, 2, vec![2u8, 2u8], D8PointerScheme::Whitebox).unwrap();
    let err = kernel
        .inflow_count(GridCell::new(0, 0), &[true])
        .expect_err("mismatched stream mask should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
}

#[test]
fn iterative_first_order_link_prune_topology_rejects_non_finite_cell_size() {
    let (kernel, stream_mask) = synthetic_kernel_with_terminal_head();
    let err = kernel
        .discover_first_order_links_row_major(&stream_mask, f64::NAN, 1.0)
        .expect_err("non-finite cell sizes should fail");
    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("positive and finite"));
}
