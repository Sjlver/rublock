use tracing::{instrument, trace};

use crate::solver::{CellDomain, Puzzle, Solver, Tables};
use crate::stats::{Rule, Stats, StatsHandle};

// ── LiveTuple ─────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct LiveTuple<const N: usize> {
    start: u8,
    len: u8,
    pattern: [CellDomain; N],
}

impl<const N: usize> LiveTuple<N> {
    fn new(start: usize, len: usize, digit_mask: CellDomain) -> Self {
        let mut pattern = [0 as CellDomain; N];
        pattern[0] = BlackSolverState::<N>::BLACK;
        pattern[1..=len].fill(digit_mask);
        pattern[len + 1] = BlackSolverState::<N>::BLACK;
        Self {
            start: start as u8,
            len: (len + 2) as u8,
            pattern,
        }
    }

    fn pos_of(&self, c: usize) -> Option<usize> {
        let pos = (c + N - self.start as usize) % N;
        if pos < self.len as usize {
            Some(pos)
        } else {
            None
        }
    }

    /// Yields (position_in_grid, pattern_value) for each slot.
    fn cells(&self) -> impl Iterator<Item = (usize, CellDomain)> + '_ {
        (0..self.len as usize).map(move |p| ((self.start as usize + p) % N, self.pattern[p]))
    }
}

// ── BlackSolverState ──────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct BlackSolverState<const N: usize> {
    pub puzzle: Puzzle<N>,
    domains: [[CellDomain; N]; N],
    queue: Vec<(usize, usize, CellDomain)>,

    // ── Singleton constraint ──────────────────────────────────────────────────
    // How many value-choices does this cell have?
    domain_size: [[u8; N]; N],

    // ── Hidden singles constraint ─────────────────────────────────────────────
    // Number of cells in row r (col c) whose domain has a given bit set.
    row_candidates: [[u8; N]; N],
    col_candidates: [[u8; N]; N],

    // ── General arc consistency constraint ────────────────────────────────────
    live_tuples_row: [Vec<LiveTuple<N>>; N],
    live_tuples_col: [Vec<LiveTuple<N>>; N],

    // tuple_support_row[r][c][p] = number of live row-direction tuples in row r
    //   whose pattern at column c includes the given domain bit.
    tuple_support_row: [[[u16; N]; N]; N],
    tuple_support_col: [[[u16; N]; N]; N],

    /// Debug-only propagation statistics.  See `stats.rs`.
    stats: StatsHandle,
}

struct BitName<const N: usize>(CellDomain);

impl<const N: usize> std::fmt::Display for BitName<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let b = self.0;
        if b == BlackSolverState::<N>::BLACK {
            write!(f, "BLACK")
        } else if b & BlackSolverState::<N>::DIGITS != 0 {
            write!(f, "DIGIT_{}", b.trailing_zeros())
        } else {
            panic!("BitName: {b:#b} is not a valid single bit")
        }
    }
}

impl<const N: usize> std::fmt::Debug for BlackSolverState<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "BlackSolverState {{")?;
        writeln!(
            f,
            "  puzzle = {:?} {:?}",
            self.puzzle.row_targets, self.puzzle.col_targets
        )?;
        writeln!(f, "  domains =")?;
        for r in 0..N {
            write!(f, "   ")?;
            for c in 0..N {
                write!(f, " ")?;
                write!(f, "{:0width$b}", self.domains[r][c], width = N - 1)?;
            }
            writeln!(f)?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

impl<const N: usize> BlackSolverState<N> {
    const BLACK: CellDomain = 1 << 0;
    const DIGITS: CellDomain = ((1 << (N - 2)) - 1) << 1;
    const FULL_DOMAIN: CellDomain = Self::BLACK | Self::DIGITS;

    #[instrument(skip(puzzle))]
    pub fn new(puzzle: Puzzle<N>) -> Self {
        // Initialize counters from the full domain (before any clear_mask).
        let domain_size: [[u8; N]; N] = [[N as u8; N]; N];

        // Digit indices 1..=N-2 are valid; 0 and N-1 are not digit bits.
        let candidates_for_bits: [u8; N] =
            std::array::from_fn(|p| if p <= N - 2 { N as u8 } else { 0 });
        let row_candidates: [[u8; N]; N] = [candidates_for_bits; N];
        let col_candidates: [[u8; N]; N] = [candidates_for_bits; N];

        let live_tuples_row: [Vec<LiveTuple<N>>; N] = std::array::from_fn(|_| Vec::new());
        let live_tuples_col: [Vec<LiveTuple<N>>; N] = std::array::from_fn(|_| Vec::new());

        let tuple_support_row: [[[u16; N]; N]; N] = [[[0u16; N]; N]; N];
        let tuple_support_col: [[[u16; N]; N]; N] = [[[0u16; N]; N]; N];

        let mut state = Self {
            puzzle,
            domains: [[Self::FULL_DOMAIN; N]; N],
            queue: Vec::new(),
            domain_size,
            row_candidates,
            col_candidates,
            live_tuples_row,
            live_tuples_col,
            tuple_support_row,
            tuple_support_col,
            stats: StatsHandle::new(),
        };

        state.init_live_tuples();
        state.seed_queue();
        state.propagate();

        state
    }

    /// Return a snapshot of the stats collected so far.
    pub fn stats(&self) -> Stats {
        self.stats.snapshot()
    }

    /// Enumerate the live tuples
    fn init_live_tuples(&mut self) {
        let tables = Tables::build(N - 2);

        for r in 0..N {
            let inside_target = self.puzzle.row_targets[r] as usize;
            let outside_target = tables.max_sum - inside_target;

            // Inside (non-wrapping)
            for (len, digit_mask) in tables.valid_tuples_for_target(inside_target) {
                for start in 0..N {
                    if start + len + 2 <= N {
                        let t = LiveTuple::new(start, len, digit_mask);
                        trace!(
                            row = r,
                            start = start,
                            bits = format_args!(
                                "{:0width$b}",
                                if t.len > 2 { t.pattern[1] } else { 0 },
                                width = N - 1
                            ),
                            "inside row tuple live"
                        );
                        self.live_tuples_row[r].push(t);
                    }
                }
            }

            // Outside (wrapping):
            for (len, digit_mask) in tables.valid_tuples_for_target(outside_target) {
                for start in 0..N {
                    if start + len + 2 > N {
                        let t = LiveTuple::new(start, len, digit_mask);
                        trace!(
                            row = r,
                            start = start,
                            bits = format_args!(
                                "{:0width$b}",
                                if t.len > 2 { t.pattern[1] } else { 0 },
                                width = N - 1
                            ),
                            "outside row tuple live"
                        );
                        self.live_tuples_row[r].push(t);
                    }
                }
            }
        }

        for c in 0..N {
            let inside_target = self.puzzle.col_targets[c] as usize;
            let outside_target = tables.max_sum - inside_target;

            // Inside (non-wrapping)
            for (len, digit_mask) in tables.valid_tuples_for_target(inside_target) {
                for start in 0..N {
                    if start + len + 2 <= N {
                        let t = LiveTuple::new(start, len, digit_mask);
                        trace!(
                            col = c,
                            start = start,
                            bits = format_args!(
                                "{:0width$b}",
                                if t.len > 2 { t.pattern[1] } else { 0 },
                                width = N - 1
                            ),
                            "inside col tuple live"
                        );
                        self.live_tuples_col[c].push(t);
                    }
                }
            }

            // Outside (wrapping)
            for (len, digit_mask) in tables.valid_tuples_for_target(outside_target) {
                for start in 0..N {
                    if start + len + 2 > N {
                        let t = LiveTuple::new(start, len, digit_mask);
                        trace!(
                            col = c,
                            start = start,
                            bits = format_args!(
                                "{:0width$b}",
                                if t.len > 2 { t.pattern[1] } else { 0 },
                                width = N - 1
                            ),
                            "outside row tuple live"
                        );
                        self.live_tuples_col[c].push(t);
                    }
                }
            }
        }

        // Initialize support counts from live tuples.
        for r in 0..N {
            for t in &self.live_tuples_row[r] {
                for (c2, mut bits) in t.cells() {
                    while bits != 0 {
                        let b = bits.trailing_zeros() as usize;
                        bits &= bits - 1;
                        self.tuple_support_row[r][c2][b] += 1;
                    }
                }
            }
        }

        for c in 0..N {
            for t in &self.live_tuples_col[c] {
                for (r2, mut bits) in t.cells() {
                    while bits != 0 {
                        let b = bits.trailing_zeros() as usize;
                        bits &= bits - 1;
                        self.tuple_support_col[r2][c][b] += 1;
                    }
                }
            }
        }
    }

    /// Seed queue with bits that have no support.
    //
    // The `r`/`c` range loops below are paired with cross-indexing into
    // `live_tuples_row[r]` / `live_tuples_col[c]`, so the clippy-suggested
    // `iter_mut().enumerate()` rewrite actually hurts readability here.
    #[allow(clippy::needless_range_loop)]
    fn seed_queue(&mut self) {
        let mut row_tuple_supported_bits: [[CellDomain; N]; N] = [[0; N]; N];
        for r in 0..N {
            for t in &self.live_tuples_row[r] {
                for (c2, pat) in t.cells() {
                    row_tuple_supported_bits[r][c2] |= pat;
                }
            }
        }
        let mut col_tuple_supported_bits: [[CellDomain; N]; N] = [[0; N]; N];
        for c in 0..N {
            for t in &self.live_tuples_col[c] {
                for (r2, pat) in t.cells() {
                    col_tuple_supported_bits[r2][c] |= pat;
                }
            }
        }
        for r in 0..N {
            for c in 0..N {
                let supported = row_tuple_supported_bits[r][c] & col_tuple_supported_bits[r][c];
                // Seeding is driven by live-tuple support, so attribute these
                // removals to the arc-consistency rule.
                self.clear_mask(r, c, !supported & Self::FULL_DOMAIN, Rule::ArcConsistency);
            }
        }
    }

    // ── Core mutation primitives ──────────────────────────────────────────────

    #[instrument(skip(self), fields(mask = format_args!("{mask:0width$b}", width = N - 1)))]
    fn clear_mask(&mut self, r: usize, c: usize, mask: CellDomain, rule: Rule) {
        let before = self.domains[r][c];
        self.domains[r][c] &= !mask;
        let removed = before & !self.domains[r][c];
        if removed != 0 {
            self.stats.incr_bits(rule, removed.count_ones());
        }
        let mut bits = removed;
        while bits != 0 {
            let b = bits & bits.wrapping_neg();
            bits &= bits - 1;
            trace!(b = format_args!("{}", BitName::<N>(b)), "bit removed");
            self.queue.push((r, c, b));
        }
    }

    #[instrument(skip(self), fields(bit = format_args!("{}", BitName::<N>(bit))))]
    fn set_cell(&mut self, r: usize, c: usize, bit: CellDomain, rule: Rule) {
        debug_assert_eq!(bit.count_ones(), 1, "set_cell requires exactly one bit");
        debug_assert_eq!(
            bit & !Self::FULL_DOMAIN,
            0,
            "set_cell requires a domain bit"
        );
        trace!(bit = format_args!("{}", BitName::<N>(bit)), "setting cell");

        if bit & Self::DIGITS != 0 {
            for col in (0..N).filter(|&col| col != c) {
                self.clear_mask(r, col, bit, rule);
            }
            for row in (0..N).filter(|&row| row != r) {
                self.clear_mask(row, c, bit, rule);
            }
        }
        self.clear_mask(r, c, !bit & Self::FULL_DOMAIN, rule);
    }

    // ── Update handlers ───────────────────────────────────────────────────────

    fn update(&mut self, row: usize, col: usize, bit: CellDomain) {
        trace!(
            row,
            col,
            bit = format_args!("{}", BitName::<N>(bit)),
            "update"
        );
        self.update_singleton(row, col, bit);
        self.update_hidden_singles(row, col, bit);
        self.update_arc(row, col, bit);
    }

    #[instrument(skip(self), fields(bit = format_args!("{}", BitName::<N>(bit))))]
    fn update_singleton(&mut self, row: usize, col: usize, bit: CellDomain) {
        self.domain_size[row][col] -= 1;
        if self.domain_size[row][col] == 1 {
            self.set_cell(row, col, self.domains[row][col], Rule::Singleton);
        }
    }

    #[instrument(skip(self), fields(bit = format_args!("{}", BitName::<N>(bit))))]
    fn update_hidden_singles(&mut self, r: usize, c: usize, bit: CellDomain) {
        let b = bit.trailing_zeros() as usize;

        self.row_candidates[r][b] -= 1;
        if (bit & Self::DIGITS != 0)
            && (self.row_candidates[r][b] == 1)
            && let Some(c2) = (0..N).find(|&c2| self.domains[r][c2] & bit != 0)
        {
            self.set_cell(r, c2, bit, Rule::HiddenSingle);
        }
        if (bit == Self::BLACK)
            && (self.row_candidates[r][b] == 2)
            && let Some(c2) = (0..N).find(|&c2| self.domains[r][c2] & bit != 0)
            && let Some(c3) = (c2 + 1..N).find(|&c3| self.domains[r][c3] & bit != 0)
        {
            self.set_cell(r, c2, bit, Rule::HiddenSingle);
            self.set_cell(r, c3, bit, Rule::HiddenSingle);
        }

        self.col_candidates[c][b] -= 1;
        if (bit & Self::DIGITS != 0)
            && (self.col_candidates[c][b] == 1)
            && let Some(r2) = (0..N).find(|&r2| self.domains[r2][c] & bit != 0)
        {
            self.set_cell(r2, c, bit, Rule::HiddenSingle);
        }
        if (bit == Self::BLACK)
            && (self.col_candidates[c][b] == 2)
            && let Some(r2) = (0..N).find(|&r2| self.domains[r2][c] & bit != 0)
            && let Some(r3) = (r2 + 1..N).find(|&r3| self.domains[r3][c] & bit != 0)
        {
            self.set_cell(r2, c, bit, Rule::HiddenSingle);
            self.set_cell(r3, c, bit, Rule::HiddenSingle);
        }
    }

    #[instrument(skip(self), fields(bit = format_args!("{}", BitName::<N>(bit))))]
    fn update_arc(&mut self, r: usize, c: usize, bit: CellDomain) {
        // Row direction: check tuples in live_row[r] that cover column c.
        let mut i = 0;
        while i < self.live_tuples_row[r].len() {
            let t = &self.live_tuples_row[r][i];
            let Some(pos) = t.pos_of(c) else {
                i += 1;
                continue;
            };
            if t.pattern[pos] & bit == 0 {
                i += 1;
                continue;
            }
            // Check liveness: do the cell domains still support this pattern?
            // We check two things here: the tuple might be dead because the current
            // cell no longer supports it, or because there is no more cell that
            // has `bit`.
            if (self.domains[r][c] & t.pattern[pos] != 0)
                && t.cells().any(|(c2, _)| self.domains[r][c2] & bit != 0)
            {
                i += 1;
                continue;
            }
            // Tuple is dead: remove and update support counts.
            trace!(
                start = t.start,
                bits = format_args!(
                    "{:0width$b}",
                    if t.len > 2 { t.pattern[1] } else { 0 },
                    width = N - 1
                ),
                "row tuple killed"
            );
            let dead = self.live_tuples_row[r].swap_remove(i);
            for (c2, mut bits) in dead.cells() {
                while bits != 0 {
                    let b = bits.trailing_zeros() as usize;
                    bits &= bits - 1;
                    self.tuple_support_row[r][c2][b] -= 1;
                    if self.tuple_support_row[r][c2][b] == 0 {
                        self.clear_mask(r, c2, 1 << b, Rule::ArcConsistency);
                    }
                }
            }
            // Don't increment i: swap_remove moved the last element here.
        }

        // Column direction: check tuples in live_col[c] that cover row r.
        let mut i = 0;
        while i < self.live_tuples_col[c].len() {
            let t = &self.live_tuples_col[c][i];
            let Some(pos) = t.pos_of(r) else {
                i += 1;
                continue;
            };
            if t.pattern[pos] & bit == 0 {
                i += 1;
                continue;
            }
            if (self.domains[r][c] & t.pattern[pos] != 0)
                && t.cells().any(|(r2, _)| self.domains[r2][c] & bit != 0)
            {
                i += 1;
                continue;
            }
            trace!(
                start = t.start,
                bits = format_args!(
                    "{:0width$b}",
                    if t.len > 2 { t.pattern[1] } else { 0 },
                    width = N - 1
                ),
                "col tuple killed"
            );
            let dead = self.live_tuples_col[c].swap_remove(i);
            for (r2, mut bits) in dead.cells() {
                while bits != 0 {
                    let b = bits.trailing_zeros() as usize;
                    bits &= bits - 1;
                    self.tuple_support_col[r2][c][b] -= 1;
                    if self.tuple_support_col[r2][c][b] == 0 {
                        self.clear_mask(r2, c, 1 << b, Rule::ArcConsistency);
                    }
                }
            }
        }
    }

    // ── Propagation ───────────────────────────────────────────────────────────

    pub fn propagate(&mut self) {
        while let Some((r, c, bit)) = self.queue.pop() {
            // Return early if a contradiction is detected
            if self.domains[r][c] == 0 {
                return;
            }

            self.update(r, c, bit);
        }
    }

    // ── Search ────────────────────────────────────────────────────────────────

    pub fn is_contradiction(&self) -> bool {
        self.domains.iter().flatten().any(|&d| d == 0)
    }

    pub fn is_solved(&self) -> bool {
        self.domains.iter().flatten().all(|&d| d.count_ones() == 1)
    }

    /// Return the solved grid as `-1` for black and positive digits otherwise.
    ///
    /// Returns `None` when the state is not fully solved.
    pub fn solved_cells(&self) -> Option<[[i8; N]; N]> {
        if !self.is_solved() {
            return None;
        }

        let mut cells = [[0_i8; N]; N];
        for (r, row) in self.domains.iter().enumerate() {
            for (c, &domain) in row.iter().enumerate() {
                let digits = domain & Self::DIGITS;
                cells[r][c] = if digits == 0 {
                    -1
                } else {
                    digits.trailing_zeros() as i8
                };
            }
        }

        Some(cells)
    }

    fn pick_branching_bit(&mut self, row: usize, col: usize) -> CellDomain {
        // Heuristic is to branch on the bit that has the smallest tuple support.
        // If there is equality, prefer black.
        let mut domain = self.domains[row][col];
        let mut best = 0;
        let mut min_support = u16::MAX;
        while domain != 0 {
            let bit = domain.trailing_zeros() as usize;
            domain &= domain - 1;
            let support = self.tuple_support_row[row][col][bit];
            if support < min_support {
                best = bit;
                min_support = support;
            }
        }
        1 << best
    }

    fn pick_branching_cell(&self) -> Option<(usize, usize)> {
        let mut best: Option<(usize, usize, u32)> = None;
        for r in 0..N {
            for c in 0..N {
                let freedom = self.domains[r][c].count_ones();
                if freedom > 1 && best.is_none_or(|b| freedom < b.2) {
                    if freedom == 2 {
                        return Some((r, c));
                    }
                    best = Some((r, c, freedom));
                }
            }
        }
        best.map(|(r, c, _)| (r, c))
    }
}

// ── Solver trait impl ─────────────────────────────────────────────────────────
//
// Mirror of the adapter in `basic_solver.rs`: each required trait method
// forwards to the identically-named inherent method above.  `take_branch` /
// `reject_branch` delegate to the internal propagation primitives, tagging the
// change as a `Backtracking` decision for the stats counters.

impl<const N: usize> Solver<N> for BlackSolverState<N> {
    fn new(puzzle: Puzzle<N>) -> Self {
        Self::new(puzzle)
    }

    fn stats(&self) -> Stats {
        self.stats.snapshot()
    }

    fn stats_handle(&self) -> &StatsHandle {
        &self.stats
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
        self.set_cell(r, c, bit, Rule::Backtracking);
    }

    fn reject_branch(&mut self, r: usize, c: usize, bit: CellDomain) {
        self.clear_mask(r, c, bit, Rule::Backtracking);
    }

    fn solved_cells(&self) -> Option<[[i8; N]; N]> {
        Self::solved_cells(self)
    }
}

impl<const N: usize> std::fmt::Display for BlackSolverState<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
                if domain == Self::BLACK {
                    write!(f, " # |")?;
                } else if domain.count_ones() == 1 {
                    write!(f, "{:2} |", domain.trailing_zeros())?;
                } else {
                    let sym = match domain.count_ones() {
                        0 => " X ",
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
    fn live_tuple_pos_at_works() {
        // A zero-digit tuple: [BLACK1_ROW, BLACK2_ROW]
        let t0 = LiveTuple::<6>::new(0, 0, 0);
        assert_eq!(t0.pos_of(0), Some(0));
        assert_eq!(t0.pos_of(1), Some(1));
        assert_eq!(t0.pos_of(2), None);
        assert_eq!(t0.pos_of(3), None);
        assert_eq!(t0.pos_of(4), None);
        assert_eq!(t0.pos_of(5), None);
    }

    #[test]
    fn live_tuple_pos_at_works_with_wrapping() {
        // A one-digit wrapping tuple: [BLACK, digit(1), BLACK]
        let t0 = LiveTuple::<6>::new(5, 1, 1 << 1);
        assert_eq!(t0.pos_of(0), Some(1));
        assert_eq!(t0.pos_of(1), Some(2));
        assert_eq!(t0.pos_of(2), None);
        assert_eq!(t0.pos_of(3), None);
        assert_eq!(t0.pos_of(4), None);
        assert_eq!(t0.pos_of(5), Some(0));
    }

    #[test]
    fn black_solver_state_initializes_correctly() {
        let _ = tracing_subscriber::fmt::try_init();
        let state = BlackSolverState::new(Puzzle::new([0; 4], [0; 4]));

        let expected_domains: [[CellDomain; 4]; 4] = [[BlackSolverState::<4>::FULL_DOMAIN; 4]; 4];
        assert_eq!(state.domains, expected_domains);

        assert_eq!(state.domain_size, [[4; 4]; 4]);

        // row_candidates: digits 1,2 and black have 4 candidates each
        assert_eq!(state.row_candidates, [[4, 4, 4, 0]; 4]);
        assert_eq!(state.row_candidates, [[4, 4, 4, 0]; 4]);
    }

    #[test]
    fn propagation_target_9() {
        let _ = tracing_subscriber::fmt::try_init();
        // With target = 9, the two middle cells must be non-black.
        // Similarly, the value 1 must be in the outside cage at the end of the row.
        let state = BlackSolverState::new(Puzzle::new([9, 0, 0, 0, 0, 0], [0; 6]));

        let black = BlackSolverState::<6>::BLACK;
        assert_eq!(
            state.domains[0].map(|d| d & black),
            [black, black, 0, 0, black, black,]
        );
        let bit1 = 1 << 1;
        assert_eq!(state.domains[0].map(|d| d & bit1), [bit1, 0, 0, 0, 0, bit1]);
    }

    #[test]
    fn sample_puzzle() {
        let state = BlackSolverState::new(Puzzle::new([5, 7, 4, 0, 0, 6], [6, 0, 0, 7, 0, 6]));
        assert_eq!(state.count_solutions(2), 2);
    }

    // ── solve() tests ─────────────────────────────────────────────────────────

    #[test]
    fn solve_returns_unique_for_newspaper_puzzle() {
        let state = BlackSolverState::new(Puzzle::new([8, 2, 3, 8, 9, 0], [0, 0, 5, 9, 0, 4]));
        match state.solve() {
            SolveOutcome::Unique(s) => assert!(s.is_solved()),
            other => panic!("expected Unique, got {other:?}"),
        }
    }

    #[test]
    fn solve_returns_multiple_for_underconstrained_puzzle() {
        let state = BlackSolverState::new(Puzzle::new([5, 7, 4, 0, 0, 6], [6, 0, 0, 7, 0, 6]));
        match state.solve() {
            SolveOutcome::Multiple(s) => assert!(s.is_solved()),
            other => panic!("expected Multiple, got {other:?}"),
        }
    }

    #[test]
    fn solve_returns_unsolvable_for_impossible_puzzle() {
        let state = BlackSolverState::new(Puzzle::new([1; 6], [1; 6]));
        assert!(matches!(state.solve(), SolveOutcome::Unsolvable));
    }

    // ── Stats tests ───────────────────────────────────────────────────────────

    #[cfg(debug_assertions)]
    #[test]
    fn stats_track_bits_removed_and_search_nodes() {
        let state = BlackSolverState::new(Puzzle::new([8, 2, 3, 8, 9, 0], [0, 0, 5, 9, 0, 4]));
        let _ = state.solve();
        let s = state.stats();
        assert!(s.search_nodes >= 1, "search_nodes = {}", s.search_nodes);
        assert!(
            s.bits_arc_consistency > 0,
            "bits_arc_consistency = {}",
            s.bits_arc_consistency
        );
    }
}
