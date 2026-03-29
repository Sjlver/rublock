// ── Cell ──────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Cell {
    Empty,
    Black,
    Number(u8), // valid range: 1..=N-2 for an N×N grid
}

// ── Puzzle ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Puzzle<const N: usize> {
    board: [[Cell; N]; N],
    row_targets: [u8; N],
    col_targets: [u8; N],
}

impl<const N: usize> Puzzle<N> {
    fn new(row_targets: [u8; N], col_targets: [u8; N]) -> Self {
        Self {
            board: [[Cell::Empty; N]; N],
            row_targets,
            col_targets,
        }
    }
}

// ── Domain types ──────────────────────────────────────────────────────────────
//
// CellDomain: which values can a cell still hold?
//   bit 0     = unused
//   bit n     = number n  (n = 1..=N-2)
//   bit N-1   = black 1 in row
//   bit N     = black 2 in row
//   bit N+1   = black 1 in column
//   bit N+2   = black 2 in column
//
// We distinguish between various values for black. Black 1 means the first
// black entry in a row (or column), and black 2 is the second.

type CellDomain = u64;

// ── SolverState ───────────────────────────────────────────────────────────────
//
// Working state during search.
//
// Clone but not Copy: cloning is explicit and used for backtracking (save a
// snapshot before committing to a guess). Copy is intentionally absent —
// accidental copies of this large struct would silently produce stale state.

#[derive(Debug, Clone)]
struct SolverState<const N: usize> {
    puzzle: Puzzle<N>,
    cell_domains: [[CellDomain; N]; N],
}

impl<const N: usize> SolverState<N> {
    fn new(puzzle: Puzzle<N>) -> Self {
        // All value bits set: bit 1 through bit N+2.
        let full_cell: CellDomain = ((1u64 << (N + 2)) - 1) << 1;
        Self {
            puzzle,
            cell_domains: [[full_cell; N]; N],
        }
    }
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let puzzle = Puzzle::new(
        [3, 0, 6, 5, 4, 7], // row targets
        [2, 1, 9, 4, 6, 3], // col targets
    );
    let state = SolverState::new(puzzle);

    println!("puzzle: {:?}", state.puzzle);
    println!("cell[0][0] domain:      {:05b}", state.cell_domains[0][0]);
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn puzzle_new_is_all_empty() {
        let p = Puzzle::new([0; 6], [0; 6]);
        for row in p.board {
            for cell in row {
                assert_eq!(cell, Cell::Empty);
            }
        }
    }

    #[test]
    fn cell_domain_all_values_possible() {
        let state = SolverState::new(Puzzle::new([0; 6], [0; 6]));
        // 8 bits set: numbers 1-4 and the four variants of black
        assert_eq!(state.cell_domains[0][0], 0b111111110);
        assert_eq!(state.cell_domains[5][5], 0b111111110);
    }
}
