use std::fmt;

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

    // Digit bits that are incompatible with being inside (between the blacks),
    // indexed by target.  A digit d cannot be inside iff no subset of {1,2,3,4}
    // containing d sums to t.
    //
    // CANT_BE_OUTSIDE[t] == CANT_BE_INSIDE[10 - t] by symmetry (inside and
    // outside always partition the same four digits, summing to 10 in total).
    const CANT_BE_INSIDE: [u64; 11] = [
        (1 << 1) | (1 << 2) | (1 << 3) | (1 << 4), // t=0:  no digit can be inside
        (1 << 2) | (1 << 3) | (1 << 4),            // t=1:  only digit 1 can be inside
        (1 << 1) | (1 << 3) | (1 << 4),            // t=2:  only digit 2 can be inside
        (1 << 4),                                  // t=3:  digits 1–3 can be inside; 4 cannot
        (1 << 2),                                  // t=4:  digits 1,3,4 can be inside; 2 cannot
        0,                                         // t=5:  all digits can be inside
        0,                                         // t=6:  all digits can be inside
        0,                                         // t=7:  all digits can be inside
        (1 << 2),                                  // t=8:  digits 1,3,4 can be inside; 2 cannot
        (1 << 1),                                  // t=9:  digits 2,3,4 can be inside; 1 cannot
        0,                                         // t=10: all digits can be inside
    ];
    const ROW_BLACKS: u64 = Self::BLACK1_ROW | Self::BLACK2_ROW;
    const COL_BLACKS: u64 = Self::BLACK1_COL | Self::BLACK2_COL;
    const ALL_BLACKS: u64 = Self::ROW_BLACKS | Self::COL_BLACKS;

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
    /// **Digit bit** — a digit is unique across the whole row and column, so
    /// the bit is removed from every other cell in both. The cell's domain is
    /// reduced to just this bit.
    ///
    /// **Black bit** - Clear the bit from its row (if it's a row black) or
    /// column. Clear BLACK 1 from cells to the left and above, and BLACK 2
    /// from cells to the right.
    fn set_cell(&mut self, row: usize, col: usize, bit: u64) -> bool {
        debug_assert_eq!(bit.count_ones(), 1, "set_cell requires exactly one bit");
        let mut changed = false;

        if bit & Self::ALL_DIGITS != 0 {
            let k = bit.trailing_zeros() as u8;
            self.puzzle.board[row][col] = Cell::Number(k);
            // Remove this digit from every other cell in the row and column.
            for c in (0..6).filter(|&c| c != col) {
                changed |= self.clear_mask(row, c, bit);
            }
            for r in (0..6).filter(|&r| r != row) {
                changed |= self.clear_mask(r, col, bit);
            }
            // Fix this cell: keep only this digit.
            changed |= self.clear_mask(row, col, !bit);
        } else if bit & Self::ROW_BLACKS != 0 {
            self.puzzle.board[row][col] = Cell::Black;

            // Each row-black variant appears once per row.
            for c in (0..6).filter(|&c| c != col) {
                changed |= self.clear_mask(row, c, bit);
            }

            // Cell is black: drop digits and the other row-black variant.
            changed |= self.clear_mask(row, col, Self::ALL_DIGITS | (Self::ROW_BLACKS & !bit));
        } else if bit & Self::COL_BLACKS != 0 {
            self.puzzle.board[row][col] = Cell::Black;

            // Each col-black variant appears once per column.
            for r in (0..6).filter(|&r| r != row) {
                changed |= self.clear_mask(r, col, bit);
            }

            // Cell is black: drop digits and the other col-black variant.
            changed |= self.clear_mask(row, col, Self::ALL_DIGITS | (Self::COL_BLACKS & !bit));
        }

        if bit & Self::ALL_BLACKS != 0 {
            // Enforce ordering: clear all BLACK2 from cells above and to the
            // left, and clear BLACK1 from cells below and to the right.
            for left in 0..col {
                changed |= self.clear_mask(row, left, Self::BLACK2_ROW)
            }
            for right in col + 1..6 {
                changed |= self.clear_mask(row, right, Self::BLACK1_ROW)
            }
            for above in 0..row {
                changed |= self.clear_mask(above, col, Self::BLACK2_COL)
            }
            for below in row + 1..6 {
                changed |= self.clear_mask(below, col, Self::BLACK1_COL)
            }
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
    fn apply_black_range_rules(&mut self) -> bool {
        let mut changed = false;

        for r in 0..6 {
            let t = self.puzzle.row_targets[r] as usize;
            let d_min = Self::D_MIN[t];
            let d_max = Self::D_MAX[t];
            for p in 0..6 {
                // BLACK1_ROW at p: need BLACK2_ROW somewhere in [p+d_min, p+d_max].
                let lo = p + d_min;
                let hi = (p + d_max + 1).min(6);
                if (lo..hi).all(|q| self.cell_domains[r][q] & Self::BLACK2_ROW == 0) {
                    changed |= self.clear_mask(r, p, Self::BLACK1_ROW);
                }
                // BLACK2_ROW at p: need BLACK1_ROW somewhere in [p-d_max, p-d_min].
                let hi2 = (p + 1).saturating_sub(d_min);
                let lo2 = p.saturating_sub(d_max);
                if (lo2..hi2).all(|q| self.cell_domains[r][q] & Self::BLACK1_ROW == 0) {
                    changed |= self.clear_mask(r, p, Self::BLACK2_ROW);
                }
            }
        }

        for c in 0..6 {
            let t = self.puzzle.col_targets[c] as usize;
            let d_min = Self::D_MIN[t];
            let d_max = Self::D_MAX[t];
            for p in 0..6 {
                let lo = p + d_min;
                let hi = (p + d_max + 1).min(6);
                if (lo..hi).all(|q| self.cell_domains[q][c] & Self::BLACK2_COL == 0) {
                    changed |= self.clear_mask(p, c, Self::BLACK1_COL);
                }
                let hi2 = (p + 1).saturating_sub(d_min);
                let lo2 = p.saturating_sub(d_max);
                if (lo2..hi2).all(|q| self.cell_domains[q][c] & Self::BLACK1_COL == 0) {
                    changed |= self.clear_mask(p, c, Self::BLACK2_COL);
                }
            }
        }

        changed
    }

    /// Rule: remove digits that cannot be inside from cells that must be inside,
    /// and digits that cannot be outside from cells that must be outside.
    ///
    /// A cell at position `p` is **definitely outside-left** if no cell in `[0..p)`
    /// has `BLACK1_ROW` in its domain — BLACK1 can only be at `p` or later, so if
    /// `p` holds a digit it lies to the left of every possible BLACK1.  Symmetrically,
    /// `p` is **definitely outside-right** if no cell in `(p..5]` has `BLACK2_ROW`.
    ///
    /// A cell is **definitely inside** if no `BLACK1_ROW` exists in `(p..5]` (BLACK1
    /// must be before `p`) **and** no `BLACK2_ROW` exists in `[0..p)` (BLACK2 must be
    /// after `p`).
    ///
    /// This rule only removes digit bits; black placement is handled elsewhere.
    fn apply_inside_outside_rule(&mut self) -> bool {
        let mut changed = false;

        for r in 0..6 {
            let t = self.puzzle.row_targets[r] as usize;
            let cant_inside  = Self::CANT_BE_INSIDE[t];
            let cant_outside = Self::CANT_BE_INSIDE[10 - t];

            for p in 0..6 {
                let b1_before = (0..p).any(|q| self.cell_domains[r][q] & Self::BLACK1_ROW != 0);
                let b2_before = (0..p).any(|q| self.cell_domains[r][q] & Self::BLACK2_ROW != 0);
                let b1_after = (p + 1..6).any(|q| self.cell_domains[r][q] & Self::BLACK1_ROW != 0);
                let b2_after = (p + 1..6).any(|q| self.cell_domains[r][q] & Self::BLACK2_ROW != 0);

                if !b1_before || !b2_after {
                    changed |= self.clear_mask(r, p, cant_outside);
                }
                if !b1_after && !b2_before {
                    changed |= self.clear_mask(r, p, cant_inside);
                }
            }
        }

        for c in 0..6 {
            let t = self.puzzle.col_targets[c] as usize;
            let cant_inside  = Self::CANT_BE_INSIDE[t];
            let cant_outside = Self::CANT_BE_INSIDE[10 - t];

            for p in 0..6 {
                let b1_before = (0..p).any(|q| self.cell_domains[q][c] & Self::BLACK1_COL != 0);
                let b2_before = (0..p).any(|q| self.cell_domains[q][c] & Self::BLACK2_COL != 0);
                let b1_after = (p + 1..6).any(|q| self.cell_domains[q][c] & Self::BLACK1_COL != 0);
                let b2_after = (p + 1..6).any(|q| self.cell_domains[q][c] & Self::BLACK2_COL != 0);

                if !b1_before || !b2_after {
                    changed |= self.clear_mask(p, c, cant_outside);
                }
                if !b1_after && !b2_before {
                    changed |= self.clear_mask(p, c, cant_inside);
                }
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
    /// For each row we check the four digit bits and the two row-black bits via
    /// `singleton_in_row`; if exactly one cell carries a given bit, that cell is
    /// the only candidate and is assigned via `set_cell`.  Column scanning is
    /// identical but uses `singleton_in_col` with the col-black bits.
    fn apply_hidden_single_rule(&mut self) -> bool {
        let mut changed = false;

        for r in 0..6 {
            // Bits 1–4 are digits; bits 5–6 are the two row-black variants.
            for bit_pos in 1u32..=6 {
                let bit = 1u64 << bit_pos;
                if let Some(only_col) = self.singleton_in_row(r, bit) {
                    changed |= self.set_cell(r, only_col, bit);
                }
            }
        }

        for c in 0..6 {
            // Bits 1–4 are digits; bits 7–8 are the two col-black variants.
            for bit_pos in [1u32, 2, 3, 4, 7, 8] {
                let bit = 1u64 << bit_pos;
                if let Some(only_row) = self.singleton_in_col(c, bit) {
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

    // ── Low-level helper ─────────────────────────────────────────────────────

    /// Return the unique position in row `r` where `bit` appears in the domain,
    /// or `None` if no such position exists or more than one does.
    fn singleton_in_row(&self, r: usize, bit: u64) -> Option<usize> {
        let mut found = None;
        for c in 0..6 {
            if self.cell_domains[r][c] & bit != 0 {
                if found.is_some() {
                    return None;
                }
                found = Some(c);
            }
        }
        found
    }

    /// Return the unique position in column `c` where `bit` appears in the domain,
    /// or `None` if no such position exists or more than one does.
    fn singleton_in_col(&self, c: usize, bit: u64) -> Option<usize> {
        let mut found = None;
        for r in 0..6 {
            if self.cell_domains[r][c] & bit != 0 {
                if found.is_some() {
                    return None;
                }
                found = Some(r);
            }
        }
        found
    }

    /// Rule: when both black squares in a row or column are pinned to exactly
    /// one position each, and the cells between them contain exactly one
    /// unassigned cell, that cell's digit is fully determined.
    ///
    /// Once `BLACK1` and `BLACK2` each have a unique candidate position, the
    /// inside region is fixed.  If all but one inside cell already hold a digit,
    /// the remaining cell must be `target − sum(assigned inside digits)`.
    fn apply_single_inside_rule(&mut self) -> bool {
        let mut changed = false;

        for r in 0..6 {
            let t = self.puzzle.row_targets[r] as usize;
            let Some(p1) = self.singleton_in_row(r, Self::BLACK1_ROW) else {
                continue;
            };
            let Some(p2) = self.singleton_in_row(r, Self::BLACK2_ROW) else {
                continue;
            };
            let mut empty_col = None;
            let mut assigned_sum = 0usize;
            let mut valid = true;
            for c in p1 + 1..p2 {
                match self.puzzle.board[r][c] {
                    Cell::Number(n) => assigned_sum += n as usize,
                    Cell::Empty => {
                        if empty_col.is_some() {
                            valid = false;
                            break;
                        }
                        empty_col = Some(c);
                    }
                    Cell::Black => {
                        valid = false;
                        break;
                    }
                }
            }
            if !valid {
                continue;
            }
            let Some(e) = empty_col else { continue };
            let Some(digit) = t.checked_sub(assigned_sum) else {
                continue;
            };
            if digit < 1 || digit > 4 {
                continue;
            }
            let target_bit = 1u64 << digit;
            if self.cell_domains[r][e] & target_bit != 0 {
                changed |= self.set_cell(r, e, target_bit);
            }
        }

        for c in 0..6 {
            let t = self.puzzle.col_targets[c] as usize;
            let Some(p1) = self.singleton_in_col(c, Self::BLACK1_COL) else {
                continue;
            };
            let Some(p2) = self.singleton_in_col(c, Self::BLACK2_COL) else {
                continue;
            };
            let mut empty_row = None;
            let mut assigned_sum = 0usize;
            let mut valid = true;
            for r in p1 + 1..p2 {
                match self.puzzle.board[r][c] {
                    Cell::Number(n) => assigned_sum += n as usize,
                    Cell::Empty => {
                        if empty_row.is_some() {
                            valid = false;
                            break;
                        }
                        empty_row = Some(r);
                    }
                    Cell::Black => {
                        valid = false;
                        break;
                    }
                }
            }
            if !valid {
                continue;
            }
            let Some(e) = empty_row else { continue };
            let Some(digit) = t.checked_sub(assigned_sum) else {
                continue;
            };
            if digit < 1 || digit > 4 {
                continue;
            }
            let target_bit = 1u64 << digit;
            if self.cell_domains[e][c] & target_bit != 0 {
                changed |= self.set_cell(e, c, target_bit);
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
                | self.apply_inside_outside_rule()
                | self.apply_black_consistency_rule()
                | self.apply_singleton_rule()
                | self.apply_hidden_single_rule()
                | self.apply_single_inside_rule();
            if !changed {
                break;
            }
        }
    }
}

// ── Display ───────────────────────────────────────────────────────────────────

impl fmt::Display for SolverState<6> {
    /// Print the board and, for unsolved cells, the remaining domain bits.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ct = &self.puzzle.col_targets;
        writeln!(
            f,
            "    {:2}  {:2}  {:2}  {:2}  {:2}  {:2}",
            ct[0], ct[1], ct[2], ct[3], ct[4], ct[5]
        )?;
        writeln!(f, "   +---+---+---+---+---+---+")?;
        for r in 0..6 {
            write!(f, "{:2} |", self.puzzle.row_targets[r])?;
            for c in 0..6 {
                match self.puzzle.board[r][c] {
                    Cell::Black => write!(f, " # |")?,
                    Cell::Number(n) => write!(f, "{:2} |", n)?,
                    Cell::Empty => {
                        let sym = match self.cell_domains[r][c].count_ones() {
                            0 => " X ", // contradiction
                            1 => " ⠁ ",
                            2 => " ⠃ ",
                            3 => " ⠇ ",
                            4 => " ⡇ ",
                            5 => " ⡏ ",
                            6 => " ⡟ ",
                            7 => " ⡿ ",
                            8 => " ⣿ ",
                            bits => panic!("invalid bit count at row {r} col {c}: {bits}"),
                        };
                        write!(f, "{}|", sym)?;
                    }
                }
            }
            writeln!(f)?;
            writeln!(f, "   +---+---+---+---+---+---+")?;
        }
        Ok(())
    }
}

// ── main ──────────────────────────────────────────────────────────────────────

fn main() {
    let puzzle = Puzzle::new([3, 3, 5, 0, 7, 0], [5, 0, 2, 6, 5, 10]);
    let mut state = SolverState::new(puzzle);

    state.propagate();
    println!("{}", state);
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
    fn inside_outside_rule_target_9() {
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
        assert_ne!(
            state.cell_domains[0][0] & (1 << 1),
            0,
            "col=0 should keep digit 1"
        );
        assert_ne!(
            state.cell_domains[0][0] & SolverState::<6>::BLACK1_ROW,
            0,
            "col=0 should keep black-1"
        );
        assert_eq!(
            state.cell_domains[0][0]
                & ((1 << 2) | (1 << 3) | (1 << 4) | SolverState::<6>::BLACK2_ROW),
            0,
            "col=0 should have digits 2-4 and black-2 cleared"
        );
        // Position 5: only digit 1 or black-2 remain (row bits).
        assert_ne!(
            state.cell_domains[0][5] & (1 << 1),
            0,
            "col=5 should keep digit 1"
        );
        assert_ne!(
            state.cell_domains[0][5] & SolverState::<6>::BLACK2_ROW,
            0,
            "col=5 should keep black-2"
        );
        assert_eq!(
            state.cell_domains[0][5]
                & ((1 << 2) | (1 << 3) | (1 << 4) | SolverState::<6>::BLACK1_ROW),
            0,
            "col=5 should have digits 2-4 and black-1 cleared"
        );
    }

    #[test]
    fn inside_outside_rule_target_8_column() {
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
        assert_ne!(
            state.cell_domains[0][2] & (1 << 2),
            0,
            "row=0 should keep digit 2"
        );
        assert_ne!(
            state.cell_domains[0][2] & SolverState::<6>::BLACK1_COL,
            0,
            "row=0 should keep black-1-col"
        );
        assert_eq!(
            state.cell_domains[0][2]
                & ((1 << 1) | (1 << 3) | (1 << 4) | SolverState::<6>::BLACK2_COL),
            0,
            "row=0 should have digits 1,3,4 and black-2-col cleared"
        );
        // Row 5: only digit 2 or black-2-col remain (col bits).
        assert_ne!(
            state.cell_domains[5][2] & (1 << 2),
            0,
            "row=5 should keep digit 2"
        );
        assert_ne!(
            state.cell_domains[5][2] & SolverState::<6>::BLACK2_COL,
            0,
            "row=5 should keep black-2-col"
        );
        assert_eq!(
            state.cell_domains[5][2]
                & ((1 << 1) | (1 << 3) | (1 << 4) | SolverState::<6>::BLACK1_COL),
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
    fn set_cell_row_black_propagates() {
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

        // BLACK2_ROW is gone from every everything to the left, kept on the right
        for c in 0..1 {
            assert_eq!(
                state.cell_domains[2][c] & SolverState::<6>::BLACK2_ROW,
                0,
                "col {c} should lose BLACK2_ROW"
            );
        }
        for c in 2..6 {
            assert_ne!(
                state.cell_domains[2][c] & SolverState::<6>::BLACK2_ROW,
                0,
                "col {c} should keep BLACK2_ROW"
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

    #[test]
    fn single_inside_rule_assigns_target_digit() {
        // Row 0 has target 3. Pin BLACK1_ROW to col 1 and BLACK2_ROW to col 3
        // by stripping those bits from every other column in that row.
        // The only inside cell is col 2, which must be assigned digit 3.
        let mut state = SolverState::new(Puzzle::new([3, 0, 0, 0, 0, 0], [0; 6]));
        state.set_cell(0, 1, SolverState::<6>::BLACK1_ROW);
        state.set_cell(0, 3, SolverState::<6>::BLACK2_ROW);
        state.apply_single_inside_rule();

        assert_eq!(state.puzzle.board[0][2], Cell::Number(3));
        assert_eq!(
            state.cell_domains[0][2],
            1 << 3,
            "inside cell should hold only digit 3"
        );
    }

    #[test]
    fn single_inside_rule_partial_assignment() {
        // Row 0, target 6. BLACK1_ROW at col 0, BLACK2_ROW at col 4.
        // Inside: cols 1, 2, 3.  Col 1 = digit 2, col 3 = digit 1, col 2 = empty.
        // Expected: col 2 = 6 − 2 − 1 = 3.
        let mut state = SolverState::new(Puzzle::new([6, 0, 0, 0, 0, 0], [0; 6]));
        state.set_cell(0, 0, SolverState::<6>::BLACK1_ROW);
        state.set_cell(0, 4, SolverState::<6>::BLACK2_ROW);
        state.set_cell(0, 1, 1 << 2); // digit 2
        state.set_cell(0, 3, 1 << 1); // digit 1
        state.apply_single_inside_rule();

        assert_eq!(state.puzzle.board[0][2], Cell::Number(3));
        assert_eq!(
            state.cell_domains[0][2],
            1 << 3,
            "empty inside cell should be digit 3"
        );
    }

    // ── Newspaper puzzles ─────────────────────────────────────────────────────
    //
    // Integration tests: propagate a full puzzle and assert the exact Display
    // output.  Update the expected strings whenever the solver rules change.

    #[test]
    fn newspaper_puzzle_1() {
        let mut state = SolverState::new(Puzzle::new([8, 2, 3, 8, 9, 0], [0, 0, 5, 9, 0, 4]));
        state.propagate();
        assert_eq!(
            state.to_string(),
            concat!(
                "     0   0   5   9   0   4\n",
                "   +---+---+---+---+---+---+\n",
                " 8 | 2 | # | 3 | 1 | 4 | # |\n",
                "   +---+---+---+---+---+---+\n",
                " 2 | 1 | # | 2 | # | 3 | 4 |\n",
                "   +---+---+---+---+---+---+\n",
                " 3 | 3 | 4 | # | 2 | 1 | # |\n",
                "   +---+---+---+---+---+---+\n",
                " 8 | # | 3 | 1 | 4 | # | 2 |\n",
                "   +---+---+---+---+---+---+\n",
                " 9 | # | 2 | 4 | 3 | # | 1 |\n",
                "   +---+---+---+---+---+---+\n",
                " 0 | 4 | 1 | # | # | 2 | 3 |\n",
                "   +---+---+---+---+---+---+\n",
            )
        );
    }

    #[test]
    fn newspaper_puzzle_2() {
        let mut state = SolverState::new(Puzzle::new([3, 3, 5, 0, 7, 0], [5, 0, 2, 6, 5, 10]));
        state.propagate();
        assert_eq!(
            state.to_string(),
            concat!(
                "     5   0   2   6   5  10\n",
                "   +---+---+---+---+---+---+\n",
                " 3 | ⠇ | ⠇ | ⠃ | # | 3 | # |\n",
                "   +---+---+---+---+---+---+\n",
                " 3 | # | 3 | # | ⠇ | ⠇ | ⠇ |\n",
                "   +---+---+---+---+---+---+\n",
                " 5 | ⠃ | # | 2 | 3 | # | ⠃ |\n",
                "   +---+---+---+---+---+---+\n",
                " 0 | ⡇ | # | # | ⠇ | ⠇ | ⡇ |\n",
                "   +---+---+---+---+---+---+\n",
                " 7 | # | ⠇ | ⠇ | # | ⠃ | ⠇ |\n",
                "   +---+---+---+---+---+---+\n",
                " 0 | ⡇ | ⠇ | ⠇ | ⠃ | # | # |\n",
                "   +---+---+---+---+---+---+\n",
            )
        );
    }
}
