use crate::solver::{Puzzle, SolverState};

// ── Cell ──────────────────────────────────────────────────────────────────────

/// The value of a single cell in a fully or partially filled grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    /// The cell has not been assigned yet (used during grid enumeration).
    Empty,
    /// The cell is one of the two black squares in its row and column.
    Black,
    /// The cell holds a digit from 1 to 4.
    Number(u8),
}

// ── Grid ──────────────────────────────────────────────────────────────────────

/// A fully filled 6×6 grid: every cell is either `Black` or `Number(1..=4)`.
///
/// Each row and each column must contain exactly two black squares and the
/// digits 1, 2, 3, 4 — this is the structural invariant that the grid
/// enumerator maintains, and that `compute_targets` relies on.
#[derive(Debug, Clone)]
pub struct Grid {
    pub cells: [[Cell; 6]; 6],
}

impl Grid {
    /// Compute the row and column targets for this filled grid.
    ///
    /// The target for a row (or column) is the sum of the numbers that lie
    /// strictly between the two black squares.  Adjacent blacks give a target
    /// of zero.
    ///
    /// Panics if any cell is `Empty`, or if a row or column does not have
    /// exactly two black squares.
    pub fn compute_targets(&self) -> ([u8; 6], [u8; 6]) {
        let mut row_targets = [0u8; 6];
        let mut col_targets = [0u8; 6];

        for r in 0..6 {
            let (b1, b2) = black_pair_in_row(&self.cells[r], r);
            row_targets[r] = (b1 + 1..b2)
                .map(|c| match self.cells[r][c] {
                    Cell::Number(n) => n,
                    other => panic!("unexpected {other:?} between blacks at ({r}, {c})"),
                })
                .sum();
        }

        for c in 0..6 {
            let col: [Cell; 6] = std::array::from_fn(|r| self.cells[r][c]);
            let (b1, b2) = black_pair_in_row(&col, c);
            col_targets[c] = (b1 + 1..b2)
                .map(|r| match self.cells[r][c] {
                    Cell::Number(n) => n,
                    other => panic!("unexpected {other:?} between blacks at ({r}, {c})"),
                })
                .sum();
        }

        (row_targets, col_targets)
    }

    /// Return `true` if this grid constitutes a valid puzzle.
    ///
    /// A puzzle is *valid* when the targets derived from the grid's black
    /// placement and digit arrangement lead to exactly one solution — i.e.,
    /// a solver given only the 12 target numbers can reconstruct this grid
    /// uniquely.
    pub fn is_valid_puzzle(&self) -> bool {
        let (row_targets, col_targets) = self.compute_targets();
        let puzzle = Puzzle::new(row_targets, col_targets);
        let state = SolverState::new(puzzle);
        state.count_solutions(2) == 1
    }
}

/// Find the positions of the two black squares in a row (or column) of cells.
///
/// Panics if the row does not contain exactly two `Black` cells.
fn black_pair_in_row(cells: &[Cell; 6], index: usize) -> (usize, usize) {
    let blacks: Vec<usize> = (0..6).filter(|&i| cells[i] == Cell::Black).collect();
    assert_eq!(
        blacks.len(),
        2,
        "row/col {index}: expected 2 black squares, found {}",
        blacks.len()
    );
    (blacks[0], blacks[1])
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_grid(rows: &[[u8; 6]; 6]) -> Grid {
        // 0 = Black, 1-4 = Number
        let cells = std::array::from_fn(|r| {
            std::array::from_fn(|c| match rows[r][c] {
                0 => Cell::Black,
                n => Cell::Number(n),
            })
        });
        Grid { cells }
    }

    #[test]
    fn compute_targets_matches_newspaper_puzzle_1() {
        // This is the unique solution to newspaper puzzle 1
        // (row targets [8,2,3,8,9,0], col targets [0,0,5,9,0,4]).
        // We reconstruct the grid and verify that compute_targets recovers
        // those exact targets.
        //
        // Row 5 has blacks at columns 2 and 3 (adjacent) → row target 0.
        // Columns 0 and 1 have blacks at rows 3,4 and 0,1 respectively
        // (both adjacent pairs) → col targets 0 and 0.
        let grid = make_grid(&[
            [2, 0, 3, 1, 4, 0],
            [1, 0, 2, 0, 3, 4],
            [3, 4, 0, 2, 1, 0],
            [0, 3, 1, 4, 0, 2],
            [0, 2, 4, 3, 0, 1],
            [4, 1, 0, 0, 2, 3],
        ]);
        let (row_targets, col_targets) = grid.compute_targets();
        assert_eq!(row_targets, [8, 2, 3, 8, 9, 0]);
        assert_eq!(col_targets, [0, 0, 5, 9, 0, 4]);
    }

    #[test]
    fn is_valid_puzzle_for_known_good_grid() {
        // Newspaper puzzle 1's solution (from the solver integration test).
        // Blacks at: row 0 → cols 1,5; row 1 → cols 1,3; etc.
        // We verify that it's recognised as a valid (unique-solution) puzzle.
        let grid = make_grid(&[
            [2, 0, 3, 1, 4, 0],
            [1, 0, 2, 0, 3, 4],
            [3, 4, 0, 2, 1, 0],
            [0, 3, 1, 4, 0, 2],
            [0, 2, 4, 3, 0, 1],
            [4, 1, 0, 0, 2, 3],
        ]);
        assert!(grid.is_valid_puzzle());
    }
}
