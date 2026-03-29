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
//   bit 0     = Black
//   bit n     = number n  (n = 1..=N-2)
//   bits N-1+ = unused
//
// PosMask: which positions can a value still occupy in a row/column?
//   bit n = position n is still available  (n = 0..N-1)

type CellDomain = u64;
type PosMask = u64;

// ── LineState ─────────────────────────────────────────────────────────────────
//
// Per-row or per-column solver state: for each value, which positions are
// still candidates?
//
// black1 and black2 are kept separate because they have different initial
// masks: the first black can only be left of the second, so
//   black1 starts at positions 0..=N-2  (can't occupy the last cell)
//   black2 starts at positions 1..=N-1  (can't occupy the first cell)
//
// numbers has N slots; only indices 0..=N-3 are meaningful (number n maps to
// index n-1). The two trailing slots are unused. Using N-2 as an array length
// would require `#![feature(generic_const_exprs)]` (nightly only), so we
// accept the minor waste and enforce the valid range with a runtime assert in
// positions_for instead.
//
// LineState is Copy because all its fields are u64 / [u64; N].

#[derive(Debug, Clone, Copy)]
struct LineState<const N: usize> {
    black1: PosMask,
    black2: PosMask,
    numbers: [PosMask; N],
}

impl<const N: usize> LineState<N> {
    fn full() -> Self {
        // bits 0..=N-2: (1 << N-1) - 1
        let black1 = (1u64 << (N - 1)) - 1;
        // bits 1..=N-1: all N bits set, then clear bit 0
        let black2 = ((1u64 << N) - 1) & !1u64;
        // numbers start unrestricted
        let all = (1u64 << N) - 1;
        Self {
            black1,
            black2,
            numbers: [all; N],
        }
    }

    // Position mask for number `n` (1-based, matching puzzle values).
    // Panics if n is outside the valid range 1..=N-2.
    fn positions_for(&self, n: u8) -> PosMask {
        assert!(
            n >= 1 && (n as usize) <= N - 2,
            "number {n} out of range for a {N}×{N} grid (valid: 1..={})",
            N - 2
        );
        self.numbers[(n - 1) as usize]
    }
}

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
    row_states: [LineState<N>; N],
    col_states: [LineState<N>; N],
}

impl<const N: usize> SolverState<N> {
    fn new(puzzle: Puzzle<N>) -> Self {
        // All N-1 value bits set: bit 0 (Black) through bit N-2 (number N-2).
        let full_cell: CellDomain = (1u64 << (N - 1)) - 1;
        Self {
            puzzle,
            cell_domains: [[full_cell; N]; N],
            row_states: [LineState::full(); N],
            col_states: [LineState::full(); N],
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

    println!("cell[0][0] domain:      {:05b}", state.cell_domains[0][0]);
    println!("row[0] black1 positions: {:06b}", state.row_states[0].black1);
    println!("row[0] black2 positions: {:06b}", state.row_states[0].black2);
    println!("row[0] positions for 3:  {:06b}", state.row_states[0].positions_for(3));
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn puzzle_new_is_all_empty() {
        let p = Puzzle::<6>::new([0; 6], [0; 6]);
        for row in p.board {
            for cell in row {
                assert_eq!(cell, Cell::Empty);
            }
        }
    }

    #[test]
    fn line_state_black1_excludes_last_position() {
        let ls = LineState::<6>::full();
        // bit 5 (position 5) must be clear — black1 can't be at the last position
        assert_eq!(ls.black1 & (1 << 5), 0);
        // bits 0-4 must all be set
        assert_eq!(ls.black1, 0b011111);
    }

    #[test]
    fn line_state_black2_excludes_first_position() {
        let ls = LineState::<6>::full();
        // bit 0 (position 0) must be clear — black2 can't be at the first position
        assert_eq!(ls.black2 & 1, 0);
        // bits 1-5 must all be set
        assert_eq!(ls.black2, 0b111110);
    }

    #[test]
    fn line_state_numbers_all_positions_open() {
        let ls = LineState::<6>::full();
        for n in 1..=4 {
            assert_eq!(ls.positions_for(n), 0b111111, "failed for number {n}");
        }
    }

    #[test]
    #[should_panic]
    fn line_state_positions_for_out_of_range_panics() {
        let ls = LineState::<6>::full();
        ls.positions_for(5); // 5 is not a valid number in a 6×6 grid (valid: 1–4)
    }

    #[test]
    fn cell_domain_all_values_possible() {
        let state = SolverState::new(Puzzle::new([0; 6], [0; 6]));
        // 5 bits set: Black(0) + numbers 1-4
        assert_eq!(state.cell_domains[0][0], 0b11111);
        assert_eq!(state.cell_domains[5][5], 0b11111);
    }
}
