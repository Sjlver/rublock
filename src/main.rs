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

    // Composite masks for common groups of bits.
    const ALL_DIGITS: u64 = (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4);
    const ROW_BLACKS: u64 = (1 << 5) | (1 << 6); // BLACK1_ROW | BLACK2_ROW
    const COL_BLACKS: u64 = (1 << 7) | (1 << 8); // BLACK1_COL | BLACK2_COL

    // Valid distance d = p2 - p1 between the two blacks, indexed by target.
    //
    // With k = d - 1 cells between the blacks:
    //   min achievable sum: 0, 1, 3, 6, 10   (for k = 0..=4)
    //   max achievable sum: 0, 4, 7, 9, 10
    //
    // D_MIN[t] = smallest d with max_sum[d-1] >= t  (target is reachable)
    // D_MAX[t] = largest  d with min_sum[d-1] <= t  (target is not overshot)
    const D_MIN: [usize; 11] = [1, 2, 2, 2, 2, 3, 3, 3, 4, 4, 5];
    const D_MAX: [usize; 11] = [1, 2, 2, 3, 3, 3, 4, 4, 4, 4, 5];

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

    /// Assign `bit` to cell (row, col) and propagate the constraint.
    ///
    /// `bit` must be a single set bit representing one domain value.  The
    /// propagation depends on the kind of value:
    ///
    /// **Digit bit** — a digit is unique across the whole row *and* column, so
    /// the bit is removed from every other cell in both.  The cell's domain is
    /// reduced to just this bit.
    ///
    /// **Row-black bit** (BLACK1_ROW / BLACK2_ROW) — each row-black variant
    /// appears exactly once per row, so the bit is cleared from every other
    /// cell in the row.  The cell's other row-black variant and all digit bits
    /// are cleared (it's black).  The two *column*-black bits are left open:
    /// being the first black in a row doesn't yet tell us whether this cell is
    /// the first or second black in its column.
    ///
    /// Additionally, the blacks must appear in order (black-1 before black-2).
    /// Placing black-1 at `col` means black-2 is impossible anywhere to its
    /// left, and vice versa.  Those cells lose the corresponding bit.
    ///
    /// **Col-black bit** — symmetric to row-black but scoped to the column.
    fn set_cell(&mut self, row: usize, col: usize, bit: u64) -> bool {
        debug_assert_eq!(bit.count_ones(), 1, "set_cell requires exactly one bit");
        let mut changed = false;

        if bit & Self::ALL_DIGITS != 0 {
            let k = bit.trailing_zeros() as u8;
            self.puzzle.board[row][col] = Cell::Number(k);
            // Remove this digit from every other cell in the row and column.
            for c in (0..6).filter(|&c| c != col) {
                changed |= self.clear_bit(row, c, bit);
            }
            for r in (0..6).filter(|&r| r != row) {
                changed |= self.clear_bit(r, col, bit);
            }
            // Fix this cell: keep only this digit.
            changed |= self.clear_mask(row, col, !bit);
        } else if bit & Self::ROW_BLACKS != 0 {
            // TODO: Let's make sure that when setting a cell to black, we remove BLACK 2 COL from the preceding columns, and BLACK 2 ROW from the preceding rows, and vice versa. We seem to not remove enough possibilities here.          
            self.puzzle.board[row][col] = Cell::Black;
            // Each row-black variant appears once per row.
            for c in (0..6).filter(|&c| c != col) {
                changed |= self.clear_bit(row, c, bit);
            }
            // Ordering: the two blacks appear left-to-right as black-1 then black-2.
            // Placing black-1 at `col` means black-2 cannot be anywhere to its left,
            // and placing black-2 at `col` means black-1 cannot be anywhere to its right.
            if bit == Self::BLACK1_ROW {
                for c in 0..col {
                    changed |= self.clear_bit(row, c, Self::BLACK2_ROW);
                }
            } else {
                for c in (col + 1)..6 {
                    changed |= self.clear_bit(row, c, Self::BLACK1_ROW);
                }
            }
            // Cell is black: drop digits and the other row-black variant.
            changed |= self.clear_mask(row, col, Self::ALL_DIGITS | (Self::ROW_BLACKS & !bit));
        } else if bit & Self::COL_BLACKS != 0 {
            self.puzzle.board[row][col] = Cell::Black;
            // Each col-black variant appears once per column.
            for r in (0..6).filter(|&r| r != row) {
                changed |= self.clear_bit(r, col, bit);
            }
            // Ordering: black-1 is above black-2 in the column.
            if bit == Self::BLACK1_COL {
                for r in 0..row {
                    changed |= self.clear_bit(r, col, Self::BLACK2_COL);
                }
            } else {
                for r in (row + 1)..6 {
                    changed |= self.clear_bit(r, col, Self::BLACK1_COL);
                }
            }
            // Cell is black: drop digits and the other col-black variant.
            changed |= self.clear_mask(row, col, Self::ALL_DIGITS | (Self::COL_BLACKS & !bit));
        }

        changed
    }

    // ── Rules ────────────────────────────────────────────────────────────────

    /// Rule: black-1 and black-2 must be placed at a distance `d = p2 - p1`
    /// within `[D_MIN[target], D_MAX[target]]`, and each must have a valid
    /// counterpart still present in the current domain.
    ///
    /// For black-1 at position `p`, remove it if no cell in
    /// `[p + D_MIN, p + D_MAX]` still has BLACK2 in its domain.  Black-2 is
    /// symmetric: for black-2 at `p`, look for BLACK1 in `[p - D_MAX, p - D_MIN]`.
    ///
    /// This subsumes the earlier positional bound rule: on a fresh board (all
    /// bits set) checking the range reduces to exactly the old positional check.
    /// But once other rules narrow the BLACK2 candidates, this rule fires again
    /// with more force.
    fn apply_black_range_rules(&mut self) -> bool {
        let mut changed = false;

        for r in 0..6 {
            let t = self.puzzle.row_targets[r] as usize;
            let d_min = Self::D_MIN[t];
            let d_max = Self::D_MAX[t];
            for p in 0..6 {
                // BLACK1_ROW at p: need BLACK2_ROW somewhere in [p+d_min, p+d_max].
                let lo = p + d_min;
                let hi = (p + d_max).min(5);
                if lo > 5 || (lo..=hi).all(|q| self.cell_domains[r][q] & Self::BLACK2_ROW == 0) {
                    changed |= self.clear_bit(r, p, Self::BLACK1_ROW);
                }
                // BLACK2_ROW at p: need BLACK1_ROW somewhere in [p-d_max, p-d_min].
                if p < d_min {
                    changed |= self.clear_bit(r, p, Self::BLACK2_ROW);
                } else {
                    let hi2 = p - d_min;
                    let lo2 = p.saturating_sub(d_max);
                    if (lo2..=hi2).all(|q| self.cell_domains[r][q] & Self::BLACK1_ROW == 0) {
                        changed |= self.clear_bit(r, p, Self::BLACK2_ROW);
                    }
                }
            }
        }

        for c in 0..6 {
            let t = self.puzzle.col_targets[c] as usize;
            let d_min = Self::D_MIN[t];
            let d_max = Self::D_MAX[t];
            for p in 0..6 {
                let lo = p + d_min;
                let hi = (p + d_max).min(5);
                if lo > 5 || (lo..=hi).all(|q| self.cell_domains[q][c] & Self::BLACK2_COL == 0) {
                    changed |= self.clear_bit(p, c, Self::BLACK1_COL);
                }
                if p < d_min {
                    changed |= self.clear_bit(p, c, Self::BLACK2_COL);
                } else {
                    let hi2 = p - d_min;
                    let lo2 = p.saturating_sub(d_max);
                    if (lo2..=hi2).all(|q| self.cell_domains[q][c] & Self::BLACK1_COL == 0) {
                        changed |= self.clear_bit(p, c, Self::BLACK2_COL);
                    }
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

    /// Rule: if a cell's domain has shrunk to a single bit, assign it; if it
    /// has no digit bits remaining, mark it as black on the board.
    ///
    /// A singleton domain means there is only one possible value — call
    /// `set_cell` to fix it and propagate.  A domain with only black bits (but
    /// more than one) means the cell is definitely black even though we don't
    /// yet know its row/column order; we record that on the board so it can be
    /// used for display and checking, but defer full assignment.
    fn apply_singleton_rule(&mut self) -> bool {
        let mut changed = false;

        for r in 0..6 {
            for c in 0..6 {
                let domain = self.cell_domains[r][c];
                if domain.count_ones() == 1 {
                    changed |= self.set_cell(r, c, domain);
                } else if domain & Self::ALL_DIGITS == 0 && domain != 0 {
                    if self.puzzle.board[r][c] != Cell::Black {
                        self.puzzle.board[r][c] = Cell::Black;
                        changed = true;
                    }
                }
            }
        }

        changed
    }

    /// Rule: if a value can only go in one cell in a row or column, place it
    /// there.
    ///
    /// For each row we scan the four digit bits and the two row-black bits.  If
    /// exactly one cell in the row has a given bit set in its domain, that cell
    /// is the only candidate — assign it via `set_cell`.  Column scanning is
    /// identical but uses the col-black bits instead of the row-black bits.
    ///
    /// We do not pre-compute or cache per-row/column availability: the O(6)
    /// scan is cheap, and storing a summary would be redundant state that needs
    /// to be kept in sync.
    fn apply_hidden_single_rule(&mut self) -> bool {
        let mut changed = false;

        for r in 0..6 {
            // Bits 1–4 are digits; bits 5–6 are the two row-black variants.
            for bit_pos in 1u32..=6 {
                let bit = 1u64 << bit_pos;
                let mut count = 0;
                let mut only_col = 0;
                for c in 0..6 {
                    if self.cell_domains[r][c] & bit != 0 {
                        count += 1;
                        only_col = c;
                    }
                }
                if count == 1 {
                    changed |= self.set_cell(r, only_col, bit);
                }
            }
        }

        for c in 0..6 {
            // Bits 1–4 are digits; bits 7–8 are the two col-black variants.
            for bit_pos in [1u32, 2, 3, 4, 7, 8] {
                let bit = 1u64 << bit_pos;
                let mut count = 0;
                let mut only_row = 0;
                for r in 0..6 {
                    if self.cell_domains[r][c] & bit != 0 {
                        count += 1;
                        only_row = r;
                    }
                }
                if count == 1 {
                    changed |= self.set_cell(only_row, c, bit);
                }
            }
        }

        changed
    }

    /// Rule: black is a single physical fact — a cell is either black or it
    /// isn't, in both its row and its column simultaneously.
    ///
    /// If a cell has no row-black bits in its domain, it cannot be black, so
    /// any col-black bits are also impossible and are cleared.  Vice versa: no
    /// col-black bits means the row-black bits must also go.
    fn apply_black_consistency_rule(&mut self) -> bool {
        let mut changed = false;

        for r in 0..6 {
            for c in 0..6 {
                let domain = self.cell_domains[r][c];
                if domain & Self::ROW_BLACKS == 0 {
                    changed |= self.clear_mask(r, c, Self::COL_BLACKS);
                }
                if domain & Self::COL_BLACKS == 0 {
                    changed |= self.clear_mask(r, c, Self::ROW_BLACKS);
                }
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
            let changed = self.apply_black_range_rules()
                | self.apply_endpoint_digit_rules()
                | self.apply_black_consistency_rule()
                | self.apply_singleton_rule()
                | self.apply_hidden_single_rule();
            if !changed {
                break;
            }
        }
    }
}

// ── Display ───────────────────────────────────────────────────────────────────

impl SolverState<6> {
    /// Print the board and, for unsolved cells, the remaining domain bits.
    fn display(&self) {
        let col_targets = &self.puzzle.col_targets;
        println!("     {:2} {:2} {:2} {:2} {:2} {:2}", col_targets[0], col_targets[1], col_targets[2], col_targets[3], col_targets[4], col_targets[5]);
        println!("   +---+---+---+---+---+---+");
        for r in 0..6 {
            let t = self.puzzle.row_targets[r];
            print!("{:2} |", t);
            for c in 0..6 {
                let sym = match self.puzzle.board[r][c] {
                    Cell::Black => " # ".to_string(),
                    Cell::Number(n) => format!(" {} ", n),
                    Cell::Empty => {
                        let d = self.cell_domains[r][c];
                        let bits = d.count_ones();
                        if bits == 0 {
                            " ! ".to_string() // contradiction
                        } else {
                            format!("{:2}.", bits)
                        }
                    }
                };
                print!("{}|", sym);
            }
            println!();
            println!("   +---+---+---+---+---+---+");
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

    // ── set_cell / singleton / hidden-single / black-consistency tests ────────

    #[test]
    fn set_cell_digit_propagates_to_row_and_col() {
        // Manually place digit 3 at (0, 0) and check it is removed from the
        // rest of row 0 and column 0, while other cells are untouched.
        let mut state = SolverState::new(Puzzle::new([0; 6], [0; 6]));
        state.set_cell(0, 0, 1 << 3);

        // The cell itself holds only digit 3.
        assert_eq!(state.cell_domains[0][0], 1 << 3);
        assert_eq!(state.puzzle.board[0][0], Cell::Number(3));

        // Digit 3 is gone from the rest of row 0 and col 0.
        for c in 1..6 {
            assert_eq!(state.cell_domains[0][c] & (1 << 3), 0, "row 0, col {c}");
        }
        for r in 1..6 {
            assert_eq!(state.cell_domains[r][0] & (1 << 3), 0, "row {r}, col 0");
        }

        // An unrelated cell (1, 1) is completely untouched.
        assert_eq!(state.cell_domains[1][1], 0b111111110);
    }

    #[test]
    fn set_cell_row_black_propagates_within_row_not_col() {
        let mut state = SolverState::new(Puzzle::new([0; 6], [0; 6]));
        state.set_cell(2, 1, SolverState::<6>::BLACK1_ROW);

        // The assigned cell keeps BLACK1_ROW and both col-black bits, but
        // loses digits and BLACK2_ROW.
        let d = state.cell_domains[2][1];
        assert_ne!(d & SolverState::<6>::BLACK1_ROW, 0, "keep BLACK1_ROW");
        assert_ne!(d & SolverState::<6>::BLACK1_COL, 0, "keep BLACK1_COL");
        assert_ne!(d & SolverState::<6>::BLACK2_COL, 0, "keep BLACK2_COL");
        assert_eq!(d & SolverState::<6>::BLACK2_ROW, 0, "drop BLACK2_ROW");
        assert_eq!(d & SolverState::<6>::ALL_DIGITS, 0, "drop all digits");

        // BLACK1_ROW is gone from every other cell in row 2.
        for c in (0..6).filter(|&c| c != 1) {
            assert_eq!(
                state.cell_domains[2][c] & SolverState::<6>::BLACK1_ROW,
                0,
                "col {c} should lose BLACK1_ROW"
            );
        }

        // Column 1 is unaffected (col-black order not yet decided).
        for r in (0..6).filter(|&r| r != 2) {
            assert_eq!(
                state.cell_domains[r][1] & SolverState::<6>::BLACK1_COL,
                SolverState::<6>::BLACK1_COL,
                "row {r} col 1 should still have BLACK1_COL"
            );
        }
    }

    #[test]
    fn set_cell_row_black_ordering_constraint() {
        // Place BLACK1_ROW at position 3.  Positions 0..3 must lose BLACK2_ROW
        // (they are left of black-1 and can't be black-2).  Positions 4 and 5
        // keep BLACK2_ROW (black-2 still possible there).
        let mut state = SolverState::new(Puzzle::new([0; 6], [0; 6]));
        state.set_cell(0, 3, SolverState::<6>::BLACK1_ROW);

        for c in 0..3 {
            assert_eq!(
                state.cell_domains[0][c] & SolverState::<6>::BLACK2_ROW,
                0,
                "col {c} should lose BLACK2_ROW (left of black-1)"
            );
        }
        for c in 4..6 {
            assert_ne!(
                state.cell_domains[0][c] & SolverState::<6>::BLACK2_ROW,
                0,
                "col {c} should keep BLACK2_ROW (right of black-1)"
            );
        }
    }

    #[test]
    fn black_range_rule_uses_actual_domain_state() {
        // Row 0 with target 0 (d_min = d_max = 1, blacks must be adjacent).
        // Manually clear BLACK2_ROW from every cell except col 3.
        // Then BLACK1_ROW is only valid at col 2 (the sole cell adjacent to
        // the remaining BLACK2_ROW candidate).
        let mut state = SolverState::new(Puzzle::new([0; 6], [0; 6]));
        for c in (0..6).filter(|&c| c != 3) {
            state.cell_domains[0][c] &= !SolverState::<6>::BLACK2_ROW;
        }
        state.apply_black_range_rules();

        assert_ne!(
            state.cell_domains[0][2] & SolverState::<6>::BLACK1_ROW,
            0,
            "col 2 should keep BLACK1_ROW (adjacent to the only BLACK2_ROW at col 3)"
        );
        for p in (0..6).filter(|&p| p != 2) {
            assert_eq!(
                state.cell_domains[0][p] & SolverState::<6>::BLACK1_ROW,
                0,
                "col {p} should lose BLACK1_ROW"
            );
        }
    }

    #[test]
    fn apply_singleton_rule_assigns_sole_digit() {
        let mut state = SolverState::new(Puzzle::new([0; 6], [0; 6]));
        // Force cell (3, 3) to have only digit 2 in its domain.
        state.cell_domains[3][3] = 1 << 2;
        // Run just this one rule (not propagate, to isolate it).
        state.apply_singleton_rule();

        assert_eq!(state.puzzle.board[3][3], Cell::Number(2));
        // Digit 2 should be gone from the rest of row 3 and col 3.
        for c in (0..6).filter(|&c| c != 3) {
            assert_eq!(state.cell_domains[3][c] & (1 << 2), 0);
        }
        for r in (0..6).filter(|&r| r != 3) {
            assert_eq!(state.cell_domains[r][3] & (1 << 2), 0);
        }
    }

    #[test]
    fn apply_hidden_single_rule_assigns_forced_digit() {
        let mut state = SolverState::new(Puzzle::new([0; 6], [0; 6]));
        // Remove digit 4 from every cell in row 0 except column 2.
        for c in (0..6).filter(|&c| c != 2) {
            state.cell_domains[0][c] &= !(1 << 4);
        }
        state.apply_hidden_single_rule();

        assert_eq!(state.puzzle.board[0][2], Cell::Number(4));
        assert_eq!(state.cell_domains[0][2], 1 << 4);
    }

    #[test]
    fn apply_black_consistency_rule_clears_col_blacks_when_no_row_blacks() {
        let mut state = SolverState::new(Puzzle::new([0; 6], [0; 6]));
        // Strip all row-black bits from cell (1, 4).
        state.cell_domains[1][4] &= !SolverState::<6>::ROW_BLACKS;
        state.apply_black_consistency_rule();

        // Col-black bits must now also be gone.
        assert_eq!(state.cell_domains[1][4] & SolverState::<6>::COL_BLACKS, 0);
        // But digit bits are intact.
        assert_eq!(
            state.cell_domains[1][4] & SolverState::<6>::ALL_DIGITS,
            SolverState::<6>::ALL_DIGITS
        );
    }

    // ── Newspaper puzzles ─────────────────────────────────────────────────────
    //
    // These tests assert only the cells that propagation fully determines.
    // Domain sizes for unsolved cells are intentionally not checked: they will
    // shrink as more rules are added, so asserting them would make the tests
    // brittle.

    #[test]
    fn newspaper_puzzle_1() {
        // rows 8 2 3 8 9 0 / cols 0 0 5 9 0 4
        //
        //       0  0  5  9  0  4
        //    +---+---+---+---+---+---+
        //  8 |   |   |   | 1 |   |   |
        //  2 |   |   |   | # |   |   |
        //  3 |   |   |   |   |   |   |
        //  8 |   |   |   |   |   |   |
        //  9 |   |   |   |   |   |   |
        //  0 |   |   | # | # |   |   |
        let mut state = SolverState::new(Puzzle::new(
            [8, 2, 3, 8, 9, 0],
            [0, 0, 5, 9, 0, 4],
        ));
        state.propagate();

        //       0  0  5  9  0  4
        //    +---+---+---+---+---+---+
        //  8 | 2 | # |   | 1 |   | # |
        //  2 |   |   |   | # |   |   |
        //  3 |   |   | # |   |   |   |
        //  8 | # |   |   |   | # | 2 |
        //  9 |   |   |   |   |   |   |
        //  0 |   |   | # | # |   |   |
        assert_eq!(state.puzzle.board[0][0], Cell::Number(2));
        assert_eq!(state.puzzle.board[0][1], Cell::Black);
        assert_eq!(state.puzzle.board[0][3], Cell::Number(1));
        assert_eq!(state.puzzle.board[0][5], Cell::Black);
        assert_eq!(state.puzzle.board[1][3], Cell::Black);
        assert_eq!(state.puzzle.board[2][2], Cell::Black);
        assert_eq!(state.puzzle.board[3][0], Cell::Black);
        assert_eq!(state.puzzle.board[3][4], Cell::Black);
        assert_eq!(state.puzzle.board[3][5], Cell::Number(2));
        assert_eq!(state.puzzle.board[5][2], Cell::Black);
        assert_eq!(state.puzzle.board[5][3], Cell::Black);
    }

    #[test]
    fn newspaper_puzzle_2() {
        // rows 3 3 5 0 7 0 / cols 5 0 2 6 5 10
        //
        //       5  0  2  6  5 10
        //    +---+---+---+---+---+---+
        //  3 |   |   |   |   |   | # |
        //  3 |   |   |   |   |   |   |
        //  5 |   |   |   |   |   |   |
        //  0 |   |   |   |   |   |   |
        //  7 |   |   |   |   |   |   |
        //  0 |   |   |   |   |   | # |
        //
        // Column 5 has target 10: the only way to sum 1+2+3+4=10 is with all
        // four digits between the blacks, forcing black at both endpoints.
        // Row 5 target 0 requires adjacent blacks, but the column constraints
        // don't yet uniquely determine which pair — backtracking is needed.
        let mut state = SolverState::new(Puzzle::new(
            [3, 3, 5, 0, 7, 0],
            [5, 0, 2, 6, 5, 10],
        ));
        state.propagate();

        assert_eq!(state.puzzle.board[0][5], Cell::Black);
        assert_eq!(state.puzzle.board[5][5], Cell::Black);
    }
}
