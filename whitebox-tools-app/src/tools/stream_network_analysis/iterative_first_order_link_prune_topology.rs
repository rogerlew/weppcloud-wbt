use std::collections::{HashMap, HashSet};
use std::io::{Error, ErrorKind};
use std::sync::mpsc;
use std::thread;

use num_cpus;

const D8_DX: [isize; 8] = [1, 1, 1, 0, -1, -1, -1, 0];
const D8_DY: [isize; 8] = [-1, 0, 1, 1, 1, 0, -1, -1];

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum D8PointerScheme {
    Whitebox,
    Esri,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub(crate) struct GridCell {
    pub(crate) row: isize,
    pub(crate) col: isize,
}

impl GridCell {
    pub(crate) const fn new(row: isize, col: isize) -> Self {
        Self { row, col }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub(crate) enum TopologyClass {
    NonStream,
    Head,
    Mid,
    Junction,
    TerminalHead,
    TerminalJunction,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FirstOrderLink {
    pub(crate) source: GridCell,
    pub(crate) receiver: GridCell,
    pub(crate) path: Vec<GridCell>,
    pub(crate) length_m: f64,
    pub(crate) encounter_order: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReceiverCandidateGroup {
    pub(crate) receiver: GridCell,
    pub(crate) candidates: Vec<FirstOrderLink>,
}

pub(crate) struct TopologyKernel {
    rows: isize,
    columns: isize,
    pointers: Vec<u8>,
    pointer_scheme: D8PointerScheme,
}

impl TopologyKernel {
    pub(crate) fn new(
        rows: isize,
        columns: isize,
        pointers: Vec<u8>,
        pointer_scheme: D8PointerScheme,
    ) -> Result<Self, Error> {
        if rows <= 0 || columns <= 0 {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "IFOLP topology kernel requires positive raster dimensions (rows={}, columns={})",
                    rows, columns
                ),
            ));
        }

        let expected_len = (rows * columns) as usize;
        if pointers.len() != expected_len {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "IFOLP topology kernel pointer length mismatch (expected {}, got {})",
                    expected_len,
                    pointers.len()
                ),
            ));
        }

        Ok(Self {
            rows,
            columns,
            pointers,
            pointer_scheme,
        })
    }

    pub(crate) fn decode_pointer_index(&self, pointer_value: u8) -> Result<usize, Error> {
        decode_pointer_index(pointer_value, self.pointer_scheme)
    }

    pub(crate) fn is_in_bounds(&self, cell: GridCell) -> bool {
        cell.row >= 0 && cell.row < self.rows && cell.col >= 0 && cell.col < self.columns
    }

    pub(crate) fn downstream_neighbor(&self, cell: GridCell) -> Result<Option<GridCell>, Error> {
        let idx = self.index_of(cell).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("Grid cell ({}, {}) is out of bounds", cell.row, cell.col),
            )
        })?;

        let pointer_value = self.pointers[idx];
        let direction_index = self.decode_pointer_index(pointer_value)?;
        let next = GridCell::new(
            cell.row + D8_DY[direction_index],
            cell.col + D8_DX[direction_index],
        );

        if self.is_in_bounds(next) {
            Ok(Some(next))
        } else {
            Ok(None)
        }
    }

    pub(crate) fn downstream_stream_neighbor(
        &self,
        cell: GridCell,
        stream_mask: &[bool],
    ) -> Result<Option<GridCell>, Error> {
        self.validate_stream_mask(stream_mask)?;

        if !self.is_stream_cell(stream_mask, cell) {
            return Ok(None);
        }

        match self.downstream_neighbor(cell)? {
            Some(next) if self.is_stream_cell(stream_mask, next) => Ok(Some(next)),
            _ => Ok(None),
        }
    }

    pub(crate) fn upstream_stream_neighbors(
        &self,
        cell: GridCell,
        stream_mask: &[bool],
    ) -> Result<Vec<GridCell>, Error> {
        self.validate_stream_mask(stream_mask)?;

        let mut inflows = Vec::new();
        for n in 0..8 {
            let neighbor = GridCell::new(cell.row + D8_DY[n], cell.col + D8_DX[n]);
            if !self.is_stream_cell(stream_mask, neighbor) {
                continue;
            }

            if let Some(downstream) = self.downstream_neighbor(neighbor)? {
                if downstream == cell {
                    inflows.push(neighbor);
                }
            }
        }

        Ok(inflows)
    }

    pub(crate) fn inflow_count(&self, cell: GridCell, stream_mask: &[bool]) -> Result<u8, Error> {
        self.validate_stream_mask(stream_mask)?;

        if !self.is_stream_cell(stream_mask, cell) {
            return Ok(0);
        }

        self.count_upstream_stream_neighbors(cell, stream_mask)
    }

    pub(crate) fn compute_inflow_counts(&self, stream_mask: &[bool]) -> Result<Vec<u8>, Error> {
        let num_threads = resolve_num_threads(self.rows as usize);
        self.compute_inflow_counts_with_threads(stream_mask, num_threads, 1024)
    }

    #[cfg(test)]
    pub(crate) fn compute_inflow_counts_with_forced_threads_for_tests(
        &self,
        stream_mask: &[bool],
        forced_threads: usize,
    ) -> Result<Vec<u8>, Error> {
        self.compute_inflow_counts_with_threads(stream_mask, forced_threads.max(1), 0)
    }

    fn compute_inflow_counts_with_threads(
        &self,
        stream_mask: &[bool],
        num_threads: usize,
        parallel_min_rows: usize,
    ) -> Result<Vec<u8>, Error> {
        self.validate_stream_mask(stream_mask)?;

        let mut inflow_counts = vec![0u8; stream_mask.len()];
        let rows = self.rows as usize;
        let columns = self.columns as usize;

        if num_threads <= 1 || rows < parallel_min_rows {
            for row in 0..self.rows {
                for col in 0..self.columns {
                    let cell = GridCell::new(row, col);
                    if !self.is_stream_cell(stream_mask, cell) {
                        continue;
                    }

                    let idx = self
                        .index_of(cell)
                        .expect("in-bounds row/col iteration must resolve index");
                    inflow_counts[idx] = self.count_upstream_stream_neighbors(cell, stream_mask)?;
                }
            }
            return Ok(inflow_counts);
        }

        thread::scope(|scope| -> Result<(), Error> {
            let (tx, rx) = mpsc::channel::<Result<(usize, Vec<u8>), Error>>();
            for tid in 0..num_threads {
                let tx_worker = tx.clone();
                scope.spawn(move || {
                    for row in (0..rows).filter(|r| r % num_threads == tid) {
                        let mut row_counts = vec![0u8; columns];
                        for col in 0..columns {
                            let cell = GridCell::new(row as isize, col as isize);
                            if !self.is_stream_cell(stream_mask, cell) {
                                continue;
                            }
                            let inflow =
                                match self.count_upstream_stream_neighbors(cell, stream_mask) {
                                    Ok(value) => value,
                                    Err(err) => {
                                        let _ = tx_worker.send(Err(err));
                                        return;
                                    }
                                };
                            row_counts[col] = inflow;
                        }
                        if tx_worker.send(Ok((row, row_counts))).is_err() {
                            return;
                        }
                    }
                });
            }
            drop(tx);

            let mut rows_received = 0usize;
            while rows_received < rows {
                match rx.recv() {
                    Ok(Ok((row, row_counts))) => {
                        let row_start = row * columns;
                        let row_end = row_start + columns;
                        inflow_counts[row_start..row_end].copy_from_slice(&row_counts);
                        rows_received += 1;
                    }
                    Ok(Err(err)) => return Err(err),
                    Err(_) => {
                        return Err(Error::new(
                            ErrorKind::Other,
                            "IFOLP topology inflow-count worker channel closed unexpectedly",
                        ));
                    }
                }
            }

            Ok(())
        })?;

        Ok(inflow_counts)
    }

    pub(crate) fn classify_topology(
        &self,
        stream_mask: &[bool],
    ) -> Result<Vec<TopologyClass>, Error> {
        self.validate_stream_mask(stream_mask)?;

        let inflow_counts = self.compute_inflow_counts(stream_mask)?;
        let mut classes = vec![TopologyClass::NonStream; stream_mask.len()];
        for row in 0..self.rows {
            for col in 0..self.columns {
                let cell = GridCell::new(row, col);
                let idx = self
                    .index_of(cell)
                    .expect("in-bounds row/col iteration must resolve index");
                if !stream_mask[idx] {
                    continue;
                }

                let inflow_count = inflow_counts[idx];
                let has_downstream = match self.downstream_neighbor(cell)? {
                    Some(next) => self.is_stream_cell(stream_mask, next),
                    None => false,
                };
                classes[idx] = classify_stream_cell(inflow_count, has_downstream);
            }
        }

        Ok(classes)
    }

    pub(crate) fn discover_first_order_links_row_major(
        &self,
        stream_mask: &[bool],
        cell_size_x: f64,
        cell_size_y: f64,
    ) -> Result<Vec<FirstOrderLink>, Error> {
        let topology = self.classify_topology(stream_mask)?;
        let mut links = Vec::new();
        for row in 0..self.rows {
            for col in 0..self.columns {
                let source = GridCell::new(row, col);
                if let Some(mut link) = self.first_order_link_from_source(
                    source,
                    stream_mask,
                    &topology,
                    cell_size_x,
                    cell_size_y,
                )? {
                    link.encounter_order = links.len();
                    links.push(link);
                }
            }
        }

        Ok(links)
    }

    pub(crate) fn first_order_link_from_source(
        &self,
        source: GridCell,
        stream_mask: &[bool],
        topology: &[TopologyClass],
        cell_size_x: f64,
        cell_size_y: f64,
    ) -> Result<Option<FirstOrderLink>, Error> {
        self.validate_stream_mask(stream_mask)?;
        self.validate_topology(topology)?;
        validate_cell_sizes(cell_size_x, cell_size_y)?;

        if !self.is_stream_cell(stream_mask, source) {
            return Ok(None);
        }

        let source_idx = self.index_of(source).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Source cell ({}, {}) is out of bounds",
                    source.row, source.col
                ),
            )
        })?;
        let source_class = topology[source_idx];
        if !is_source_class(source_class) {
            return Ok(None);
        }

        if source_class == TopologyClass::TerminalHead {
            let half_cell_length =
                self.half_step_length_from_pointer(source, cell_size_x, cell_size_y)?;
            return Ok(Some(FirstOrderLink {
                source,
                receiver: source,
                path: vec![source],
                length_m: half_cell_length,
                encounter_order: 0,
            }));
        }

        let mut current = source;
        let mut path = vec![source];
        let mut visited = HashSet::new();
        visited.insert(source);
        let mut length_m = 0.0;

        loop {
            let downstream = match self.downstream_stream_neighbor(current, stream_mask)? {
                Some(cell) => cell,
                None => {
                    return Ok(Some(FirstOrderLink {
                        source,
                        receiver: current,
                        path,
                        length_m,
                        encounter_order: 0,
                    }));
                }
            };

            length_m += step_length_m(current, downstream, cell_size_x, cell_size_y)?;
            path.push(downstream);

            let next_idx = self.index_of(downstream).ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "Encountered out-of-bounds downstream cell ({}, {})",
                        downstream.row, downstream.col
                    ),
                )
            })?;
            if is_receiver_class(topology[next_idx]) {
                return Ok(Some(FirstOrderLink {
                    source,
                    receiver: downstream,
                    path,
                    length_m,
                    encounter_order: 0,
                }));
            }

            if !visited.insert(downstream) {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "Cycle detected while traversing first-order link from source ({}, {})",
                        source.row, source.col
                    ),
                ));
            }

            current = downstream;
        }
    }

    pub(crate) fn candidate_is_still_alive(
        &self,
        stream_mask: &[bool],
        candidate: &FirstOrderLink,
    ) -> Result<bool, Error> {
        self.validate_stream_mask(stream_mask)?;

        if candidate.path.is_empty() {
            return Ok(false);
        }
        if candidate.path[0] != candidate.source {
            return Ok(false);
        }
        if candidate.path[candidate.path.len() - 1] != candidate.receiver {
            return Ok(false);
        }

        for cell in &candidate.path {
            if !self.is_stream_cell(stream_mask, *cell) {
                return Ok(false);
            }
        }

        for window in candidate.path.windows(2) {
            let from = window[0];
            let to = window[1];
            match self.downstream_stream_neighbor(from, stream_mask)? {
                Some(next) if next == to => continue,
                _ => return Ok(false),
            }
        }

        Ok(true)
    }

    fn validate_stream_mask(&self, stream_mask: &[bool]) -> Result<(), Error> {
        if stream_mask.len() != self.pointers.len() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Stream mask length mismatch (expected {}, got {})",
                    self.pointers.len(),
                    stream_mask.len()
                ),
            ));
        }
        Ok(())
    }

    fn validate_topology(&self, topology: &[TopologyClass]) -> Result<(), Error> {
        if topology.len() != self.pointers.len() {
            return Err(Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Topology-class length mismatch (expected {}, got {})",
                    self.pointers.len(),
                    topology.len()
                ),
            ));
        }
        Ok(())
    }

    fn is_stream_cell(&self, stream_mask: &[bool], cell: GridCell) -> bool {
        self.index_of(cell)
            .and_then(|idx| stream_mask.get(idx))
            .copied()
            .unwrap_or(false)
    }

    fn index_of(&self, cell: GridCell) -> Option<usize> {
        if !self.is_in_bounds(cell) {
            return None;
        }
        Some((cell.row * self.columns + cell.col) as usize)
    }

    fn half_step_length_from_pointer(
        &self,
        cell: GridCell,
        cell_size_x: f64,
        cell_size_y: f64,
    ) -> Result<f64, Error> {
        let idx = self.index_of(cell).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                format!(
                    "Terminal-head cell ({}, {}) is out of bounds",
                    cell.row, cell.col
                ),
            )
        })?;
        let pointer_value = self.pointers[idx];
        let direction_index = self.decode_pointer_index(pointer_value)?;
        let next = GridCell::new(
            cell.row + D8_DY[direction_index],
            cell.col + D8_DX[direction_index],
        );
        let full_step = step_length_m(cell, next, cell_size_x, cell_size_y)?;
        Ok(0.5 * full_step)
    }

    fn count_upstream_stream_neighbors(
        &self,
        cell: GridCell,
        stream_mask: &[bool],
    ) -> Result<u8, Error> {
        let mut inflow_count = 0u8;
        for n in 0..8 {
            let neighbor = GridCell::new(cell.row + D8_DY[n], cell.col + D8_DX[n]);
            if !self.is_stream_cell(stream_mask, neighbor) {
                continue;
            }
            if let Some(downstream) = self.downstream_neighbor(neighbor)? {
                if downstream == cell {
                    inflow_count += 1;
                }
            }
        }
        Ok(inflow_count)
    }
}

pub(crate) fn group_links_by_receiver_discovery_order(
    links: &[FirstOrderLink],
) -> Vec<ReceiverCandidateGroup> {
    let mut groups = Vec::<ReceiverCandidateGroup>::new();
    let mut receiver_index = HashMap::<GridCell, usize>::new();

    for link in links {
        if let Some(group_idx) = receiver_index.get(&link.receiver) {
            groups[*group_idx].candidates.push(link.clone());
        } else {
            let group_idx = groups.len();
            receiver_index.insert(link.receiver, group_idx);
            groups.push(ReceiverCandidateGroup {
                receiver: link.receiver,
                candidates: vec![link.clone()],
            });
        }
    }

    groups
}

pub(crate) fn select_shortest_link_strict_epsilon<'a>(
    candidates: &'a [FirstOrderLink],
    epsilon: f64,
) -> Option<&'a FirstOrderLink> {
    let epsilon = epsilon.max(0.0);
    let mut best: Option<&FirstOrderLink> = None;
    for candidate in candidates {
        match best {
            Some(current_best) => {
                if candidate.length_m < current_best.length_m - epsilon {
                    best = Some(candidate);
                }
            }
            None => best = Some(candidate),
        }
    }
    best
}

fn validate_cell_sizes(cell_size_x: f64, cell_size_y: f64) -> Result<(), Error> {
    if !cell_size_x.is_finite()
        || !cell_size_y.is_finite()
        || cell_size_x <= 0.0
        || cell_size_y <= 0.0
    {
        return Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Cell sizes must be positive and finite for IFOLP topology traversal (x={}, y={})",
                cell_size_x, cell_size_y
            ),
        ));
    }
    Ok(())
}

fn step_length_m(
    from: GridCell,
    to: GridCell,
    cell_size_x: f64,
    cell_size_y: f64,
) -> Result<f64, Error> {
    let row_delta = (to.row - from.row).abs();
    let col_delta = (to.col - from.col).abs();

    match (row_delta, col_delta) {
        (0, 1) => Ok(cell_size_x),
        (1, 0) => Ok(cell_size_y),
        (1, 1) => Ok((cell_size_x.powi(2) + cell_size_y.powi(2)).sqrt()),
        _ => Err(Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Non-neighbour traversal from ({}, {}) to ({}, {})",
                from.row, from.col, to.row, to.col
            ),
        )),
    }
}

fn classify_stream_cell(inflow_count: u8, has_downstream_stream: bool) -> TopologyClass {
    if has_downstream_stream {
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
    }
}

fn is_source_class(classification: TopologyClass) -> bool {
    matches!(
        classification,
        TopologyClass::Head | TopologyClass::TerminalHead
    )
}

fn is_receiver_class(classification: TopologyClass) -> bool {
    matches!(
        classification,
        TopologyClass::Junction | TopologyClass::TerminalHead | TopologyClass::TerminalJunction
    )
}

fn decode_pointer_index(
    pointer_value: u8,
    pointer_scheme: D8PointerScheme,
) -> Result<usize, Error> {
    let direction = match pointer_scheme {
        D8PointerScheme::Whitebox => match pointer_value {
            1 => Some(0),
            2 => Some(1),
            4 => Some(2),
            8 => Some(3),
            16 => Some(4),
            32 => Some(5),
            64 => Some(6),
            128 => Some(7),
            _ => None,
        },
        D8PointerScheme::Esri => match pointer_value {
            1 => Some(1),
            2 => Some(2),
            4 => Some(3),
            8 => Some(4),
            16 => Some(5),
            32 => Some(6),
            64 => Some(7),
            128 => Some(0),
            _ => None,
        },
    };

    direction.ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            format!(
                "Invalid D8 pointer value {} for {:?} pointer scheme",
                pointer_value, pointer_scheme
            ),
        )
    })
}

fn resolve_num_threads(row_count: usize) -> usize {
    if row_count == 0 {
        return 1;
    }

    let mut num_procs = num_cpus::get() as isize;
    if let Ok(configs) = whitebox_common::configs::get_configs() {
        let max_procs = configs.max_procs;
        if max_procs > 0 && max_procs < num_procs {
            num_procs = max_procs;
        }
    }

    num_procs.max(1).min(row_count as isize) as usize
}
