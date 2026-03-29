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

// ── Solver rules (N = 6) ─────────────────────────────────────────────────────
//
// This impl block is intentionally specialised for 6×6.  Once the rules are
// correct and tested, they can be generalised and moved to the generic
// `impl<const N: usize> SolverState<N>` block above.

impl SolverState<6> {
    // Bit positions for the "black" value variants, specific to N = 6.
    // (The general formula is in the CellDomain layout comment above.)
    const BLACK1_ROW: u64 = 1 << 5; // N - 1
    const BLACK2_ROW: u64 = 1 << 6; // N
    const BLACK1_COL: u64 = 1 << 7; // N + 1
    const BLACK2_COL: u64 = 1 << 8; // N + 2

    // Maximum sum achievable by taking the k largest numbers from {1, 2, 3, 4}.
    // For N = 6 there are N - 2 = 4 number-cells per line, so k ranges 0..=4.
    const MAX_SUM: [u8; 5] = [0, 4, 7, 9, 10];

    // ── Low-level helper ─────────────────────────────────────────────────────

    /// Clear one domain bit.  Returns `true` iff the domain actually shrank.
    fn clear_bit(&mut self, row: usize, col: usize, bit: u64) -> bool {
        let before = self.cell_domains[row][col];
        self.cell_domains[row][col] = before & !bit;
        self.cell_domains[row][col] != before
    }

    /// Clear all bits in `mask` from a cell's domain.  Returns `true` iff any
    /// bit was actually set before (i.e. the domain shrank).
    fn clear_mask(&mut self, row: usize, col: usize, mask: u64) -> bool {
        let before = self.cell_domains[row][col];
        self.cell_domains[row][col] = before & !mask;
        self.cell_domains[row][col] != before
    }

    // ── Rules ────────────────────────────────────────────────────────────────

    /// Rule: a black piece cannot be placed so close to its end of the line
    /// that the required sum is unreachable.
    ///
    /// Consider black-1 (the *first* black in a row) at position `p`.  The
    /// best-case position for black-2 is the far end (position 5), leaving at
    /// most `4 - p` number-cells between them.  The maximum sum from `k`
    /// numbers is `MAX_SUM[k]`.  So if `target > MAX_SUM[4 - p]`, position
    /// `p` can never satisfy the target and we remove the black-1 bit there.
    ///
    /// Position 5 is always forbidden for black-1 (no room for black-2).
    /// Black-2 is symmetric: position 0 is always forbidden, and position `p`
    /// is forbidden when `target > MAX_SUM[p - 1]`.
    fn apply_black_position_rules(&mut self) -> bool {
        let mut changed = false;

        for r in 0..6 {
            let t = self.puzzle.row_targets[r];
            for p in 0..6 {
                // `&&` short-circuits, so the subtraction is only evaluated
                // when safe (p < 5 guarantees 4 - p ≥ 0 as usize).
                let can_be_black1 = p < 5 && t <= Self::MAX_SUM[4 - p];
                if !can_be_black1 {
                    changed |= self.clear_bit(r, p, Self::BLACK1_ROW);
                }
                let can_be_black2 = p > 0 && t <= Self::MAX_SUM[p - 1];
                if !can_be_black2 {
                    changed |= self.clear_bit(r, p, Self::BLACK2_ROW);
                }
            }
        }

        // Identical logic for columns (row and column indices are swapped).
        for c in 0..6 {
            let t = self.puzzle.col_targets[c];
            for p in 0..6 {
                let can_be_black1 = p < 5 && t <= Self::MAX_SUM[4 - p];
                if !can_be_black1 {
                    changed |= self.clear_bit(p, c, Self::BLACK1_COL);
                }
                let can_be_black2 = p > 0 && t <= Self::MAX_SUM[p - 1];
                if !can_be_black2 {
                    changed |= self.clear_bit(p, c, Self::BLACK2_COL);
                }
            }
        }

        changed
    }

    /// Rule: for targets 8 and 9, one specific digit is forced to an endpoint,
    /// and each endpoint cell is restricted to that digit or the adjacent black.
    ///
    /// All four digits {1, 2, 3, 4} appear in every row/column, summing to 10.
    /// Digits *between* the two blacks sum to the target, so digits *outside*
    /// (before black-1 or after black-2) sum to `10 - target`.
    ///
    /// - target = 9 → outside sum = 1 → only digit 1 alone achieves this.
    ///   Digit 1 must sit at an endpoint (position 0 or 5); the other endpoint
    ///   must be a black.  So each endpoint holds either digit 1 or the black
    ///   piece that naturally borders it: black-1 at position 0, black-2 at 5.
    ///
    /// - target = 8 → outside sum = 2 → only digit 2 (two 1s are ruled out by
    ///   uniqueness).  Same endpoint logic applies with digit 2.
    ///
    /// Concretely, for each endpoint we clear all digits except the forced one,
    /// plus the black variant that cannot appear there.
    fn apply_endpoint_digit_rules(&mut self) -> bool {
        let mut changed = false;

        for r in 0..6 {
            match self.puzzle.row_targets[r] {
                9 => {
                    // Digit 1 is outside the blacks → remove from middle cells.
                    for c in 1..5 {
                        changed |= self.clear_bit(r, c, 1 << 1);
                    }
                    // Position 0: digit 1 or black-1.  Clear digits 2–4 and black-2.
                    changed |= self.clear_mask(r, 0, (1 << 2) | (1 << 3) | (1 << 4) | Self::BLACK2_ROW);
                    // Position 5: digit 1 or black-2.  Clear digits 2–4 and black-1.
                    changed |= self.clear_mask(r, 5, (1 << 2) | (1 << 3) | (1 << 4) | Self::BLACK1_ROW);
                }
                8 => {
                    // Digit 2 is outside the blacks → remove from middle cells.
                    for c in 1..5 {
                        changed |= self.clear_bit(r, c, 1 << 2);
                    }
                    // Position 0: digit 2 or black-1.  Clear digits 1, 3–4 and black-2.
                    changed |= self.clear_mask(r, 0, (1 << 1) | (1 << 3) | (1 << 4) | Self::BLACK2_ROW);
                    // Position 5: digit 2 or black-2.  Clear digits 1, 3–4 and black-1.
                    changed |= self.clear_mask(r, 5, (1 << 1) | (1 << 3) | (1 << 4) | Self::BLACK1_ROW);
                }
                _ => {}
            }
        }

        for c in 0..6 {
            match self.puzzle.col_targets[c] {
                9 => {
                    for r in 1..5 {
                        changed |= self.clear_bit(r, c, 1 << 1);
                    }
                    changed |= self.clear_mask(0, c, (1 << 2) | (1 << 3) | (1 << 4) | Self::BLACK2_COL);
                    changed |= self.clear_mask(5, c, (1 << 2) | (1 << 3) | (1 << 4) | Self::BLACK1_COL);
                }
                8 => {
                    for r in 1..5 {
                        changed |= self.clear_bit(r, c, 1 << 2);
                    }
                    changed |= self.clear_mask(0, c, (1 << 1) | (1 << 3) | (1 << 4) | Self::BLACK2_COL);
                    changed |= self.clear_mask(5, c, (1 << 1) | (1 << 3) | (1 << 4) | Self::BLACK1_COL);
                }
                _ => {}
            }
        }

        changed
    }

    // ── Propagation loop ─────────────────────────────────────────────────────

    /// Run all rules in a loop until no domain shrinks further (a fixpoint).
    ///
    /// Each rule returns `true` if it removed at least one domain bit.  We use
    /// `|` (not `||`) so that **every** rule runs on every pass — logical `||`
    /// would skip later rules the moment an earlier one returns `true`.  When a
    /// full pass leaves every domain unchanged, we have reached a fixpoint and
    /// backtracking search can begin.
    pub fn propagate(&mut self) {
        loop {
            let changed = self.apply_black_position_rules() | self.apply_endpoint_digit_rules();
            if !changed {
                break;
            }
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

    // ── Solver rule tests ─────────────────────────────────────────────────────

    #[test]
    fn black1_row_always_forbidden_at_last_position() {
        // Black-1 can never sit at position 5, even for target = 0.
        let mut state = SolverState::new(Puzzle::new([0; 6], [0; 6]));
        state.propagate();
        for r in 0..6 {
            assert_eq!(
                state.cell_domains[r][5] & SolverState::<6>::BLACK1_ROW,
                0,
                "row {r}: black-1 should be cleared at the last position"
            );
        }
    }

    #[test]
    fn black1_row_positional_bounds_target_9() {
        // With target = 9, black-1 may only be at positions 0 and 1.
        //   p=0: MAX_SUM[4] = 10 ≥ 9  → allowed
        //   p=1: MAX_SUM[3] =  9 ≥ 9  → allowed
        //   p=2: MAX_SUM[2] =  7 < 9  → forbidden
        //   p=3: MAX_SUM[1] =  4 < 9  → forbidden
        //   p=4: MAX_SUM[0] =  0 < 9  → forbidden
        //   p=5: always forbidden
        let mut state = SolverState::new(Puzzle::new([9, 0, 0, 0, 0, 0], [0; 6]));
        state.propagate();

        assert_ne!(
            state.cell_domains[0][0] & SolverState::<6>::BLACK1_ROW,
            0,
            "p=0 should still be allowed"
        );
        assert_ne!(
            state.cell_domains[0][1] & SolverState::<6>::BLACK1_ROW,
            0,
            "p=1 should still be allowed"
        );
        for p in 2..6 {
            assert_eq!(
                state.cell_domains[0][p] & SolverState::<6>::BLACK1_ROW,
                0,
                "p={p} should be forbidden for black-1 with target 9"
            );
        }
    }

    #[test]
    fn endpoint_digit_rule_target_9() {
        // Row 0 has target 9: digit 1 is outside the blacks.
        let mut state = SolverState::new(Puzzle::new([9, 0, 0, 0, 0, 0], [0; 6]));
        state.propagate();

        // Middle cells lose digit 1.
        for c in 1..5 {
            assert_eq!(
                state.cell_domains[0][c] & (1 << 1),
                0,
                "digit 1 should be cleared from middle cell (row=0, col={c})"
            );
        }
        // Position 0: only digit 1 or black-1 remain (row bits).
        assert_ne!(state.cell_domains[0][0] & (1 << 1), 0, "col=0 should keep digit 1");
        assert_ne!(
            state.cell_domains[0][0] & SolverState::<6>::BLACK1_ROW,
            0,
            "col=0 should keep black-1"
        );
        assert_eq!(
            state.cell_domains[0][0] & ((1 << 2) | (1 << 3) | (1 << 4) | SolverState::<6>::BLACK2_ROW),
            0,
            "col=0 should have digits 2-4 and black-2 cleared"
        );
        // Position 5: only digit 1 or black-2 remain (row bits).
        assert_ne!(state.cell_domains[0][5] & (1 << 1), 0, "col=5 should keep digit 1");
        assert_ne!(
            state.cell_domains[0][5] & SolverState::<6>::BLACK2_ROW,
            0,
            "col=5 should keep black-2"
        );
        assert_eq!(
            state.cell_domains[0][5] & ((1 << 2) | (1 << 3) | (1 << 4) | SolverState::<6>::BLACK1_ROW),
            0,
            "col=5 should have digits 2-4 and black-1 cleared"
        );
    }

    #[test]
    fn endpoint_digit_rule_target_8_column() {
        // Column 2 has target 8: digit 2 is outside the blacks.
        let mut col_targets = [0u8; 6];
        col_targets[2] = 8;
        let mut state = SolverState::new(Puzzle::new([0; 6], col_targets));
        state.propagate();

        // Middle cells lose digit 2.
        for r in 1..5 {
            assert_eq!(
                state.cell_domains[r][2] & (1 << 2),
                0,
                "digit 2 should be cleared from middle cell (row={r}, col=2)"
            );
        }
        // Row 0: only digit 2 or black-1-col remain (col bits).
        assert_ne!(state.cell_domains[0][2] & (1 << 2), 0, "row=0 should keep digit 2");
        assert_ne!(
            state.cell_domains[0][2] & SolverState::<6>::BLACK1_COL,
            0,
            "row=0 should keep black-1-col"
        );
        assert_eq!(
            state.cell_domains[0][2] & ((1 << 1) | (1 << 3) | (1 << 4) | SolverState::<6>::BLACK2_COL),
            0,
            "row=0 should have digits 1,3,4 and black-2-col cleared"
        );
        // Row 5: only digit 2 or black-2-col remain (col bits).
        assert_ne!(state.cell_domains[5][2] & (1 << 2), 0, "row=5 should keep digit 2");
        assert_ne!(
            state.cell_domains[5][2] & SolverState::<6>::BLACK2_COL,
            0,
            "row=5 should keep black-2-col"
        );
        assert_eq!(
            state.cell_domains[5][2] & ((1 << 1) | (1 << 3) | (1 << 4) | SolverState::<6>::BLACK1_COL),
            0,
            "row=5 should have digits 1,3,4 and black-1-col cleared"
        );
    }
}
