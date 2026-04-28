use rand::seq::SliceRandom;

// ── Cell ──────────────────────────────────────────────────────────────────────

/// The value of a single cell in a fully or partially filled grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cell {
    /// The cell has not been assigned yet (used during grid enumeration).
    Empty,
    /// The cell is one of the two black squares in its row and column.
    Black,
    /// The cell holds a digit from 1 to N-2.
    Number(u8),
}

// ── Grid ──────────────────────────────────────────────────────────────────────

/// A fully filled N×N grid: every cell is either `Black` or `Number(1..=N-2)`.
///
/// Each row and each column must contain exactly two black squares and the
/// digits 1..=(N-2) — this is the structural invariant that the grid
/// enumerator maintains, and that `compute_targets` relies on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grid<const N: usize> {
    pub cells: [[Cell; N]; N],
}

impl<const N: usize> Grid<N> {
    /// Compute the row and column targets for this filled grid.
    ///
    /// The target for a row (or column) is the sum of the numbers that lie
    /// strictly between the two black squares.  Adjacent blacks give a target
    /// of zero.
    ///
    /// Panics if any cell is `Empty`, or if a row or column does not have
    /// exactly two black squares.
    pub fn compute_targets(&self) -> ([u8; N], [u8; N]) {
        let mut row_targets = [0u8; N];
        let mut col_targets = [0u8; N];

        for (r, target) in row_targets.iter_mut().enumerate() {
            let (b1, b2) = black_pair_in_row(&self.cells[r]);
            *target = (b1 + 1..b2)
                .map(|c| match self.cells[r][c] {
                    Cell::Number(n) => n,
                    other => panic!("unexpected {other:?} between blacks at ({r}, {c})"),
                })
                .sum();
        }

        for (c, target) in col_targets.iter_mut().enumerate() {
            let col: [Cell; N] = std::array::from_fn(|r| self.cells[r][c]);
            let (b1, b2) = black_pair_in_row(&col);
            *target = (b1 + 1..b2)
                .map(|r| match self.cells[r][c] {
                    Cell::Number(n) => n,
                    other => panic!("unexpected {other:?} between blacks at ({r}, {c})"),
                })
                .sum();
        }

        (row_targets, col_targets)
    }

    /// Return a new grid with rows and columns swapped.
    pub fn transpose(&self) -> Self {
        Grid {
            cells: std::array::from_fn(|r| std::array::from_fn(|c| self.cells[c][r])),
        }
    }
}

/// Find the positions of the two black squares in a row (or column) of cells.
///
/// Panics if the row does not contain exactly two `Black` cells.
fn black_pair_in_row<const N: usize>(cells: &[Cell; N]) -> (usize, usize) {
    let mut iter = (0..N).filter(|&i| cells[i] == Cell::Black);
    let b1 = iter.next().expect("expected a black square");
    let b2 = iter.next().expect("expected a second black square");
    assert_eq!(iter.next(), None, "expected exactly two black squares");

    (b1, b2)
}

// ── Random grid generation ────────────────────────────────────────────────────

/// Fill an N×N grid with a randomly-chosen valid arrangement of digits and
/// blacks (two blacks per row/column, each digit 1..=N-2 appearing exactly
/// once per row/column).  Used by `gen_puzzle` and `compare` to seed solvable
/// puzzles by deriving targets via [`Grid::compute_targets`].
///
/// Retries internally on dead ends — always returns `Some` eventually.
pub fn random_grid<const N: usize>(rng: &mut impl rand::Rng) -> Grid<N> {
    loop {
        let mut cells = [[Cell::Empty; N]; N];
        if let Some(grid) = dfs::<N>(&mut cells, 0, rng) {
            return grid;
        }
    }
}

// Attempt to fill `cells` from `pos` onward, trying candidates in a random
// order at each position. Returns `Some(Grid)` if a complete grid was reached,
// or `None` if every candidate at some position was exhausted (dead end).
fn dfs<const N: usize>(
    cells: &mut [[Cell; N]; N],
    pos: usize,
    rng: &mut impl rand::Rng,
) -> Option<Grid<N>> {
    if pos == N * N {
        return Some(Grid { cells: *cells });
    }

    let row = pos / N;
    let col = pos % N;

    let row_blacks = (0..col).filter(|&c| cells[row][c] == Cell::Black).count();
    let col_blacks = (0..row).filter(|&r| cells[r][col] == Cell::Black).count();
    let row_digit_mask: u64 = (0..col)
        .filter_map(|c| {
            if let Cell::Number(n) = cells[row][c] {
                Some(1u64 << n)
            } else {
                None
            }
        })
        .fold(0, |a, b| a | b);
    let col_digit_mask: u64 = (0..row)
        .filter_map(|r| {
            if let Cell::Number(n) = cells[r][col] {
                Some(1u64 << n)
            } else {
                None
            }
        })
        .fold(0, |a, b| a | b);

    let digits: u8 = (N - 2) as u8;
    let mut candidates: Vec<Cell> = std::iter::once(Cell::Black)
        .chain((1..=digits).map(Cell::Number))
        .filter(|&c| match c {
            Cell::Black => row_blacks < 2 && col_blacks < 2,
            Cell::Number(d) => {
                let bit = 1u64 << d;
                row_digit_mask & bit == 0 && col_digit_mask & bit == 0
            }
            Cell::Empty => unreachable!(),
        })
        .collect();

    candidates.shuffle(rng);

    for candidate in candidates {
        cells[row][col] = candidate;
        if let Some(grid) = dfs(cells, pos + 1, rng) {
            return Some(grid);
        }
    }

    cells[row][col] = Cell::Empty;
    None
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_grid(rows: &[[u8; 6]; 6]) -> Grid<6> {
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
}
