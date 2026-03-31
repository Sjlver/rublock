use std::fmt;
use std::sync::Arc;

// ── Puzzle ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Puzzle<const N: usize> {
    pub row_targets: [u8; N],
    pub col_targets: [u8; N],
}

impl<const N: usize> Puzzle<N> {
    pub fn new(row_targets: [u8; N], col_targets: [u8; N]) -> Self {
        Self {
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
pub struct SolverState<const N: usize> {
    pub puzzle: Puzzle<N>,
    domains: [[CellDomain; N]; N],
    tables: Arc<Tables>,
}

impl<const N: usize> SolverState<N> {
    pub fn new(puzzle: Puzzle<N>) -> Self {
        // All value bits set: bit 1 through bit N+2.
        let full_cell: CellDomain = ((1u64 << (N + 2)) - 1) << 1;
        Self {
            puzzle,
            domains: [[full_cell; N]; N],
            tables: Arc::new(Tables::build(N - 2)),
        }
    }
}

// ── Precomputed tables ────────────────────────────────────────────────────────
//
// `Tables` holds data derived purely from the grid size that is cheap to build
// but reused on every propagation pass.  It is computed once in
// `SolverState::new` and shared across all backtracking clones via `Arc`.
//
// All fields are `Vec`-based because their sizes depend on `num_digits = N-2`,
// which is only known at runtime.

#[derive(Debug)]
struct Tables {
    /// For each (target, size) pair, the list of valid digit-set bitmasks.
    ///
    /// A valid digit-set for cage target `t` and size `k` is any k-element
    /// subset of the digit set whose elements sum to `t`.  Each set is encoded
    /// as a `u64` with bit `d` set (i.e. `1 << d`) if digit `d` belongs to
    /// the set — the same layout used for cell domains.
    ///
    /// Indexed as `valid_tuples[target][size]`.
    valid_tuples: Vec<Vec<Vec<u64>>>,

    /// For each target `t`, the bitmask of digit bits that cannot appear inside
    /// (between the two blacks).  Digit `d` cannot be inside iff no valid tuple
    /// for target `t` contains it.
    ///
    /// By symmetry, `cant_be_inside[max_target - t]` gives the digits that
    /// cannot be outside, since inside and outside together always hold the
    /// full digit set.
    cant_be_inside: Vec<u64>,

    /// Minimum valid distance `d = p2 - p1` between the two blacks for each
    /// target.  `d` is valid iff `valid_tuples[t][d - 1]` is non-empty.
    d_min: Vec<usize>,

    /// Maximum valid distance `d = p2 - p1` between the two blacks for each
    /// target.
    d_max: Vec<usize>,
}

impl Tables {
    /// Build tables for a grid whose rows/columns contain `num_digits` distinct
    /// digit values (i.e. `num_digits = N - 2` for an N×N grid).
    fn build(num_digits: usize) -> Self {
        // Digits are 1..=num_digits; max achievable cage sum is their total.
        let max_target: usize = (1..=num_digits).sum();
        let num_targets = max_target + 1;

        // valid_tuples[target][size]: one Vec per (target, size) pair.
        let mut valid_tuples: Vec<Vec<Vec<u64>>> = vec![vec![vec![]; num_digits + 1]; num_targets];

        // Iterate over every subset of the digit set {1, …, num_digits}.
        // For each subset, its size and sum determine exactly which slot it
        // belongs in — no inner loops or filtering needed.
        for subset in 0u64..(1u64 << num_digits) {
            let size = subset.count_ones() as usize;
            let target: usize = (0..num_digits)
                .filter(|&b| subset & (1 << b) != 0)
                .map(|b| b + 1) // bit b represents digit b+1
                .sum();
            // Shift left by 1: bit b (digit b+1) → bit b+1 in the domain mask.
            valid_tuples[target][size].push(subset << 1);
        }

        // all_digits: bitmask of every digit bit (bits 1..=num_digits).
        let all_digits: u64 = ((1u64 << num_digits) - 1) << 1;

        // cant_be_inside[t]: digit bits absent from every valid tuple for target t.
        let cant_be_inside: Vec<u64> = (0..num_targets)
            .map(|t| {
                let can_be_inside: u64 = valid_tuples[t]
                    .iter()
                    .flat_map(|size_vec| size_vec.iter().copied())
                    .fold(0, |acc, tup| acc | tup);
                all_digits & !can_be_inside
            })
            .collect();

        // d_min[t] / d_max[t]: distance d = k+1 is valid for target t iff
        // valid_tuples[t][k] is non-empty.
        let mut d_min = vec![0usize; num_targets];
        let mut d_max = vec![0usize; num_targets];
        for t in 0..num_targets {
            let (lo, hi) = (0..=num_digits)
                .filter(|&k| !valid_tuples[t][k].is_empty())
                .map(|k| k + 1)
                // Every reachable target has at least one valid distance (t=0
                // uses d=1 with no cells between the blacks).
                .fold((usize::MAX, 0), |(lo, hi), d| (lo.min(d), hi.max(d)));
            d_min[t] = lo;
            d_max[t] = hi;
        }

        Self {
            valid_tuples,
            cant_be_inside,
            d_min,
            d_max,
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

    const ROW_BLACKS: u64 = Self::BLACK1_ROW | Self::BLACK2_ROW;
    const COL_BLACKS: u64 = Self::BLACK1_COL | Self::BLACK2_COL;
    const ALL_BLACKS: u64 = Self::ROW_BLACKS | Self::COL_BLACKS;

    /// Clear all bits in `mask` from a cell's domain.  Returns `true` iff any
    /// bit was actually set before (i.e. the domain shrank).
    fn clear_mask(domains: &mut [[CellDomain; 6]; 6], row: usize, col: usize, mask: u64) -> bool {
        let before = domains[row][col];
        domains[row][col] = before & !mask;
        domains[row][col] != before
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
            // Remove this digit from every other cell in the row and column.
            for c in (0..6).filter(|&c| c != col) {
                changed |= Self::clear_mask(&mut self.domains, row, c, bit);
            }
            for r in (0..6).filter(|&r| r != row) {
                changed |= Self::clear_mask(&mut self.domains, r, col, bit);
            }
            // Fix this cell: keep only this digit.
            changed |= Self::clear_mask(&mut self.domains, row, col, !bit);
        } else if bit & Self::ROW_BLACKS != 0 {
            // Each row-black variant appears once per row.
            for c in (0..6).filter(|&c| c != col) {
                changed |= Self::clear_mask(&mut self.domains, row, c, bit);
            }

            // Cell is black: drop digits and the other row-black variant.
            changed |= Self::clear_mask(
                &mut self.domains,
                row,
                col,
                Self::ALL_DIGITS | (Self::ROW_BLACKS & !bit),
            );
        } else if bit & Self::COL_BLACKS != 0 {
            // Each col-black variant appears once per column.
            for r in (0..6).filter(|&r| r != row) {
                changed |= Self::clear_mask(&mut self.domains, r, col, bit);
            }

            // Cell is black: drop digits and the other col-black variant.
            changed |= Self::clear_mask(
                &mut self.domains,
                row,
                col,
                Self::ALL_DIGITS | (Self::COL_BLACKS & !bit),
            );
        }

        if bit & Self::ALL_BLACKS != 0 {
            // Enforce ordering: clear all BLACK2 from cells above and to the
            // left, and clear BLACK1 from cells below and to the right.
            for left in 0..col {
                changed |= Self::clear_mask(&mut self.domains, row, left, Self::BLACK2_ROW)
            }
            for right in col + 1..6 {
                changed |= Self::clear_mask(&mut self.domains, row, right, Self::BLACK1_ROW)
            }
            for above in 0..row {
                changed |= Self::clear_mask(&mut self.domains, above, col, Self::BLACK2_COL)
            }
            for below in row + 1..6 {
                changed |= Self::clear_mask(&mut self.domains, below, col, Self::BLACK1_COL)
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
            let d_min = self.tables.d_min[t];
            let d_max = self.tables.d_max[t];
            for p in 0..6 {
                // BLACK1_ROW at p: need BLACK2_ROW somewhere in [p+d_min, p+d_max].
                let lo = p + d_min;
                let hi = (p + d_max + 1).min(6);
                if (lo..hi).all(|q| self.domains[r][q] & Self::BLACK2_ROW == 0) {
                    changed |= Self::clear_mask(&mut self.domains, r, p, Self::BLACK1_ROW);
                }
                // BLACK2_ROW at p: need BLACK1_ROW somewhere in [p-d_max, p-d_min].
                let hi2 = (p + 1).saturating_sub(d_min);
                let lo2 = p.saturating_sub(d_max);
                if (lo2..hi2).all(|q| self.domains[r][q] & Self::BLACK1_ROW == 0) {
                    changed |= Self::clear_mask(&mut self.domains, r, p, Self::BLACK2_ROW);
                }
            }
        }

        for c in 0..6 {
            let t = self.puzzle.col_targets[c] as usize;
            let d_min = self.tables.d_min[t];
            let d_max = self.tables.d_max[t];
            for p in 0..6 {
                let lo = p + d_min;
                let hi = (p + d_max + 1).min(6);
                if (lo..hi).all(|q| self.domains[q][c] & Self::BLACK2_COL == 0) {
                    changed |= Self::clear_mask(&mut self.domains, p, c, Self::BLACK1_COL);
                }
                let hi2 = (p + 1).saturating_sub(d_min);
                let lo2 = p.saturating_sub(d_max);
                if (lo2..hi2).all(|q| self.domains[q][c] & Self::BLACK1_COL == 0) {
                    changed |= Self::clear_mask(&mut self.domains, p, c, Self::BLACK2_COL);
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
            let cant_inside = self.tables.cant_be_inside[t];
            let cant_outside = self.tables.cant_be_inside[10 - t];

            for p in 0..6 {
                let b1_before = (0..p).any(|q| self.domains[r][q] & Self::BLACK1_ROW != 0);
                let b2_before = (0..p).any(|q| self.domains[r][q] & Self::BLACK2_ROW != 0);
                let b1_after = (p + 1..6).any(|q| self.domains[r][q] & Self::BLACK1_ROW != 0);
                let b2_after = (p + 1..6).any(|q| self.domains[r][q] & Self::BLACK2_ROW != 0);

                if !b1_before || !b2_after {
                    changed |= Self::clear_mask(&mut self.domains, r, p, cant_outside);
                }
                if !b1_after && !b2_before {
                    changed |= Self::clear_mask(&mut self.domains, r, p, cant_inside);
                }
            }
        }

        for c in 0..6 {
            let t = self.puzzle.col_targets[c] as usize;
            let cant_inside = self.tables.cant_be_inside[t];
            let cant_outside = self.tables.cant_be_inside[10 - t];

            for p in 0..6 {
                let b1_before = (0..p).any(|q| self.domains[q][c] & Self::BLACK1_COL != 0);
                let b2_before = (0..p).any(|q| self.domains[q][c] & Self::BLACK2_COL != 0);
                let b1_after = (p + 1..6).any(|q| self.domains[q][c] & Self::BLACK1_COL != 0);
                let b2_after = (p + 1..6).any(|q| self.domains[q][c] & Self::BLACK2_COL != 0);

                if !b1_before || !b2_after {
                    changed |= Self::clear_mask(&mut self.domains, p, c, cant_outside);
                }
                if !b1_after && !b2_before {
                    changed |= Self::clear_mask(&mut self.domains, p, c, cant_inside);
                }
            }
        }

        changed
    }

    /// Rule: if a cell's domain has shrunk to a single bit, assign it.
    ///
    /// A singleton domain means there is only one possible value — call
    /// `set_cell` to fix it and propagate.
    fn apply_singleton_rule(&mut self) -> bool {
        let mut changed = false;

        for r in 0..6 {
            for c in 0..6 {
                let domain = self.domains[r][c];
                if domain.count_ones() == 1 {
                    changed |= self.set_cell(r, c, domain);
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
                let domain = self.domains[r][c];
                if domain & Self::ROW_BLACKS == 0 {
                    changed |= Self::clear_mask(&mut self.domains, r, c, Self::COL_BLACKS);
                }
                if domain & Self::COL_BLACKS == 0 {
                    changed |= Self::clear_mask(&mut self.domains, r, c, Self::ROW_BLACKS);
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
            if self.domains[r][c] & bit != 0 {
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
            if self.domains[r][c] & bit != 0 {
                if found.is_some() {
                    return None;
                }
                found = Some(r);
            }
        }
        found
    }

    // ── Cage helpers ─────────────────────────────────────────────────────────

    /// Apply cage filtering to a set of cells given as `(row, col)` pairs.
    ///
    /// Looks up all valid digit-sets for (target, k) in `VALID_TUPLES`, keeps
    /// only those where every cage cell's domain intersects the set, then
    /// removes from every cage cell any digit absent from the union.
    fn apply_cage(
        domains: &mut [[CellDomain; 6]; 6],
        cells: &[(usize, usize)],
        target: usize,
        tuples: &[Vec<Vec<u64>>],
    ) -> bool {
        let k = cells.len();
        if k == 0 {
            return false;
        }
        let cell_domains: Vec<u64> = cells
            .iter()
            .map(|&(r, c)| domains[r][c] & Self::ALL_DIGITS)
            .collect();
        let union: u64 = tuples[target][k]
            .iter()
            .filter(|&&tup| cell_domains.iter().all(|&d| d & tup != 0))
            .fold(0, |a, &tup| a | tup);
        let mut changed = false;
        for &(r, c) in cells {
            changed |= Self::clear_mask(domains, r, c, Self::ALL_DIGITS & !union);
        }
        changed
    }

    /// Rule: once both black squares in a row/column are pinned, apply cage-
    /// based arc consistency to the inside cage (sum = target) and the outside
    /// cage (sum = 10 − target).
    ///
    /// For each cage, all valid digit-sets of the right size and sum are
    /// looked up from `VALID_TUPLES`.  A set is *feasible* if every cage cell
    /// has at least one digit from that set available.  The union of all
    /// feasible sets bounds the digits that can appear in the cage; any digit
    /// outside the union is removed from every cage cell's domain.
    ///
    /// This subsumes `apply_single_inside_rule`: when only one inside cell is
    /// unassigned, exactly one digit-set is feasible and the union pins the
    /// remaining digit.
    fn apply_inside_outside_cage_rule(&mut self) -> bool {
        let tuples = &self.tables.valid_tuples;
        let mut changed = false;

        for r in 0..6 {
            let t = self.puzzle.row_targets[r] as usize;
            let Some(b1) = self.singleton_in_row(r, Self::BLACK1_ROW) else {
                continue;
            };
            let Some(b2) = self.singleton_in_row(r, Self::BLACK2_ROW) else {
                continue;
            };
            // A well-formed row has b1 < b2 (BLACK1 is left of BLACK2 by
            // definition).  If they are equal or reversed, the state is
            // transiently contradicted and `apply_black_range_rules` will
            // clean it up on the next pass; skip the cage rule for now.
            if b1 >= b2 {
                continue;
            }
            let inside: Vec<(usize, usize)> = (b1 + 1..b2).map(|c| (r, c)).collect();
            let outside: Vec<(usize, usize)> = (0..b1).chain(b2 + 1..6).map(|c| (r, c)).collect();
            changed |= Self::apply_cage(&mut self.domains, &inside, t, tuples);
            changed |= Self::apply_cage(&mut self.domains, &outside, 10 - t, tuples);
        }

        for c in 0..6 {
            let t = self.puzzle.col_targets[c] as usize;
            let Some(b1) = self.singleton_in_col(c, Self::BLACK1_COL) else {
                continue;
            };
            let Some(b2) = self.singleton_in_col(c, Self::BLACK2_COL) else {
                continue;
            };
            if b1 >= b2 {
                continue;
            }
            let inside: Vec<(usize, usize)> = (b1 + 1..b2).map(|r| (r, c)).collect();
            let outside: Vec<(usize, usize)> = (0..b1).chain(b2 + 1..6).map(|r| (r, c)).collect();
            changed |= Self::apply_cage(&mut self.domains, &inside, t, tuples);
            changed |= Self::apply_cage(&mut self.domains, &outside, 10 - t, tuples);
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
                | self.apply_inside_outside_cage_rule();
            if !changed {
                break;
            }
        }
    }

    // ── Backtracking search ───────────────────────────────────────────────────

    /// Returns `true` if any cell's domain is empty, indicating the current
    /// partial assignment is contradictory and this branch can be pruned.
    pub fn is_contradiction(&self) -> bool {
        self.domains.iter().flatten().any(|&d| d == 0)
    }

    /// Returns `true` if every cell has been uniquely determined.
    ///
    /// A digit cell is fully determined when it has exactly one digit bit and
    /// no black bits.  A black cell is fully determined when it has exactly one
    /// row-black bit and one col-black bit (the ROW and COL orderings are
    /// independent, so both must be pinned).
    pub fn is_solved(&self) -> bool {
        self.domains.iter().flatten().all(|&d| {
            let digits = d & Self::ALL_DIGITS;
            let row_blacks = d & Self::ROW_BLACKS;
            let col_blacks = d & Self::COL_BLACKS;
            if digits != 0 {
                // Digit cell: exactly one digit, no black bits.
                digits.count_ones() == 1 && row_blacks == 0 && col_blacks == 0
            } else {
                // Black cell: exactly one row ordering and one col ordering.
                row_blacks.count_ones() == 1 && col_blacks.count_ones() == 1
            }
        })
    }

    /// Return the bits to branch on for the given cell domain.
    ///
    /// To avoid double-counting solutions, we branch on ONE dimension at a time:
    ///
    /// 1. **Row identity**: is this cell digit-1, digit-2, …, digit-4,
    ///    BLACK1_ROW, or BLACK2_ROW?  These six options are mutually exclusive —
    ///    picking one commits the cell's identity in its row.
    /// 2. **Column ordering**: only after the row identity is pinned to a black
    ///    do we branch on whether the cell is BLACK1_COL or BLACK2_COL.
    ///
    /// Returns `0` when the cell is fully determined (no branching needed).
    fn branching_bits(domain: u64) -> u64 {
        let primary = domain & (Self::ALL_DIGITS | Self::ROW_BLACKS);
        if primary.count_ones() > 1 {
            return primary;
        }
        let col_blacks = domain & Self::COL_BLACKS;
        if col_blacks.count_ones() > 1 {
            return col_blacks;
        }
        0
    }

    /// Find the most-constrained unsettled cell (the one with the fewest
    /// remaining choices), using `branching_bits` as the measure.
    ///
    /// Returns `None` when every cell is already fully determined.
    fn pick_branching_cell(&self) -> Option<(usize, usize)> {
        let mut best: Option<(usize, usize, u32)> = None;
        for r in 0..6 {
            for c in 0..6 {
                let bits = Self::branching_bits(self.domains[r][c]);
                let freedom = bits.count_ones();
                if freedom > 1 && best.map_or(true, |b| freedom < b.2) {
                    best = Some((r, c, freedom));
                }
            }
        }
        best.map(|(r, c, _)| (r, c))
    }

    /// Count the number of distinct solutions, stopping once `max` is reached.
    ///
    /// Returns the number of solutions found, which is at most `max`.
    ///
    /// Practical usage:
    /// - Pass `max = 1` to test satisfiability.
    /// - Pass `max = 2` to cheaply distinguish "unique solution" (returns 1)
    ///   from "multiple solutions" (returns 2), which is what puzzle validation
    ///   needs — no point counting further once we know uniqueness is broken.
    ///
    /// The method clones the solver state before each candidate branch so that
    /// constraint-propagation side-effects don't leak across sibling branches.
    pub fn count_solutions(&self, max: usize) -> usize {
        if max == 0 {
            return 0;
        }

        let mut state = self.clone();
        state.propagate();

        if state.is_contradiction() {
            return 0;
        }
        if state.is_solved() {
            return 1;
        }

        let Some((row, col)) = state.pick_branching_cell() else {
            // Propagation stalled but the grid is neither solved nor
            // contradicted.  This shouldn't happen for well-formed 6×6
            // inputs, but returning 0 is safer than panicking.
            return 0;
        };

        let bits = Self::branching_bits(state.domains[row][col]);
        let mut total = 0;
        let mut bit = 1u64;
        while bit <= bits {
            if bits & bit != 0 {
                let mut branch = state.clone();
                branch.set_cell(row, col, bit);
                total += branch.count_solutions(max - total);
                if total >= max {
                    return total;
                }
            }
            bit <<= 1;
        }
        total
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
                let domain = self.domains[r][c];
                if domain & Self::ALL_DIGITS == 0 && domain != 0 {
                    write!(f, " # |")?;
                } else if domain.count_ones() == 1 {
                    write!(f, "{:2} |", domain.trailing_zeros())?;
                } else {
                    let sym = match domain.count_ones() {
                        0 => " X ", // contradiction
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
            writeln!(f)?;
            writeln!(f, "   +---+---+---+---+---+---+")?;
        }
        Ok(())
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cell_domain_all_values_possible() {
        let state = SolverState::new(Puzzle::new([0; 6], [0; 6]));
        // 8 bits set: numbers 1-4 and the four variants of black
        assert_eq!(state.domains[0][0], 0b111111110);
        assert_eq!(state.domains[5][5], 0b111111110);
    }

    // ── Solver rule tests ─────────────────────────────────────────────────────

    #[test]
    fn black1_row_always_forbidden_at_last_position() {
        // Black-1 can never sit at position 5, even for target = 0.
        let mut state = SolverState::new(Puzzle::new([0; 6], [0; 6]));
        state.propagate();
        for r in 0..6 {
            assert_eq!(
                state.domains[r][5] & SolverState::<6>::BLACK1_ROW,
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
            state.domains[0][0] & SolverState::<6>::BLACK1_ROW,
            0,
            "p=0 should still be allowed"
        );
        assert_ne!(
            state.domains[0][1] & SolverState::<6>::BLACK1_ROW,
            0,
            "p=1 should still be allowed"
        );
        for p in 2..6 {
            assert_eq!(
                state.domains[0][p] & SolverState::<6>::BLACK1_ROW,
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
                state.domains[0][c] & (1 << 1),
                0,
                "digit 1 should be cleared from middle cell (row=0, col={c})"
            );
        }
        // Position 0: only digit 1 or black-1 remain (row bits).
        assert_ne!(
            state.domains[0][0] & (1 << 1),
            0,
            "col=0 should keep digit 1"
        );
        assert_ne!(
            state.domains[0][0] & SolverState::<6>::BLACK1_ROW,
            0,
            "col=0 should keep black-1"
        );
        assert_eq!(
            state.domains[0][0] & ((1 << 2) | (1 << 3) | (1 << 4) | SolverState::<6>::BLACK2_ROW),
            0,
            "col=0 should have digits 2-4 and black-2 cleared"
        );
        // Position 5: only digit 1 or black-2 remain (row bits).
        assert_ne!(
            state.domains[0][5] & (1 << 1),
            0,
            "col=5 should keep digit 1"
        );
        assert_ne!(
            state.domains[0][5] & SolverState::<6>::BLACK2_ROW,
            0,
            "col=5 should keep black-2"
        );
        assert_eq!(
            state.domains[0][5] & ((1 << 2) | (1 << 3) | (1 << 4) | SolverState::<6>::BLACK1_ROW),
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
                state.domains[r][2] & (1 << 2),
                0,
                "digit 2 should be cleared from middle cell (row={r}, col=2)"
            );
        }
        // Row 0: only digit 2 or black-1-col remain (col bits).
        assert_ne!(
            state.domains[0][2] & (1 << 2),
            0,
            "row=0 should keep digit 2"
        );
        assert_ne!(
            state.domains[0][2] & SolverState::<6>::BLACK1_COL,
            0,
            "row=0 should keep black-1-col"
        );
        assert_eq!(
            state.domains[0][2] & ((1 << 1) | (1 << 3) | (1 << 4) | SolverState::<6>::BLACK2_COL),
            0,
            "row=0 should have digits 1,3,4 and black-2-col cleared"
        );
        // Row 5: only digit 2 or black-2-col remain (col bits).
        assert_ne!(
            state.domains[5][2] & (1 << 2),
            0,
            "row=5 should keep digit 2"
        );
        assert_ne!(
            state.domains[5][2] & SolverState::<6>::BLACK2_COL,
            0,
            "row=5 should keep black-2-col"
        );
        assert_eq!(
            state.domains[5][2] & ((1 << 1) | (1 << 3) | (1 << 4) | SolverState::<6>::BLACK1_COL),
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
        assert_eq!(state.domains[0][0], 1 << 3);

        // Digit 3 is gone from the rest of row 0 and col 0.
        for c in 1..6 {
            assert_eq!(state.domains[0][c] & (1 << 3), 0, "row 0, col {c}");
        }
        for r in 1..6 {
            assert_eq!(state.domains[r][0] & (1 << 3), 0, "row {r}, col 0");
        }

        // An unrelated cell (1, 1) is completely untouched.
        assert_eq!(state.domains[1][1], 0b111111110);
    }

    #[test]
    fn set_cell_row_black_propagates() {
        let mut state = SolverState::new(Puzzle::new([0; 6], [0; 6]));
        state.set_cell(2, 1, SolverState::<6>::BLACK1_ROW);

        // The assigned cell keeps BLACK1_ROW and both col-black bits, but
        // loses digits and BLACK2_ROW.
        let d = state.domains[2][1];
        assert_ne!(d & SolverState::<6>::BLACK1_ROW, 0, "keep BLACK1_ROW");
        assert_ne!(d & SolverState::<6>::BLACK1_COL, 0, "keep BLACK1_COL");
        assert_ne!(d & SolverState::<6>::BLACK2_COL, 0, "keep BLACK2_COL");
        assert_eq!(d & SolverState::<6>::BLACK2_ROW, 0, "drop BLACK2_ROW");
        assert_eq!(d & SolverState::<6>::ALL_DIGITS, 0, "drop all digits");

        // BLACK1_ROW is gone from every other cell in row 2.
        for c in (0..6).filter(|&c| c != 1) {
            assert_eq!(
                state.domains[2][c] & SolverState::<6>::BLACK1_ROW,
                0,
                "col {c} should lose BLACK1_ROW"
            );
        }

        // BLACK2_ROW is gone from every everything to the left, kept on the right
        for c in 0..1 {
            assert_eq!(
                state.domains[2][c] & SolverState::<6>::BLACK2_ROW,
                0,
                "col {c} should lose BLACK2_ROW"
            );
        }
        for c in 2..6 {
            assert_ne!(
                state.domains[2][c] & SolverState::<6>::BLACK2_ROW,
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
                state.domains[0][c] & SolverState::<6>::BLACK2_ROW,
                0,
                "col {c} should lose BLACK2_ROW (left of black-1)"
            );
        }
        for c in 4..6 {
            assert_ne!(
                state.domains[0][c] & SolverState::<6>::BLACK2_ROW,
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
            state.domains[0][c] &= !SolverState::<6>::BLACK2_ROW;
        }
        state.apply_black_range_rules();

        assert_ne!(
            state.domains[0][2] & SolverState::<6>::BLACK1_ROW,
            0,
            "col 2 should keep BLACK1_ROW (adjacent to the only BLACK2_ROW at col 3)"
        );
        for p in (0..6).filter(|&p| p != 2) {
            assert_eq!(
                state.domains[0][p] & SolverState::<6>::BLACK1_ROW,
                0,
                "col {p} should lose BLACK1_ROW"
            );
        }
    }

    #[test]
    fn apply_singleton_rule_assigns_sole_digit() {
        let mut state = SolverState::new(Puzzle::new([0; 6], [0; 6]));
        // Force cell (3, 3) to have only digit 2 in its domain.
        state.domains[3][3] = 1 << 2;
        // Run just this one rule (not propagate, to isolate it).
        state.apply_singleton_rule();

        assert_eq!(state.domains[3][3], 1 << 2);
        // Digit 2 should be gone from the rest of row 3 and col 3.
        for c in (0..6).filter(|&c| c != 3) {
            assert_eq!(state.domains[3][c] & (1 << 2), 0);
        }
        for r in (0..6).filter(|&r| r != 3) {
            assert_eq!(state.domains[r][3] & (1 << 2), 0);
        }
    }

    #[test]
    fn apply_hidden_single_rule_assigns_forced_digit() {
        let mut state = SolverState::new(Puzzle::new([0; 6], [0; 6]));
        // Remove digit 4 from every cell in row 0 except column 2.
        for c in (0..6).filter(|&c| c != 2) {
            state.domains[0][c] &= !(1 << 4);
        }
        state.apply_hidden_single_rule();

        assert_eq!(state.domains[0][2], 1 << 4);
    }

    #[test]
    fn apply_black_consistency_rule_clears_col_blacks_when_no_row_blacks() {
        let mut state = SolverState::new(Puzzle::new([0; 6], [0; 6]));
        // Strip all row-black bits from cell (1, 4).
        state.domains[1][4] &= !SolverState::<6>::ROW_BLACKS;
        state.apply_black_consistency_rule();

        // Col-black bits must now also be gone.
        assert_eq!(state.domains[1][4] & SolverState::<6>::COL_BLACKS, 0);
        // But digit bits are intact.
        assert_eq!(
            state.domains[1][4] & SolverState::<6>::ALL_DIGITS,
            SolverState::<6>::ALL_DIGITS
        );
    }

    #[test]
    fn cage_rule_sole_inside_cell_narrows_to_target_digit() {
        // Row 0 has target 3. Pin BLACK1_ROW to col 1 and BLACK2_ROW to col 3.
        // The only inside cell is col 2; the only feasible tuple is {3}, so
        // the cage rule must narrow that cell's digit domain to just digit 3.
        let mut state = SolverState::new(Puzzle::new([3, 0, 0, 0, 0, 0], [0; 6]));
        state.set_cell(0, 1, SolverState::<6>::BLACK1_ROW);
        state.set_cell(0, 3, SolverState::<6>::BLACK2_ROW);
        state.apply_inside_outside_cage_rule();

        assert_eq!(
            state.domains[0][2] & SolverState::<6>::ALL_DIGITS,
            1 << 3,
            "inside cell's digit domain should be reduced to just digit 3"
        );
    }

    #[test]
    fn cage_rule_partial_assignment_narrows_remaining_digit() {
        // Row 0, target 6. BLACK1_ROW at col 0, BLACK2_ROW at col 4.
        // Inside: cols 1, 2, 3.  Col 1 = digit 2, col 3 = digit 1.
        // Only feasible inside tuple: {1, 2, 3}, so col 2 must be digit 3.
        let mut state = SolverState::new(Puzzle::new([6, 0, 0, 0, 0, 0], [0; 6]));
        state.set_cell(0, 0, SolverState::<6>::BLACK1_ROW);
        state.set_cell(0, 4, SolverState::<6>::BLACK2_ROW);
        state.set_cell(0, 1, 1 << 2); // digit 2
        state.set_cell(0, 3, 1 << 1); // digit 1
        state.apply_inside_outside_cage_rule();

        assert_eq!(
            state.domains[0][2] & SolverState::<6>::ALL_DIGITS,
            1 << 3,
            "empty inside cell's digit domain should be reduced to just digit 3"
        );
    }

    // ── Backtracking tests ────────────────────────────────────────────────────

    #[test]
    fn count_solutions_returns_1_for_unique_puzzle() {
        // Both newspaper puzzles should have exactly one solution.
        let state = SolverState::new(Puzzle::new([8, 2, 3, 8, 9, 0], [0, 0, 5, 9, 0, 4]));
        assert_eq!(state.count_solutions(2), 1);

        let state = SolverState::new(Puzzle::new([3, 3, 5, 0, 7, 0], [5, 0, 2, 6, 5, 10]));
        assert_eq!(state.count_solutions(2), 1);
    }

    #[test]
    fn count_solutions_returns_0_for_impossible_puzzle() {
        // Targets that cannot be satisfied: all targets = 1 requires a 1-cell
        // gap in every row and column, which is impossible to satisfy globally.
        let state = SolverState::new(Puzzle::new([1; 6], [1; 6]));
        assert_eq!(state.count_solutions(1), 0);
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
                " 3 | 2 | 1 | 4 | # | 3 | # |\n",
                "   +---+---+---+---+---+---+\n",
                " 3 | # | 3 | # | 1 | 2 | 4 |\n",
                "   +---+---+---+---+---+---+\n",
                " 5 | 4 | # | 2 | 3 | # | 1 |\n",
                "   +---+---+---+---+---+---+\n",
                " 0 | 1 | # | # | 2 | 4 | 3 |\n",
                "   +---+---+---+---+---+---+\n",
                " 7 | # | 4 | 3 | # | 1 | 2 |\n",
                "   +---+---+---+---+---+---+\n",
                " 0 | 3 | 2 | 1 | 4 | # | # |\n",
                "   +---+---+---+---+---+---+\n",
            )
        );
    }
}
