use crate::grid::{Cell, Grid};

// ── PartialGrid ───────────────────────────────────────────────────────────────

/// Working state for cell-by-cell grid enumeration.
///
/// Cells are filled in row-major order (left to right, top to bottom).
/// Per-row and per-column constraint counts are updated incrementally so
/// that invalid branches can be pruned as soon as a constraint is violated.
///
/// Each row and column must contain exactly 2 black squares and the digits
/// 1..=(N-2), for a total of N cells per row/column.
#[derive(Clone)]
pub struct PartialGrid<const N: usize> {
    cells: [[Cell; N]; N],
    /// Number of cells filled so far (0..=N*N).
    filled: usize,
    /// Number of black squares placed in each row so far.
    row_black: [u8; N],
    /// Bitmask of digits placed in each row so far.
    /// Bit `k` (0-indexed from LSB) is set when digit `k + 1` has been used.
    row_digit_mask: [u8; N],
    /// Number of black squares placed in each column so far.
    col_black: [u8; N],
    /// Bitmask of digits placed in each column so far.
    col_digit_mask: [u8; N],
}

impl<const N: usize> PartialGrid<N> {
    pub fn new() -> Self {
        Self {
            cells: [[Cell::Empty; N]; N],
            filled: 0,
            row_black: [0; N],
            row_digit_mask: [0; N],
            col_black: [0; N],
            col_digit_mask: [0; N],
        }
    }

    fn is_complete(&self) -> bool {
        self.filled == N * N
    }

    /// Try placing `value` at the next empty cell (row-major order).
    ///
    /// Returns `Some(new_state)` if the placement is consistent with all
    /// constraints, or `None` if it violates a constraint.
    ///
    /// After placing, a look-ahead checks that the remaining cells in the
    /// current row and column can still satisfy the grid invariants:
    /// exactly two black squares and digits 1..=(N-2) in every row and column.
    fn try_place(&self, value: Cell) -> Option<Self> {
        let row = self.filled / N;
        let col = self.filled % N;

        let mut next = self.clone();
        next.cells[row][col] = value;
        next.filled += 1;

        // Update constraint counts and check for immediate violations.
        match value {
            Cell::Black => {
                next.row_black[row] += 1;
                next.col_black[col] += 1;
                if next.row_black[row] > 2 || next.col_black[col] > 2 {
                    return None;
                }
            }
            Cell::Number(n) => {
                let bit = 1u8 << (n - 1);
                if next.row_digit_mask[row] & bit != 0 || next.col_digit_mask[col] & bit != 0 {
                    return None; // digit already used in this row or column
                }
                next.row_digit_mask[row] |= bit;
                next.col_digit_mask[col] |= bit;
            }
            Cell::Empty => unreachable!("try_place called with Empty"),
        }

        // Look-ahead for the current row.
        // The remaining cells (positions col+1..=N-1) must be able to supply
        // the blacks and digits that are still missing.
        let cells_left_in_row = (N - 1) - col;
        let row_blacks_needed = 2u8.saturating_sub(next.row_black[row]) as usize;
        let row_digits_needed = ((N as u32 - 2) - next.row_digit_mask[row].count_ones()) as usize;
        if row_blacks_needed + row_digits_needed > cells_left_in_row {
            return None;
        }

        // Look-ahead for the current column.
        // The remaining rows (positions row+1..=N-1) must satisfy the same.
        let rows_left_in_col = (N - 1) - row;
        let col_blacks_needed = 2u8.saturating_sub(next.col_black[col]) as usize;
        let col_digits_needed = ((N as u32 - 2) - next.col_digit_mask[col].count_ones()) as usize;
        if col_blacks_needed + col_digits_needed > rows_left_in_col {
            return None;
        }

        Some(next)
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
/// least `target` items.  Each item in the returned `Vec` represents a
/// distinct subtree that a worker thread can process independently.
///
/// A good `target` is around 100× the number of CPU cores, which gives
/// `rayon`'s work-stealing scheduler enough items to balance uneven subtree
/// sizes while keeping BFS overhead low.
pub fn generate_partial_grids<const N: usize>(target: usize) -> Vec<PartialGrid<N>> {
    let mut queue = vec![PartialGrid::new()];

    loop {
        // Stop if we have enough items or every item is already complete.
        if queue.len() >= target || queue.iter().all(|p| p.is_complete()) {
            break;
        }

        let prev = std::mem::take(&mut queue);
        for partial in prev {
            if partial.is_complete() {
                // Leaf node: keep it as-is; it will be counted in the DFS phase.
                queue.push(partial);
            } else {
                for value in candidates::<N>() {
                    if let Some(extended) = partial.try_place(value) {
                        queue.push(extended);
                    }
                }
            }
        }
    }

    queue
}

// ── Per-work-item DFS ─────────────────────────────────────────────────────────

/// Count the complete grids and valid puzzles reachable from `partial`.
///
/// Returns `(total_grids, valid_puzzle_grids)`.
///
/// This is the unit of parallel work: call it from `par_iter` on the items
/// returned by `generate_partial_grids`.
pub fn count_from_partial<const N: usize>(partial: &PartialGrid<N>) -> (u64, u64) {
    let mut total = 0u64;
    let mut valid = 0u64;
    dfs(partial, &mut total, &mut valid);
    (total, valid)
}

fn dfs<const N: usize>(partial: &PartialGrid<N>, total: &mut u64, valid: &mut u64) {
    if partial.is_complete() {
        *total += 1;
        let grid = Grid {
            cells: partial.cells,
        };
        if grid.is_valid_puzzle() {
            *valid += 1;
        }
        return;
    }

    for value in candidates::<N>() {
        if let Some(next) = partial.try_place(value) {
            dfs(&next, total, valid);
        }
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_partial_grids_stops_at_target() {
        let partials = generate_partial_grids::<5>(50);
        assert!(partials.len() >= 50);
    }

    #[test]
    fn generate_partial_grids_all_items_consistent() {
        // Verify that every returned partial satisfies the row/column
        // invariants for the cells that have been filled.
        // N=5: each row has 2 blacks and digits 1,2,3 (mask = 0b111 = 7).
        let partials = generate_partial_grids::<5>(200);
        let full_digit_mask: u8 = (1 << (5u8 - 2)) - 1; // 0b111
        for p in &partials {
            let row = p.filled / 5;
            // All completed rows must be fully valid.
            for r in 0..row {
                assert_eq!(p.row_black[r], 2, "row {r} black count");
                assert_eq!(p.row_digit_mask[r], full_digit_mask, "row {r} digits");
            }
            // The partial row (if any) must not be over-committed.
            if p.filled % 5 != 0 {
                assert!(p.row_black[row] <= 2);
                assert_eq!(p.row_digit_mask[row] & !full_digit_mask, 0);
            }
        }
    }

    // `count_from_partial` is exercised end-to-end by the `enumerate` binary.
    // A unit test starting from a shallow partial would need to run the full
    // DFS over millions of grids, which is too slow for `cargo test`.
}
