use std::fmt;
use std::sync::Arc;

use crate::changeset::ChangeSet;
use crate::grid::{Cell, Grid};
use crate::recorder::{Recorder, Rule, SearchNodes};
use crate::solver::{CellDomain, Puzzle, Solver, Tables};

// ── BasicSolverState ───────────────────────────────────────────────────────────────
//
// Working state during search.
//
// Clone but not Copy: cloning is explicit and used for backtracking (save a
// snapshot before committing to a guess). Copy is intentionally absent —
// accidental copies of this large struct would silently produce stale state.

/// State of the basic solver, generic over the propagation event recorder
/// (see [`crate::recorder`]).  Defaults to [`SearchNodes`].
#[derive(Clone)]
pub struct BasicSolverState<const N: usize, R: Recorder = SearchNodes> {
    pub puzzle: Puzzle<N>,
    domains: [[CellDomain; N]; N],
    tables: Arc<Tables>,
    recorder: R,
}

impl<const N: usize, R: Recorder> BasicSolverState<N, R> {
    pub fn with_recorder(puzzle: Puzzle<N>) -> Self {
        // All value bits set: bit 1 through bit N+2.
        let full_cell: CellDomain = ((1 << (N + 2)) - 1) << 1;
        Self {
            puzzle,
            domains: [[full_cell; N]; N],
            tables: Arc::new(Tables::build(N - 2)),
            recorder: R::default(),
        }
    }

    /// The recorder this state writes events to.
    pub fn recorder(&self) -> &R {
        &self.recorder
    }
}

impl<const N: usize> BasicSolverState<N, SearchNodes> {
    /// Construct a solver with the default [`SearchNodes`] recorder.
    pub fn new(puzzle: Puzzle<N>) -> Self {
        Self::with_recorder(puzzle)
    }
}

// We implement `Debug` manually to avoid requiring `R: Debug` and to keep
// the printed output focused on the state we care about — puzzle targets
// and the domain grid.
impl<const N: usize, R: Recorder> fmt::Debug for BasicSolverState<N, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BasicSolverState")
            .field("puzzle", &self.puzzle)
            .field("domains", &self.domains)
            .finish()
    }
}

// ── Solver rules ──────────────────────────────────────────────────────────────

impl<const N: usize, R: Recorder> BasicSolverState<N, R> {
    // Bit positions for the "black" value variants.
    const BLACK1_ROW: CellDomain = 1 << (N - 1);
    const BLACK2_ROW: CellDomain = 1 << N;
    const BLACK1_COL: CellDomain = 1 << (N + 1);
    const BLACK2_COL: CellDomain = 1 << (N + 2);

    // Composite masks for common groups of bits.
    // Digit bits occupy positions 1..=(N-2); ALL_DIGITS sets exactly those bits.
    const ALL_DIGITS: CellDomain = ((1 << (N - 2)) - 1) << 1;

    const ROW_BLACKS: CellDomain = Self::BLACK1_ROW | Self::BLACK2_ROW;
    const COL_BLACKS: CellDomain = Self::BLACK1_COL | Self::BLACK2_COL;
    const ALL_BLACKS: CellDomain = Self::ROW_BLACKS | Self::COL_BLACKS;

    /// Clear all bits in `mask` from a cell's domain.  Returns a `ChangeSet`
    /// with the cell's row and column set iff any bit was actually cleared (i.e.
    /// the domain shrank); otherwise returns an empty `ChangeSet`.
    ///
    /// The `rule` tag is used to attribute the bits removed here to the
    /// propagation rule that triggered the call (debug-only; see `stats.rs`).
    fn clear_mask(&mut self, row: usize, col: usize, mask: CellDomain, rule: Rule) -> ChangeSet {
        let mut cs = ChangeSet::default();
        let before = self.domains[row][col];
        let after = before & !mask;
        self.domains[row][col] = after;
        if after != before {
            self.recorder.on_bits_removed(row, col, before, after, rule);
            cs.set_row(row);
            cs.set_col(col);
        }
        cs
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
    ///
    /// `set_cell` is a helper used by several rules, so the caller passes the
    /// `rule` tag it wants to attribute these removals to.
    fn set_cell(&mut self, row: usize, col: usize, bit: CellDomain, rule: Rule) -> ChangeSet {
        debug_assert_eq!(bit.count_ones(), 1, "set_cell requires exactly one bit");
        let mut changed = ChangeSet::default();

        if bit & Self::ALL_DIGITS != 0 {
            // Remove this digit from every other cell in the row and column.
            for c in (0..N).filter(|&c| c != col) {
                changed |= self.clear_mask(row, c, bit, rule);
            }
            for r in (0..N).filter(|&r| r != row) {
                changed |= self.clear_mask(r, col, bit, rule);
            }
            // Fix this cell: keep only this digit.
            changed |= self.clear_mask(row, col, !bit, rule);
        } else if bit & Self::ROW_BLACKS != 0 {
            // Each row-black variant appears once per row.
            for c in (0..N).filter(|&c| c != col) {
                changed |= self.clear_mask(row, c, bit, rule);
            }

            // Cell is black: drop digits and the other row-black variant.
            changed |=
                self.clear_mask(row, col, Self::ALL_DIGITS | (Self::ROW_BLACKS & !bit), rule);
        } else if bit & Self::COL_BLACKS != 0 {
            // Each col-black variant appears once per column.
            for r in (0..N).filter(|&r| r != row) {
                changed |= self.clear_mask(r, col, bit, rule);
            }

            // Cell is black: drop digits and the other col-black variant.
            changed |=
                self.clear_mask(row, col, Self::ALL_DIGITS | (Self::COL_BLACKS & !bit), rule);
        }

        if bit & Self::ALL_BLACKS != 0 {
            // Enforce ordering: clear all BLACK2 from cells above and to the
            // left, and clear BLACK1 from cells below and to the right.
            for left in 0..col {
                changed |= self.clear_mask(row, left, Self::BLACK2_ROW, rule);
            }
            for right in col + 1..N {
                changed |= self.clear_mask(row, right, Self::BLACK1_ROW, rule);
            }
            for above in 0..row {
                changed |= self.clear_mask(above, col, Self::BLACK2_COL, rule);
            }
            for below in row + 1..N {
                changed |= self.clear_mask(below, col, Self::BLACK1_COL, rule);
            }
        }

        changed
    }

    // ── Rules ────────────────────────────────────────────────────────────────

    /// Returns true if the given `pattern` has support in `cells`, meaning
    /// that each cell is compatible with the pattern, and any bit in pattern
    /// is supported by at least one cell.
    fn is_pattern_supported(&self, pattern: &[CellDomain], cells: &[(usize, usize)]) -> bool {
        debug_assert_eq!(pattern.len(), cells.len());
        let pattern_bits = pattern.iter().fold(0 as CellDomain, |acc, &b| acc | b);
        let mut supported: CellDomain = 0;
        for (&p, &(r, c)) in pattern.iter().zip(cells) {
            let s = self.domains[r][c] & p;
            if s == 0 {
                return false;
            }
            supported |= s;
        }
        supported == pattern_bits
    }

    /// Update `mask` with bits supported by `pattern` placed at `cells` (row scan).
    fn mark_row_pattern_supported(
        mask: &mut [CellDomain],
        pattern: &[CellDomain],
        cells: &[(usize, usize)],
    ) {
        for (&p, &(_, c)) in pattern.iter().zip(cells) {
            mask[c] |= p;
        }
    }

    /// Update `mask` with bits supported by `pattern` placed at `cells` (col scan).
    fn mark_col_pattern_supported(
        mask: &mut [CellDomain],
        pattern: &[CellDomain],
        cells: &[(usize, usize)],
    ) {
        for (&p, &(r, _)) in pattern.iter().zip(cells) {
            mask[r] |= p;
        }
    }

    /// Returns `len` cell positions starting at `(row, col)`. If `wrap` is set, the
    /// range must wrap over at the far boundary. Otherwise it must not.
    fn row_cells(row: usize, col: usize, len: usize, wrap: bool) -> Option<Vec<(usize, usize)>> {
        let end = col + len;
        if wrap {
            if end <= N {
                return None;
            }
            Some((col..N).chain(0..end % N).map(|c| (row, c)).collect())
        } else {
            if end > N {
                return None;
            }
            Some((col..end).map(|c| (row, c)).collect())
        }
    }

    /// Same as `row_cells`, but vertical.
    fn col_cells(row: usize, col: usize, len: usize, wrap: bool) -> Option<Vec<(usize, usize)>> {
        let end = row + len;
        if wrap {
            if end <= N {
                return None;
            }
            Some((row..N).chain(0..end % N).map(|r| (r, col)).collect())
        } else {
            if end > N {
                return None;
            }
            Some((row..end).map(|r| (r, col)).collect())
        }
    }

    /// Rule: domain bits must have support from at least one valid tuple.
    ///
    /// This applies all tuples that could possibly match at all possible
    /// positions. It computes the union of the tuple bits. Any domain bit
    /// that is not in this union isn't supported by any tuple, and can
    /// be removed.
    fn apply_general_arc_consistency(&mut self, prev: ChangeSet) -> ChangeSet {
        let mut changed = ChangeSet::default();

        for row in prev.iter_rows() {
            let inside_target = self.puzzle.row_targets[row] as usize;
            let outside_target = self.tables.max_sum - inside_target;

            let patterns: Vec<(Vec<CellDomain>, bool)> =
                self.tables
                    .valid_tuples_for_target(inside_target)
                    .map(|(len, tuple)| {
                        (
                            std::iter::once(Self::BLACK1_ROW)
                                .chain(std::iter::repeat_n(tuple, len))
                                .chain(std::iter::once(Self::BLACK2_ROW))
                                .collect(),
                            false,
                        )
                    })
                    .chain(self.tables.valid_tuples_for_target(outside_target).map(
                        |(len, tuple)| {
                            (
                                std::iter::once(Self::BLACK2_ROW)
                                    .chain(std::iter::repeat_n(tuple, len))
                                    .chain(std::iter::once(Self::BLACK1_ROW))
                                    .collect(),
                                true,
                            )
                        },
                    ))
                    .collect();

            // Accumulate bits supported by at least one pattern. Note that we don't touch
            // column blacks, only row bits.
            let mut mask = [Self::COL_BLACKS; N];

            for (pattern, wrap) in patterns {
                for col in 0..N {
                    let Some(cells) = Self::row_cells(row, col, pattern.len(), wrap) else {
                        continue;
                    };
                    if self.is_pattern_supported(&pattern, &cells) {
                        Self::mark_row_pattern_supported(&mut mask, &pattern, &cells);
                    }
                }
            }

            // Now, mask is the union of all patterns that could possibly match this row.
            for (c, &m) in mask.iter().enumerate() {
                changed |= self.clear_mask(row, c, !m, Rule::ArcConsistency);
            }
        }

        for col in prev.iter_cols() {
            let inside_target = self.puzzle.col_targets[col] as usize;
            let outside_target = self.tables.max_sum - inside_target;

            let patterns: Vec<(Vec<CellDomain>, bool)> =
                self.tables
                    .valid_tuples_for_target(inside_target)
                    .map(|(len, tuple)| {
                        (
                            std::iter::once(Self::BLACK1_COL)
                                .chain(std::iter::repeat_n(tuple, len))
                                .chain(std::iter::once(Self::BLACK2_COL))
                                .collect(),
                            false,
                        )
                    })
                    .chain(self.tables.valid_tuples_for_target(outside_target).map(
                        |(len, tuple)| {
                            (
                                std::iter::once(Self::BLACK2_COL)
                                    .chain(std::iter::repeat_n(tuple, len))
                                    .chain(std::iter::once(Self::BLACK1_COL))
                                    .collect(),
                                true,
                            )
                        },
                    ))
                    .collect();

            let mut mask = [Self::ROW_BLACKS; N];

            for (pattern, wrap) in patterns {
                for row in 0..N {
                    let Some(cells) = Self::col_cells(row, col, pattern.len(), wrap) else {
                        continue;
                    };
                    if self.is_pattern_supported(&pattern, &cells) {
                        Self::mark_col_pattern_supported(&mut mask, &pattern, &cells);
                    }
                }
            }

            for (r, &m) in mask.iter().enumerate() {
                changed |= self.clear_mask(r, col, !m, Rule::ArcConsistency);
            }
        }

        changed
    }

    /// Rule: if a cell's domain has shrunk to a single bit, assign it.
    ///
    /// A singleton domain means there is only one possible value — call
    /// `set_cell` to fix it and propagate.
    fn apply_singleton_rule(&mut self, prev: ChangeSet) -> ChangeSet {
        let mut changed = ChangeSet::default();

        for r in prev.iter_rows() {
            for c in prev.iter_cols() {
                let domain = self.domains[r][c];
                let row_domain = domain & (Self::ALL_DIGITS | Self::ROW_BLACKS);
                let col_domain = domain & (Self::ALL_DIGITS | Self::COL_BLACKS);
                if row_domain.count_ones() == 1 {
                    changed |= self.set_cell(r, c, row_domain, Rule::Singleton);
                } else if col_domain.count_ones() == 1 {
                    changed |= self.set_cell(r, c, col_domain, Rule::Singleton);
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
    fn apply_hidden_single_rule(&mut self, prev: ChangeSet) -> ChangeSet {
        let mut changed = ChangeSet::default();

        for r in prev.iter_rows() {
            // Digit bits (1..=N-2) and row-black bits (N-1, N).
            let mut mask = Self::ALL_DIGITS | Self::ROW_BLACKS;
            while mask != 0 {
                let bit = mask & mask.wrapping_neg();
                mask &= mask - 1;
                if let Some(only_col) = self.singleton_in_row(r, bit) {
                    changed |= self.set_cell(r, only_col, bit, Rule::HiddenSingle);
                }
            }
        }

        for c in prev.iter_cols() {
            // Digit bits (1..=N-2) and col-black bits (N+1, N+2).
            let mut mask = Self::ALL_DIGITS | Self::COL_BLACKS;
            while mask != 0 {
                let bit = mask & mask.wrapping_neg();
                mask &= mask - 1;
                if let Some(only_row) = self.singleton_in_col(c, bit) {
                    changed |= self.set_cell(only_row, c, bit, Rule::HiddenSingle);
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
    fn apply_black_consistency_rule(&mut self, prev: ChangeSet) -> ChangeSet {
        let mut changed = ChangeSet::default();

        for r in prev.iter_rows() {
            for c in prev.iter_cols() {
                let domain = self.domains[r][c];
                if domain & Self::ROW_BLACKS == 0 {
                    changed |= self.clear_mask(r, c, Self::COL_BLACKS, Rule::BlackConsistency);
                }
                if domain & Self::COL_BLACKS == 0 {
                    changed |= self.clear_mask(r, c, Self::ROW_BLACKS, Rule::BlackConsistency);
                }
            }
        }

        changed
    }

    // ── Low-level helper ─────────────────────────────────────────────────────

    /// Return the unique position in row `r` where `bit` appears in the domain,
    /// or `None` if no such position exists or more than one does.
    fn singleton_in_row(&self, r: usize, bit: CellDomain) -> Option<usize> {
        let mut found = None;
        for c in 0..N {
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
    fn singleton_in_col(&self, c: usize, bit: CellDomain) -> Option<usize> {
        let mut found = None;
        for r in 0..N {
            if self.domains[r][c] & bit != 0 {
                if found.is_some() {
                    return None;
                }
                found = Some(r);
            }
        }
        found
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
        let mut changed = ChangeSet::all(N);
        while changed.any() {
            self.recorder.on_step_start();
            changed = self.apply_general_arc_consistency(changed)
                | self.apply_black_consistency_rule(changed)
                | self.apply_singleton_rule(changed)
                | self.apply_hidden_single_rule(changed);
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
    /// To avoid double-counting solutions, we use only row blacks when branching.
    ///
    /// Returns `0` when the cell is fully determined (no branching needed).
    fn branching_bits(domain: CellDomain) -> CellDomain {
        let primary = domain & (Self::ALL_DIGITS | Self::ROW_BLACKS);
        if primary.count_ones() > 1 {
            return primary;
        }
        0
    }

    /// Find the most-constrained unsettled cell (the one with the fewest
    /// remaining choices), using `branching_bits` as the measure.
    ///
    /// Returns `None` when every cell is already fully determined.
    fn pick_branching_cell(&self) -> Option<(usize, usize)> {
        let mut best: Option<(usize, usize, u32)> = None;
        for r in 0..N {
            for c in 0..N {
                let bits = Self::branching_bits(self.domains[r][c]);
                let freedom = bits.count_ones();
                if freedom > 1 && best.is_none_or(|b| freedom < b.2) {
                    best = Some((r, c, freedom));
                }
            }
        }
        best.map(|(r, c, _)| (r, c))
    }

    /// Return the solved grid (`Black` or `Number(1..=N-2)` per cell).
    ///
    /// Returns `None` when the state is not fully solved.
    pub fn solved_cells(&self) -> Option<Grid<N>> {
        if !self.is_solved() {
            return None;
        }

        let mut cells = [[Cell::Empty; N]; N];
        for (r, row) in self.domains.iter().enumerate() {
            for (c, &domain) in row.iter().enumerate() {
                let digits = domain & Self::ALL_DIGITS;
                cells[r][c] = if digits == 0 {
                    Cell::Black
                } else {
                    Cell::Number(digits.trailing_zeros() as u8)
                };
            }
        }

        Some(Grid { cells })
    }

    /// Find a bit to branch on, in the given cell.
    ///
    /// There's not much data to base this decision on, so we heuristically
    /// choose black bits first, then digit bits.
    fn pick_branching_bit(&self, row: usize, col: usize) -> CellDomain {
        let domain = self.domains[row][col];
        if domain & Self::BLACK1_ROW != 0 {
            return Self::BLACK1_ROW;
        }
        if domain & Self::BLACK2_ROW != 0 {
            return Self::BLACK2_ROW;
        }
        debug_assert!(domain & Self::ALL_DIGITS != 0);
        1 << domain.trailing_zeros()
    }
}

// ── Solver trait impl ─────────────────────────────────────────────────────────
//
// The inherent methods above are the hand-written API; this impl block is a
// thin adapter that exposes them through the shared `Solver` trait so the
// backtracking code (and other generic consumers) can drive any solver without
// knowing its concrete type.  Most trait methods forward to the identically-
// named inherent method via `Self::method(...)` — that path always resolves to
// the inherent method, so there's no accidental recursion.

impl<const N: usize, R: Recorder> Solver<N> for BasicSolverState<N, R> {
    type Recorder = R;

    fn new(puzzle: Puzzle<N>) -> Self {
        Self::with_recorder(puzzle)
    }

    fn recorder(&self) -> &R {
        &self.recorder
    }

    fn propagate(&mut self) {
        Self::propagate(self)
    }

    fn is_solved(&self) -> bool {
        Self::is_solved(self)
    }

    fn is_contradiction(&self) -> bool {
        Self::is_contradiction(self)
    }

    fn pick_branching_cell(&self) -> Option<(usize, usize)> {
        Self::pick_branching_cell(self)
    }

    fn pick_branching_bit(&mut self, row: usize, col: usize) -> CellDomain {
        Self::pick_branching_bit(self, row, col)
    }

    fn take_branch(&mut self, r: usize, c: usize, bit: CellDomain) {
        self.recorder.on_step_start();
        let _ = self.set_cell(r, c, bit, Rule::Backtracking);
    }

    fn reject_branch(&mut self, r: usize, c: usize, bit: CellDomain) {
        self.recorder.on_step_start();
        let _ = self.clear_mask(r, c, bit, Rule::Backtracking);
    }

    fn solved_cells(&self) -> Option<Grid<N>> {
        Self::solved_cells(self)
    }
}

// ── Display ───────────────────────────────────────────────────────────────────

impl<const N: usize, R: Recorder> fmt::Display for BasicSolverState<N, R> {
    /// Print the board and, for unsolved cells, the remaining domain bits.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "    ")?;
        for (c, &t) in self.puzzle.col_targets.iter().enumerate() {
            if c > 0 {
                write!(f, "  ")?;
            }
            write!(f, "{t:2}")?;
        }
        writeln!(f)?;
        let sep = format!("   +{}", "---+".repeat(N));
        writeln!(f, "{}", sep)?;
        for r in 0..N {
            write!(f, "{:2} |", self.puzzle.row_targets[r])?;
            for c in 0..N {
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
                        _ => " ? ",
                    };
                    write!(f, "{}|", sym)?;
                }
            }
            writeln!(f)?;
            writeln!(f, "{}", sep)?;
        }
        Ok(())
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::solver::SolveOutcome;

    #[test]
    fn cell_domain_all_values_possible() {
        let state = BasicSolverState::new(Puzzle::new([0; 6], [0; 6]));
        // 8 bits set: numbers 1-4 and the four variants of black
        assert_eq!(state.domains[0][0], 0b111111110);
        assert_eq!(state.domains[5][5], 0b111111110);
    }

    // ── Solver rule tests ─────────────────────────────────────────────────────

    #[test]
    fn row_cells_handles_wrapping_true() {
        assert_eq!(BasicSolverState::<6>::row_cells(0, 0, 3, true), None);
        assert_eq!(BasicSolverState::<6>::row_cells(0, 1, 3, true), None);
        assert_eq!(BasicSolverState::<6>::row_cells(0, 2, 3, true), None);
        assert_eq!(BasicSolverState::<6>::row_cells(0, 3, 3, true), None);
        assert_eq!(
            BasicSolverState::<6>::row_cells(0, 4, 3, true),
            Some(vec![(0, 4), (0, 5), (0, 0)])
        );
        assert_eq!(
            BasicSolverState::<6>::row_cells(0, 5, 3, true),
            Some(vec![(0, 5), (0, 0), (0, 1)])
        );
    }

    #[test]
    fn row_cells_handles_wrapping_false() {
        assert_eq!(
            BasicSolverState::<6>::row_cells(0, 0, 3, false),
            Some(vec![(0, 0), (0, 1), (0, 2)])
        );
        assert_eq!(
            BasicSolverState::<6>::row_cells(0, 1, 3, false),
            Some(vec![(0, 1), (0, 2), (0, 3)])
        );
        assert_eq!(
            BasicSolverState::<6>::row_cells(0, 2, 3, false),
            Some(vec![(0, 2), (0, 3), (0, 4)])
        );
        assert_eq!(
            BasicSolverState::<6>::row_cells(0, 3, 3, false),
            Some(vec![(0, 3), (0, 4), (0, 5)])
        );
        assert_eq!(BasicSolverState::<6>::row_cells(0, 4, 3, false), None);
        assert_eq!(BasicSolverState::<6>::row_cells(0, 5, 3, false), None);
    }

    #[test]
    fn black1_row_always_forbidden_at_last_position() {
        // Black-1 can never sit at position 5, even for target = 0.
        let mut state = BasicSolverState::new(Puzzle::new([0; 6], [0; 6]));
        state.apply_general_arc_consistency(ChangeSet::all(6));
        for r in 0..6 {
            assert_eq!(
                state.domains[r][5] & BasicSolverState::<6>::BLACK1_ROW,
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
        let mut state = BasicSolverState::new(Puzzle::new([9, 0, 0, 0, 0, 0], [0; 6]));
        state.apply_general_arc_consistency(ChangeSet::all(6));

        assert_ne!(
            state.domains[0][0] & BasicSolverState::<6>::BLACK1_ROW,
            0,
            "p=0 should still be allowed"
        );
        assert_ne!(
            state.domains[0][1] & BasicSolverState::<6>::BLACK1_ROW,
            0,
            "p=1 should still be allowed"
        );
        for p in 2..6 {
            assert_eq!(
                state.domains[0][p] & BasicSolverState::<6>::BLACK1_ROW,
                0,
                "p={p} should be forbidden for black-1 with target 9"
            );
        }
    }

    #[test]
    fn inside_outside_rule_target_9() {
        // Row 0 has target 9: digit 1 is outside the blacks.
        let mut state = BasicSolverState::new(Puzzle::new([9, 0, 0, 0, 0, 0], [0; 6]));
        state.apply_general_arc_consistency(ChangeSet::all(6));

        // Middle cells lose digit 1.
        for c in 1..5 {
            assert_eq!(
                state.domains[0][c] & (1 << 1),
                0,
                "digit 1 should be cleared from middle cell (row=0, col={c})"
            );
        }
        // Position 0: only digit 1 remains (row bits).
        assert_ne!(
            state.domains[0][0] & (1 << 1),
            0,
            "col=0 should keep digit 1"
        );
        assert_eq!(
            state.domains[0][0] & ((1 << 2) | (1 << 3) | (1 << 4)),
            0,
            "col=0 should have digits 2-4 cleared"
        );
        // Position 5: only digit 1 remains (row bits).
        assert_ne!(
            state.domains[0][5] & (1 << 1),
            0,
            "col=5 should keep digit 1"
        );
        assert_eq!(
            state.domains[0][5] & ((1 << 2) | (1 << 3) | (1 << 4)),
            0,
            "col=5 should have digits 2-4 cleared"
        );
    }

    #[test]
    fn inside_outside_rule_target_8_column() {
        // Column 2 has target 8: digit 2 is outside the blacks.
        let mut state = BasicSolverState::new(Puzzle::new([0; 6], [0, 0, 8, 0, 0, 0]));
        state.apply_general_arc_consistency(ChangeSet::all(6));

        // Middle cells lose digit 2.
        for r in 1..5 {
            assert_eq!(
                state.domains[r][2] & (1 << 2),
                0,
                "digit 2 should be cleared from middle cell (row={r}, col=2)"
            );
        }
        // Row 0 and 5: only digit 2 remains.
        assert_eq!(
            state.domains[0][2] & BasicSolverState::<6>::ALL_DIGITS,
            (1 << 2),
            "row=0 should have only digit 2"
        );
        assert_eq!(
            state.domains[5][2] & BasicSolverState::<6>::ALL_DIGITS,
            (1 << 2),
            "row=0 should have only digit 2"
        );
    }

    #[test]
    fn set_cell_digit_propagates_to_row_and_col() {
        // Manually place digit 3 at (0, 0) and check it is removed from the
        // rest of row 0 and column 0, while other cells are untouched.
        let mut state = BasicSolverState::new(Puzzle::new([0; 6], [0; 6]));
        state.set_cell(0, 0, 1 << 3, Rule::Singleton);

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
        let mut state = BasicSolverState::new(Puzzle::new([0; 6], [0; 6]));
        state.set_cell(2, 1, BasicSolverState::<6>::BLACK1_ROW, Rule::Singleton);

        // The assigned cell keeps BLACK1_ROW and both col-black bits, but
        // loses digits and BLACK2_ROW.
        let d = state.domains[2][1];
        assert_ne!(d & BasicSolverState::<6>::BLACK1_ROW, 0, "keep BLACK1_ROW");
        assert_ne!(d & BasicSolverState::<6>::BLACK1_COL, 0, "keep BLACK1_COL");
        assert_ne!(d & BasicSolverState::<6>::BLACK2_COL, 0, "keep BLACK2_COL");
        assert_eq!(d & BasicSolverState::<6>::BLACK2_ROW, 0, "drop BLACK2_ROW");
        assert_eq!(d & BasicSolverState::<6>::ALL_DIGITS, 0, "drop all digits");

        // BLACK1_ROW is gone from every other cell in row 2.
        for c in (0..6).filter(|&c| c != 1) {
            assert_eq!(
                state.domains[2][c] & BasicSolverState::<6>::BLACK1_ROW,
                0,
                "col {c} should lose BLACK1_ROW"
            );
        }

        // BLACK2_ROW is gone from every everything to the left, kept on the right
        for c in 0..1 {
            assert_eq!(
                state.domains[2][c] & BasicSolverState::<6>::BLACK2_ROW,
                0,
                "col {c} should lose BLACK2_ROW"
            );
        }
        for c in 2..6 {
            assert_ne!(
                state.domains[2][c] & BasicSolverState::<6>::BLACK2_ROW,
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
        let mut state = BasicSolverState::new(Puzzle::new([0; 6], [0; 6]));
        state.set_cell(0, 3, BasicSolverState::<6>::BLACK1_ROW, Rule::Singleton);

        for c in 0..3 {
            assert_eq!(
                state.domains[0][c] & BasicSolverState::<6>::BLACK2_ROW,
                0,
                "col {c} should lose BLACK2_ROW (left of black-1)"
            );
        }
        for c in 4..6 {
            assert_ne!(
                state.domains[0][c] & BasicSolverState::<6>::BLACK2_ROW,
                0,
                "col {c} should keep BLACK2_ROW (right of black-1)"
            );
        }
    }

    #[test]
    fn black_arc_consistency_uses_actual_domain_state() {
        // Row 0 with target 0 (blacks must be adjacent).
        // Manually clear BLACK2_ROW from every cell except col 3.
        // Then BLACK1_ROW is only valid at col 2 (the sole cell adjacent to
        // the remaining BLACK2_ROW candidate).
        let mut state = BasicSolverState::new(Puzzle::new([0; 6], [0; 6]));
        for c in (0..6).filter(|&c| c != 3) {
            state.domains[0][c] &= !BasicSolverState::<6>::BLACK2_ROW;
        }
        state.apply_general_arc_consistency(ChangeSet::all(6));

        assert_ne!(
            state.domains[0][2] & BasicSolverState::<6>::BLACK1_ROW,
            0,
            "col 2 should keep BLACK1_ROW (adjacent to the only BLACK2_ROW at col 3)"
        );
        for p in (0..6).filter(|&p| p != 2) {
            assert_eq!(
                state.domains[0][p] & BasicSolverState::<6>::BLACK1_ROW,
                0,
                "col {p} should lose BLACK1_ROW"
            );
        }
    }

    #[test]
    fn apply_singleton_rule_assigns_sole_digit() {
        let mut state = BasicSolverState::new(Puzzle::new([0; 6], [0; 6]));
        // Force cell (3, 3) to have only digit 2 in its domain.
        state.domains[3][3] = 1 << 2;
        // Run just this one rule (not propagate, to isolate it).
        state.apply_singleton_rule(ChangeSet::all(6));

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
        let mut state = BasicSolverState::new(Puzzle::new([0; 6], [0; 6]));
        // Remove digit 4 from every cell in row 0 except column 2.
        for c in (0..6).filter(|&c| c != 2) {
            state.domains[0][c] &= !(1 << 4);
        }
        state.apply_hidden_single_rule(ChangeSet::all(6));

        assert_eq!(state.domains[0][2], 1 << 4);
    }

    #[test]
    fn apply_black_consistency_rule_clears_col_blacks_when_no_row_blacks() {
        let mut state = BasicSolverState::new(Puzzle::new([0; 6], [0; 6]));
        // Strip all row-black bits from cell (1, 4).
        state.domains[1][4] &= !BasicSolverState::<6>::ROW_BLACKS;
        state.apply_black_consistency_rule(ChangeSet::all(6));

        // Col-black bits must now also be gone.
        assert_eq!(state.domains[1][4] & BasicSolverState::<6>::COL_BLACKS, 0);
        // But digit bits are intact.
        assert_eq!(
            state.domains[1][4] & BasicSolverState::<6>::ALL_DIGITS,
            BasicSolverState::<6>::ALL_DIGITS
        );
    }

    #[test]
    fn cage_rule_sole_inside_cell_narrows_to_target_digit() {
        // Row 0 has target 3. Pin BLACK1_ROW to col 1 and BLACK2_ROW to col 3.
        // The only inside cell is col 2; the only feasible tuple is {3}, so
        // the cage rule must narrow that cell's digit domain to just digit 3.
        let mut state = BasicSolverState::new(Puzzle::new([3, 0, 0, 0, 0, 0], [0; 6]));
        state.set_cell(0, 1, BasicSolverState::<6>::BLACK1_ROW, Rule::Singleton);
        state.set_cell(0, 3, BasicSolverState::<6>::BLACK2_ROW, Rule::Singleton);
        state.apply_general_arc_consistency(ChangeSet::all(6));

        assert_eq!(
            state.domains[0][2] & BasicSolverState::<6>::ALL_DIGITS,
            1 << 3,
            "inside cell's digit domain should be reduced to just digit 3"
        );
    }

    #[test]
    fn cage_rule_partial_assignment_narrows_remaining_digit() {
        // Row 0, target 6. BLACK1_ROW at col 0, BLACK2_ROW at col 4.
        // Inside: cols 1, 2, 3.  Col 1 = digit 2, col 3 = digit 1.
        // Only feasible inside tuple: {1, 2, 3}, so col 2 must be digit 3.
        let mut state = BasicSolverState::new(Puzzle::new([6, 0, 0, 0, 0, 0], [0; 6]));
        state.set_cell(0, 0, BasicSolverState::<6>::BLACK1_ROW, Rule::Singleton);
        state.set_cell(0, 4, BasicSolverState::<6>::BLACK2_ROW, Rule::Singleton);
        state.set_cell(0, 1, 1 << 2, Rule::Singleton); // digit 2
        state.set_cell(0, 3, 1 << 1, Rule::Singleton); // digit 1
        state.apply_general_arc_consistency(ChangeSet::all(6));

        assert_eq!(
            state.domains[0][2] & BasicSolverState::<6>::ALL_DIGITS,
            1 << 3,
            "empty inside cell's digit domain should be reduced to just digit 3"
        );
    }

    #[test]
    fn black_arc_consistency_black1_forward_complete() {
        // Row 0, target 7. Inside could be 3,4 or 1,2,4... so a priori it
        // isn't clear whether col 2 could be BLACK1. It could be for the
        // shorter target.
        let mut state = BasicSolverState::new(Puzzle::new([7, 0, 0, 0, 0, 0], [0; 6]));
        state.apply_general_arc_consistency(ChangeSet::all(6));
        assert_ne!(
            state.domains[0][2] & BasicSolverState::<6>::BLACK1_ROW,
            0,
            "BLACK1 still possible at col 2"
        );

        // If we set col 0 to be 3, then the 3,4 tuple can no longer be inside.
        // The rule should clear BLACK1 from col 2.
        state.set_cell(0, 0, 1 << 3, Rule::Singleton);
        state.apply_general_arc_consistency(ChangeSet::all(6));
        assert_eq!(
            state.domains[0][2] & BasicSolverState::<6>::BLACK1_ROW,
            0,
            "BLACK1 no longer possible at col 2"
        );

        // In fact, this should completely determine the blacks.
        assert_eq!(
            (0..6)
                .map(|c| state.domains[0][c] & BasicSolverState::<6>::BLACK1_ROW != 0)
                .collect::<Vec<_>>(),
            [false, true, false, false, false, false],
            "BLACK1 is completely determined"
        );
        assert_eq!(
            (0..6)
                .map(|c| state.domains[0][c] & BasicSolverState::<6>::BLACK2_ROW != 0)
                .collect::<Vec<_>>(),
            [false, false, false, false, false, true],
            "BLACK2 is completely determined"
        );
    }

    #[test]
    fn black_arc_consistency_black1_backward() {
        // Row 0, target 7. Inside could be 3,4 or 1,2,4... so a priori it
        // isn't clear whether col 2 could be BLACK1. It could be for the
        // shorter target.
        let mut state = BasicSolverState::new(Puzzle::new([7, 0, 0, 0, 0, 0], [0; 6]));
        state.apply_general_arc_consistency(ChangeSet::all(6));
        assert_ne!(
            state.domains[0][2] & BasicSolverState::<6>::BLACK1_ROW,
            0,
            "BLACK1 still possible at col 2"
        );

        // If we set col 5 to be 3, then the 3,4 tuple can no longer be inside.
        // This completely determines the blacks
        state.set_cell(0, 5, 1 << 3, Rule::Singleton);
        state.apply_general_arc_consistency(ChangeSet::all(6));
        assert_eq!(
            (0..6)
                .map(|c| state.domains[0][c] & BasicSolverState::<6>::BLACK1_ROW != 0)
                .collect::<Vec<_>>(),
            [true, false, false, false, false, false],
            "BLACK1 is completely determined"
        );
        assert_eq!(
            (0..6)
                .map(|c| state.domains[0][c] & BasicSolverState::<6>::BLACK2_ROW != 0)
                .collect::<Vec<_>>(),
            [false, false, false, false, true, false],
            "BLACK2 is completely determined"
        );
    }

    #[test]
    fn black_arc_consistency_black1_forward_not_enough_info() {
        // Row 0, target 7. Inside could be 3,4 or 1,2,4... so a priori it
        // isn't clear where the blacks are. The target could be 2 or 3 cells wide.
        let mut state = BasicSolverState::new(Puzzle::new([7, 0, 0, 0, 0, 0], [0; 6]));
        state.apply_general_arc_consistency(ChangeSet::all(6));

        assert_eq!(
            (0..6)
                .map(|c| state.domains[0][c] & BasicSolverState::<6>::BLACK1_ROW != 0)
                .collect::<Vec<_>>(),
            [true, true, true, false, false, false],
            "BLACK1 still possible in first three rows"
        );
        assert_eq!(
            (0..6)
                .map(|c| state.domains[0][c] & BasicSolverState::<6>::BLACK2_ROW != 0)
                .collect::<Vec<_>>(),
            [false, false, false, true, true, true],
            "BLACK2 still possible in last three rows"
        );

        // If we set col 0 to be 1, then the inside must be the 3,4 tuple.
        // However, there are still two possibilities for the blacks.
        state.set_cell(0, 0, 1 << 1, Rule::Singleton);
        state.apply_general_arc_consistency(ChangeSet::all(6));

        assert_eq!(
            (0..6)
                .map(|c| state.domains[0][c] & BasicSolverState::<6>::BLACK1_ROW != 0)
                .collect::<Vec<_>>(),
            [false, true, true, false, false, false],
            "BLACK1 has two remaining possibilities"
        );
        assert_eq!(
            (0..6)
                .map(|c| state.domains[0][c] & BasicSolverState::<6>::BLACK2_ROW != 0)
                .collect::<Vec<_>>(),
            [false, false, false, false, true, true],
            "BLACK2 has two remaining possibilities"
        );

        // Now, if we exclude 2 from the last cell, then the 1#..#2 configuration
        // is ruled out and the blacks are determined. Arc consistency alone can't
        // quite find this though; it needs a pass of the singleton rule to fix
        // black2, and then another arc consistency pass.
        state.domains[0][5] &= !(1 << 2);
        state.apply_general_arc_consistency(ChangeSet::all(6));
        state.apply_singleton_rule(ChangeSet::all(6));
        state.apply_general_arc_consistency(ChangeSet::all(6));

        assert_eq!(
            (0..6)
                .map(|c| state.domains[0][c] & BasicSolverState::<6>::BLACK1_ROW != 0)
                .collect::<Vec<_>>(),
            [false, false, true, false, false, false],
            "BLACK1 is determined"
        );
        assert_eq!(
            (0..6)
                .map(|c| state.domains[0][c] & BasicSolverState::<6>::BLACK2_ROW != 0)
                .collect::<Vec<_>>(),
            [false, false, false, false, false, true],
            "BLACK2 is determined"
        );
    }

    // ── Backtracking tests ────────────────────────────────────────────────────

    #[test]
    fn count_solutions_returns_1_for_unique_puzzle() {
        // Both newspaper puzzles should have exactly one solution.
        let state = BasicSolverState::new(Puzzle::new([8, 2, 3, 8, 9, 0], [0, 0, 5, 9, 0, 4]));
        assert_eq!(state.count_solutions(2), 1);

        let state = BasicSolverState::new(Puzzle::new([3, 3, 5, 0, 7, 0], [5, 0, 2, 6, 5, 10]));
        assert_eq!(state.count_solutions(2), 1);
    }

    #[test]
    fn count_solutions_returns_0_for_impossible_puzzle() {
        // Targets that cannot be satisfied: all targets = 1 requires a 1-cell
        // gap in every row and column, which is impossible to satisfy globally.
        let state = BasicSolverState::new(Puzzle::new([1; 6], [1; 6]));
        assert_eq!(state.count_solutions(1), 0);
    }

    // ── solve() tests ─────────────────────────────────────────────────────────

    #[test]
    fn solve_returns_unique_for_newspaper_puzzle() {
        let state = BasicSolverState::new(Puzzle::new([8, 2, 3, 8, 9, 0], [0, 0, 5, 9, 0, 4]));
        match state.solve() {
            SolveOutcome::Unique(s) => assert!(s.is_solved()),
            other => panic!("expected Unique, got {other:?}"),
        }
    }

    #[test]
    fn solve_returns_multiple_for_underconstrained_puzzle() {
        // This puzzle is known to have many solutions (see
        // `solver_finds_known_solutions` above, which counts 32).
        let state = BasicSolverState::new(Puzzle::new([10, 0, 0, 0, 3, 0], [10, 0, 0, 0, 3, 0]));
        match state.solve() {
            SolveOutcome::Multiple(s) => assert!(s.is_solved()),
            other => panic!("expected Multiple, got {other:?}"),
        }
    }

    #[test]
    fn solve_returns_unsolvable_for_impossible_puzzle() {
        let state = BasicSolverState::new(Puzzle::new([1; 6], [1; 6]));
        assert!(matches!(state.solve(), SolveOutcome::Unsolvable));
    }

    // ── Stats tests ───────────────────────────────────────────────────────────

    #[test]
    fn stats_track_bits_removed_and_search_nodes() {
        use crate::recorder::FullStats;
        // Solve a known-unique puzzle and check the counters are populated.
        let state = BasicSolverState::<6, FullStats>::with_recorder(Puzzle::new(
            [8, 2, 3, 8, 9, 0],
            [0, 0, 5, 9, 0, 4],
        ));
        let _ = state.solve();
        let s = state.recorder().snapshot();
        // One top-level search call at minimum; a uniquely-solvable puzzle
        // with good propagation typically takes 1 node.
        assert!(s.search_nodes >= 1, "search_nodes = {}", s.search_nodes);
        // Arc-consistency does the heavy lifting for this puzzle.  (The basic
        // solver has no separate `seed_queue`, so its initial reduction is
        // attributed to `ArcConsistency`, not `TargetTuples`.)
        assert!(
            s.bits_arc_consistency > 0,
            "bits_arc_consistency = {}",
            s.bits_arc_consistency
        );
    }

    #[test]
    fn stats_count_backtracks_on_underconstrained_puzzle() {
        use crate::recorder::FullStats;
        // Multiple solutions → the search has to branch, so search_nodes > 1.
        let state = BasicSolverState::<6, FullStats>::with_recorder(Puzzle::new(
            [10, 0, 0, 0, 3, 0],
            [10, 0, 0, 0, 3, 0],
        ));
        let _ = state.solve();
        let s = state.recorder().snapshot();
        assert!(
            s.search_nodes > 1,
            "expected branching, got search_nodes = {}",
            s.search_nodes
        );
    }

    // ── Newspaper puzzles ─────────────────────────────────────────────────────
    //
    // Integration tests: propagate a full puzzle and assert the exact Display
    // output.  Update the expected strings whenever the solver rules change.

    #[test]
    fn newspaper_puzzle_1() {
        let mut state = BasicSolverState::new(Puzzle::new([8, 2, 3, 8, 9, 0], [0, 0, 5, 9, 0, 4]));
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
        let mut state = BasicSolverState::new(Puzzle::new([3, 3, 5, 0, 7, 0], [5, 0, 2, 6, 5, 10]));
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

    // ── Tests for other grid sizes ────────────────────────────────────────────

    #[test]
    fn n2_all_black_grid_has_unique_solution() {
        // N=2: a 2×2 grid with 2 blacks per row/column and no digit cells.
        // Targets must be 0 (no cells between adjacent blacks).
        // The only valid arrangement is [[B1,B2],[B1,B2]] with consistent
        // column ordering, and the solver should find exactly one solution.
        let state = BasicSolverState::<2>::new(Puzzle::new([0; 2], [0; 2]));
        assert_eq!(state.count_solutions(2), 1);
    }

    #[test]
    fn n4_has_solutions_for_satisfiable_targets() {
        // N=4 puzzle that can be satisfied.  All targets 0 means adjacent
        // blacks in every row and column; many valid grids exist.
        let state = BasicSolverState::<4>::new(Puzzle::new([0; 4], [0; 4]));
        assert!(state.count_solutions(1) >= 1);
    }

    #[test]
    fn n4_no_solutions_for_contradictory_targets() {
        // All row targets 3 forces blacks to col 0 and col 3 in every row,
        // which means every cell in cols 0 and 3 is black — but a column
        // can hold at most 2 blacks, giving a contradiction.
        let state = BasicSolverState::<4>::new(Puzzle::new([3; 4], [3; 4]));
        assert_eq!(state.count_solutions(1), 0);
    }

    #[test]
    fn n4_domain_initialises_correctly() {
        // For N=4: digits are 1 and 2 (bits 1-2), row blacks are bits 3-4,
        // col blacks are bits 5-6. Full cell = bits 1-6 = 0b1111110.
        let state = BasicSolverState::<4>::new(Puzzle::new([0; 4], [0; 4]));
        assert_eq!(state.domains[0][0], 0b1111110);
        assert_eq!(BasicSolverState::<4>::ALL_DIGITS, 0b110);
        assert_eq!(BasicSolverState::<4>::BLACK1_ROW, 1 << 3);
        assert_eq!(BasicSolverState::<4>::BLACK2_ROW, 1 << 4);
        assert_eq!(BasicSolverState::<4>::BLACK1_COL, 1 << 5);
        assert_eq!(BasicSolverState::<4>::BLACK2_COL, 1 << 6);
    }

    #[test]
    fn solver_finds_known_solutions() {
        let state = BasicSolverState::new(Puzzle::new([10, 0, 0, 0, 3, 0], [10, 0, 0, 0, 3, 0]));
        assert_eq!(state.count_solutions(100), 32);
    }
}
