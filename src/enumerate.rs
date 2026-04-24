use std::collections::VecDeque;

use crate::basic_solver::BasicSolverState;
use crate::grid::{Cell, Grid};
use crate::queue_solver::QueueSolverState;
use crate::solver::Puzzle;

// ── PartialGrid ───────────────────────────────────────────────────────────────

/// Working state for cell-by-cell grid enumeration.
///
/// Cells are filled in row-major order (left to right, top to bottom).
///
/// Each row and column must contain exactly 2 black squares and the digits
/// 1..=(N-2), for a total of N cells per row/column.
#[derive(Clone)]
pub struct PartialGrid<const N: usize> {
    cells: [[Cell; N]; N],
    /// Number of cells filled so far (0..=N*N).
    filled: usize,
    /// Number of black cells placed in each row so far.
    row_black: [u8; N],
    /// Number of black cells placed in each column so far.
    col_black: [u8; N],
    /// Bitmask of digits seen in each row (bit k set ↔ digit k+1 present).
    row_digit_mask: [u64; N],
    /// Bitmask of digits seen in each column.
    col_digit_mask: [u64; N],
}

impl<const N: usize> PartialGrid<N> {
    pub fn new() -> Self {
        Self {
            cells: [[Cell::Empty; N]; N],
            filled: 0,
            row_black: [0; N],
            col_black: [0; N],
            row_digit_mask: [0; N],
            col_digit_mask: [0; N],
        }
    }
}

impl<const N: usize> Default for PartialGrid<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> PartialGrid<N> {
    fn is_complete(&self) -> bool {
        self.filled == N * N
    }

    /// Try placing `value` at the next empty cell (row-major order).
    ///
    /// Returns `Some(new_state)` if the placement is consistent with all
    /// constraints, or `None` if it violates a constraint.
    pub fn try_place(&self, value: Cell) -> Option<Self> {
        let row = self.filled / N;
        let col = self.filled % N;

        // Check the constraint before paying the clone cost.
        match value {
            Cell::Black => {
                if self.row_black[row] >= 2 || self.col_black[col] >= 2 {
                    return None;
                }
            }
            Cell::Number(n) => {
                let bit = 1u64 << (n - 1);
                if self.row_digit_mask[row] & bit != 0 || self.col_digit_mask[col] & bit != 0 {
                    return None;
                }
            }
            Cell::Empty => unreachable!(),
        }

        let mut next = self.clone();
        next.cells[row][col] = value;
        next.filled += 1;
        match value {
            Cell::Black => {
                next.row_black[row] += 1;
                next.col_black[col] += 1;
            }
            Cell::Number(n) => {
                let bit = 1u64 << (n - 1);
                next.row_digit_mask[row] |= bit;
                next.col_digit_mask[col] |= bit;
            }
            Cell::Empty => unreachable!(),
        }

        Some(next)
    }

    #[cfg(test)]
    fn is_consistent(&self) -> bool {
        // Each row and column must have at most two blacks and no repeated digit.
        for r in 0..N {
            let mut black_count: u8 = 0;
            let mut digit_mask: u64 = 0;
            for c in 0..N {
                match self.cells[r][c] {
                    Cell::Empty => {}
                    Cell::Black => {
                        black_count += 1;
                        if black_count > 2 {
                            return false;
                        }
                    }
                    Cell::Number(n) => {
                        let bit = 1u64 << (n - 1);
                        if digit_mask & bit != 0 {
                            return false;
                        }
                        digit_mask |= bit;
                    }
                }
            }
        }

        for c in 0..N {
            let mut black_count: u8 = 0;
            let mut digit_mask: u64 = 0;
            for r in 0..N {
                match self.cells[r][c] {
                    Cell::Empty => {}
                    Cell::Black => {
                        black_count += 1;
                        if black_count > 2 {
                            return false;
                        }
                    }
                    Cell::Number(n) => {
                        let bit = 1u64 << (n - 1);
                        if digit_mask & bit != 0 {
                            return false;
                        }
                        digit_mask |= bit;
                    }
                }
            }
        }

        true
    }
}

/// Iterator over the candidate values to try at each cell: black first, then
/// digits 1..=(N-2).
fn candidates<const N: usize>() -> impl Iterator<Item = Cell> {
    std::iter::once(Cell::Black).chain((1u8..=(N as u8 - 2)).map(Cell::Number))
}

// ── BFS work-queue generation ─────────────────────────────────────────────────

/// Generate partial grids suitable as parallel work items.
///
/// Expands the search tree level by level (BFS) until the queue contains at
/// least `target` items, then returns exactly `target` of them.  Each item in
/// the returned `Vec` represents a
/// distinct subtree that a worker thread can process independently.
///
/// A good `target` is around 100× the number of CPU cores, which gives
/// `rayon`'s work-stealing scheduler enough items to balance uneven subtree
/// sizes while keeping BFS overhead low.
pub fn generate_partial_grids<const N: usize>(
    start: PartialGrid<N>,
    target: usize,
) -> Vec<PartialGrid<N>> {
    let mut queue = VecDeque::from([start]);
    let mut leaves: Vec<PartialGrid<N>> = vec![];

    loop {
        // Stop if we have enough items or every item is already complete.
        if queue.len() + leaves.len() >= target {
            break;
        }

        let current = queue.pop_front();
        match current {
            None => break,
            Some(partial) => {
                if partial.is_complete() {
                    leaves.push(partial);
                } else {
                    for value in candidates::<N>() {
                        if let Some(extended) = partial.try_place(value) {
                            queue.push_back(extended);
                        }
                    }
                }
            }
        }
    }

    leaves.into_iter().chain(queue).take(target).collect()
}

// ── D4 symmetry helpers ───────────────────────────────────────────────────────

/// Generate all 8 elements of the D4 (square symmetry) orbit of a puzzle.
///
/// The symmetries are: identity, transpose, flip-rows, flip-cols, 180° rotation,
/// and the three remaining reflections.  Every valid puzzle is equivalent to
/// exactly one canonical representative — the lex-minimum of its orbit.
fn d4_orbit<const N: usize>(row: [u8; N], col: [u8; N]) -> [([u8; N], [u8; N]); 8] {
    let mut rr = row;
    rr.reverse();
    let mut rc = col;
    rc.reverse();
    [
        (row, col),
        (col, row),
        (rr, col),
        (row, rc),
        (rr, rc),
        (rc, row),
        (col, rr),
        (rc, rr),
    ]
}

/// Return `true` if `(row, col)` is the lex-minimum of its D4 orbit.
///
/// Only canonical puzzles are solved; their orbit size is used as a multiplier
/// to recover the total count.
fn is_canonical<const N: usize>(row: [u8; N], col: [u8; N]) -> bool {
    d4_orbit(row, col)
        .iter()
        .all(|&(r, c)| (row, col) <= (r, c))
}

/// Count distinct elements in the D4 orbit, i.e. the number of symmetrically
/// distinct puzzles represented by this canonical form.
fn orbit_size<const N: usize>(row: [u8; N], col: [u8; N]) -> u64 {
    let mut forms: Vec<_> = d4_orbit(row, col).into_iter().collect();
    forms.sort_unstable();
    forms.dedup();
    forms.len() as u64
}

/// Return `true` if the puzzle derived from `grid` is known to have at least
/// two solutions, without running the full solver.
///
/// For N > 3 every grid has distinct digits appearing exactly once per
/// row and column, so no grid can equal its vertical or horizontal flip.
/// Therefore palindromic row/col targets immediately imply two solutions.
///
/// For N ≤ 3 this shortcut is skipped (degenerate cases).
fn uniqueness_ruled_out<const N: usize>(
    grid: &Grid<N>,
    row_targets: [u8; N],
    col_targets: [u8; N],
) -> bool {
    if N <= 3 {
        return false;
    }
    let mut rev_row = row_targets;
    rev_row.reverse();
    if row_targets == rev_row {
        return true; // V(grid) ≠ grid always, so two solutions exist
    }
    let mut rev_col = col_targets;
    rev_col.reverse();
    if col_targets == rev_col {
        return true; // H(grid) ≠ grid always, so two solutions exist
    }
    if row_targets == col_targets {
        // Self-transposed grids can exist, so we must compare explicitly.
        return grid.transpose() != *grid;
    }
    false
}

/// Which solver backend to use when checking puzzle uniqueness.
#[derive(Clone, Copy)]
pub enum SolverChoice {
    Basic,
    Queue,
}

impl std::fmt::Display for SolverChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolverChoice::Basic => f.write_str("basic"),
            SolverChoice::Queue => f.write_str("queue"),
        }
    }
}

/// Return `true` if the puzzle with the given targets has exactly one solution.
fn is_valid_puzzle<const N: usize>(
    row_targets: [u8; N],
    col_targets: [u8; N],
    solver: SolverChoice,
) -> bool {
    let puzzle = Puzzle::new(row_targets, col_targets);
    match solver {
        SolverChoice::Basic => BasicSolverState::new(puzzle).count_solutions(2) == 1,
        SolverChoice::Queue => QueueSolverState::new(puzzle).count_solutions(2) == 1,
    }
}

// ── Per-work-item DFS ─────────────────────────────────────────────────────────

/// Count the complete grids and valid puzzles reachable from `partial`.
///
/// Returns `(total_grids, valid_puzzle_grids)`.
///
/// This is the unit of parallel work: call it from `par_iter` on the items
/// returned by `generate_partial_grids`.
pub fn count_from_partial<const N: usize>(
    partial: &PartialGrid<N>,
    solver: SolverChoice,
) -> (u64, u64) {
    let mut total = 0u64;
    let mut valid = 0u64;
    dfs(partial, &mut total, &mut valid, solver);
    (total, valid)
}

fn dfs<const N: usize>(
    partial: &PartialGrid<N>,
    total: &mut u64,
    valid: &mut u64,
    solver: SolverChoice,
) {
    if partial.is_complete() {
        *total += 1;
        let grid = Grid {
            cells: partial.cells,
        };
        let (row_targets, col_targets) = grid.compute_targets();
        if is_canonical(row_targets, col_targets)
            && !uniqueness_ruled_out(&grid, row_targets, col_targets)
            && is_valid_puzzle(row_targets, col_targets, solver)
        {
            *valid += orbit_size(row_targets, col_targets);
        }
        return;
    }

    for value in candidates::<N>() {
        if let Some(next) = partial.try_place(value) {
            dfs(&next, total, valid, solver);
        }
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_partial_grids_stops_at_target() {
        let partials = generate_partial_grids(PartialGrid::<5>::new(), 50);
        assert!(partials.len() == 50);
    }

    #[test]
    fn generate_partial_grids_all_items_consistent() {
        // Verify that every returned partial satisfies the row/column
        // invariants for the cells that have been filled.
        let partials = generate_partial_grids(PartialGrid::<5>::new(), 200);
        for p in &partials {
            assert!(p.is_consistent());
        }
    }

    #[test]
    fn is_consistent_accepts_partial_grid() {
        let mut g = PartialGrid::<6>::new();
        g.cells[0][0] = Cell::Black;
        g.cells[0][1] = Cell::Number(1);
        g.cells[0][2] = Cell::Number(2);

        assert!(g.is_consistent());
    }

    #[test]
    fn is_consistent_rejects_three_blacks() {
        let mut g = PartialGrid::<6>::new();
        g.cells[0][0] = Cell::Black;
        assert!(g.is_consistent());

        g.cells[0][1] = Cell::Black;
        assert!(g.is_consistent());

        g.cells[0][2] = Cell::Black;
        assert!(!g.is_consistent());
    }

    #[test]
    fn solver_count_solutions_matches_brute_force() {
        use crate::basic_solver::BasicSolverState;
        use crate::solver::Puzzle;
        use std::collections::HashMap;

        const N: usize = 4;

        // Collect all valid NxN grids via DFS.
        fn collect_all<const N: usize>(partial: &PartialGrid<N>, out: &mut Vec<[[Cell; N]; N]>) {
            if partial.is_complete() {
                out.push(partial.cells);
                return;
            }
            for v in candidates::<N>() {
                if let Some(next) = partial.try_place(v) {
                    collect_all(&next, out);
                }
            }
        }
        let start = PartialGrid::<N>::new();
        let mut raw_grids: Vec<[[Cell; N]; N]> = Vec::new();
        collect_all(&start, &mut raw_grids);

        // Group grid indices by their puzzle targets.
        let mut by_targets: HashMap<([u8; N], [u8; N]), usize> = HashMap::new();
        for cells in &raw_grids {
            let grid = Grid { cells: *cells };
            let targets = grid.compute_targets();
            *by_targets.entry(targets).or_insert(0) += 1;
        }

        // For every unique target set, the brute-force count is the number of
        // grids that share those targets.  The solver must agree.
        let mut mismatches: Vec<([u8; N], [u8; N], usize, usize)> = Vec::new();
        for ((row_targets, col_targets), brute_count) in &by_targets {
            let puzzle = Puzzle::new(*row_targets, *col_targets);
            let state = BasicSolverState::new(puzzle);
            let solver_count = state.count_solutions(brute_count + 1);
            if solver_count != *brute_count {
                mismatches.push((*row_targets, *col_targets, *brute_count, solver_count));
            }
        }

        if !mismatches.is_empty() {
            for (row_targets, col_targets, expected, got) in &mismatches {
                eprintln!(
                    "MISMATCH row={:?} col={:?}  brute_force={expected}  solver={got}",
                    row_targets, col_targets
                );
            }
            panic!(
                "{} mismatch(es) — solver count differs from brute force",
                mismatches.len()
            );
        }
    }

    /// Verify that the orbit-multiplied valid count from count_from_partial
    /// matches the brute-force count of distinct puzzles with a unique solution.
    #[test]
    fn valid_puzzle_count_matches_brute_force_with_symmetry() {
        use std::collections::HashMap;

        const N: usize = 4;

        fn collect_all<const N: usize>(partial: &PartialGrid<N>, out: &mut Vec<[[Cell; N]; N]>) {
            if partial.is_complete() {
                out.push(partial.cells);
                return;
            }
            for v in candidates::<N>() {
                if let Some(next) = partial.try_place(v) {
                    collect_all(&next, out);
                }
            }
        }

        // Brute-force: count distinct (row_targets, col_targets) pairs with
        // exactly one solution (= exactly one grid with those targets).
        let mut raw_grids: Vec<[[Cell; N]; N]> = Vec::new();
        collect_all(&PartialGrid::<N>::new(), &mut raw_grids);
        let mut by_targets: HashMap<([u8; N], [u8; N]), usize> = HashMap::new();
        for cells in &raw_grids {
            let grid = Grid { cells: *cells };
            let targets = grid.compute_targets();
            *by_targets.entry(targets).or_insert(0) += 1;
        }
        let brute_force_valid: u64 = by_targets.values().filter(|&&c| c == 1).count() as u64;

        // New code: count_from_partial with D4 symmetry multiplier.
        let (_, valid) = count_from_partial(&PartialGrid::<N>::new(), SolverChoice::Queue);

        assert_eq!(
            valid, brute_force_valid,
            "symmetry-aware count {valid} != brute-force {brute_force_valid}"
        );
    }

    #[test]
    fn is_consistent_rejects_duplicate_values_in_col() {
        let mut g = PartialGrid::<6>::new();
        g.cells[0][0] = Cell::Black;
        assert!(g.is_consistent());

        g.cells[1][0] = Cell::Black;
        assert!(g.is_consistent());

        g.cells[2][0] = Cell::Number(1);
        assert!(g.is_consistent());

        // A different row/column should not cause this to be inconsistent
        g.cells[5][5] = Cell::Number(1);
        assert!(g.is_consistent());

        // ... but the same column does
        g.cells[3][0] = Cell::Number(1);
        assert!(!g.is_consistent());
    }
}
