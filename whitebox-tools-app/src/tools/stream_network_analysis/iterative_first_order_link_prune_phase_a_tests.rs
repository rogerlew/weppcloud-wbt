use super::iterative_first_order_link_prune_phase_a::{run_phase_a_qualification, PhaseAInputs};
use super::iterative_first_order_link_prune_topology::{D8PointerScheme, GridCell, TopologyClass};
use std::io::ErrorKind;

fn index(rows: isize, columns: isize, row: isize, col: isize) -> usize {
    assert!(row >= 0 && row < rows);
    assert!(col >= 0 && col < columns);
    (row * columns + col) as usize
}

fn phase_a_inputs(
    rows: isize,
    columns: isize,
    pointers: Vec<u8>,
    upstream_area_cells: Vec<f64>,
    local_csa_cells: Vec<f64>,
    min_active_csa_cells: f64,
) -> PhaseAInputs {
    let expected_len = (rows * columns) as usize;
    assert_eq!(pointers.len(), expected_len);
    assert_eq!(upstream_area_cells.len(), expected_len);
    assert_eq!(local_csa_cells.len(), expected_len);

    PhaseAInputs {
        rows,
        columns,
        pointers,
        pointer_scheme: D8PointerScheme::Whitebox,
        upstream_area_cells,
        active_mask: vec![true; expected_len],
        local_csa_cells,
        min_active_csa_cells,
        epsilon: 1e-5,
    }
}

fn y_network_pointers() -> (isize, isize, Vec<u8>) {
    let rows = 3;
    let columns = 3;
    let mut pointers = vec![2u8; (rows * columns) as usize];
    pointers[index(rows, columns, 0, 0)] = 4; // SE -> (1,1)
    pointers[index(rows, columns, 0, 2)] = 16; // SW -> (1,1)
    pointers[index(rows, columns, 1, 1)] = 8; // S -> (2,1)
    pointers[index(rows, columns, 2, 1)] = 8; // S -> off-grid
    (rows, columns, pointers)
}

#[test]
fn iterative_first_order_link_prune_phase_a_rejects_source_and_promotes_downstream_head() {
    let rows = 1;
    let columns = 4;
    let pointers = vec![2u8, 2u8, 2u8, 2u8];
    let upstream_area_cells = vec![3.0, 5.0, 6.0, 7.0];
    let local_csa_cells = vec![6.0, 2.0, 2.0, 2.0];

    let result = run_phase_a_qualification(&phase_a_inputs(
        rows,
        columns,
        pointers,
        upstream_area_cells,
        local_csa_cells,
        2.0,
    ))
    .expect("phase A should succeed");

    assert!(!result.stream_mask[index(rows, columns, 0, 0)]);
    assert!(result.stream_mask[index(rows, columns, 0, 1)]);
    assert_eq!(
        result.topology[index(rows, columns, 0, 1)],
        TopologyClass::Head
    );
}

#[test]
fn iterative_first_order_link_prune_phase_a_handles_junction_collapse() {
    let (rows, columns, pointers) = y_network_pointers();
    let mut upstream_area_cells = vec![0.0f64; (rows * columns) as usize];
    let mut local_csa_cells = vec![1.0f64; (rows * columns) as usize];

    upstream_area_cells[index(rows, columns, 0, 0)] = 1.0; // removed source
    upstream_area_cells[index(rows, columns, 0, 2)] = 5.0; // surviving source
    upstream_area_cells[index(rows, columns, 1, 1)] = 5.0; // receiver remains
    upstream_area_cells[index(rows, columns, 2, 1)] = 6.0; // outlet path

    local_csa_cells[index(rows, columns, 0, 0)] = 2.0;
    local_csa_cells[index(rows, columns, 0, 2)] = 2.0;
    local_csa_cells[index(rows, columns, 1, 1)] = 2.0;
    local_csa_cells[index(rows, columns, 2, 1)] = 1.0;

    let result = run_phase_a_qualification(&phase_a_inputs(
        rows,
        columns,
        pointers,
        upstream_area_cells,
        local_csa_cells,
        1.0,
    ))
    .expect("phase A should succeed");

    assert!(!result.stream_mask[index(rows, columns, 0, 0)]);
    assert!(result.stream_mask[index(rows, columns, 1, 1)]);
    assert_eq!(
        result.topology[index(rows, columns, 1, 1)],
        TopologyClass::Mid
    );
}

#[test]
fn iterative_first_order_link_prune_phase_a_terminal_receiver_recheck_can_remove_receiver() {
    let rows = 1;
    let columns = 4;
    let pointers = vec![2u8, 2u8, 2u8, 2u8];
    let upstream_area_cells = vec![2.0, 3.0, 1.0, 1.0];
    let local_csa_cells = vec![5.0, 4.0, 2.0, 1.0];

    let result = run_phase_a_qualification(&phase_a_inputs(
        rows,
        columns,
        pointers,
        upstream_area_cells,
        local_csa_cells,
        1.0,
    ))
    .expect("phase A should succeed");

    assert!(!result.stream_mask[index(rows, columns, 0, 0)]);
    assert!(!result.stream_mask[index(rows, columns, 0, 1)]);
    assert!(!result.stream_mask[index(rows, columns, 0, 2)]);
    assert!(result.stream_mask[index(rows, columns, 0, 3)]);
}

#[test]
fn iterative_first_order_link_prune_phase_a_fails_when_no_channels_exist_after_provisional_mask() {
    let rows = 1;
    let columns = 2;
    let pointers = vec![2u8, 2u8];
    let upstream_area_cells = vec![1.0, 2.0];
    let local_csa_cells = vec![5.0, 5.0];

    let err = run_phase_a_qualification(&phase_a_inputs(
        rows,
        columns,
        pointers,
        upstream_area_cells,
        local_csa_cells,
        5.0,
    ))
    .expect_err("phase A should fail with no channels");

    assert_eq!(err.kind(), ErrorKind::InvalidInput);
    assert!(err.to_string().contains("No channels remain"));
}

#[test]
fn iterative_first_order_link_prune_phase_a_traversal_cadence_is_deterministic() {
    let (rows, columns, pointers) = y_network_pointers();
    let mut upstream_area_cells = vec![0.0f64; (rows * columns) as usize];
    let mut local_csa_cells = vec![1.0f64; (rows * columns) as usize];

    upstream_area_cells[index(rows, columns, 0, 0)] = 1.0;
    upstream_area_cells[index(rows, columns, 0, 2)] = 1.0;
    upstream_area_cells[index(rows, columns, 1, 1)] = 1.0;
    upstream_area_cells[index(rows, columns, 2, 1)] = 10.0;

    local_csa_cells[index(rows, columns, 0, 0)] = 2.0;
    local_csa_cells[index(rows, columns, 0, 2)] = 2.0;
    local_csa_cells[index(rows, columns, 1, 1)] = 2.0;
    local_csa_cells[index(rows, columns, 2, 1)] = 1.0;

    let result = run_phase_a_qualification(&phase_a_inputs(
        rows,
        columns,
        pointers,
        upstream_area_cells,
        local_csa_cells,
        1.0,
    ))
    .expect("phase A should succeed");

    assert_eq!(result.pass_traces.len(), 2);
    assert_eq!(
        result.pass_traces[0].scanned_sources,
        vec![
            GridCell::new(0, 0),
            GridCell::new(0, 2),
            GridCell::new(2, 1)
        ]
    );
    assert_eq!(
        result.pass_traces[1].scanned_sources,
        vec![GridCell::new(2, 1)]
    );
}
