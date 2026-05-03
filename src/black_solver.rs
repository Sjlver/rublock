use std::collections::VecDeque;

use tracing::{instrument, trace};

use crate::recorder::{Recorder, Rule, SearchNodes};
use crate::solver::{CellDomain, Puzzle, Solver, Tables};

// ── LiveTuple ─────────────────────────────────────────────────────────────────
// A LiveTuple for this solver works a bit differently from how it's currently
// done in other solvers. Its pattern always covers the full row. There is BLACK
// at the start position, then `len` times the inner domain, then BLACK, then
// `N - 2 - len` times the outer domain.
//
// I guess other solvers could do this too, but you've first seen the idea here :)

#[derive(Clone, Debug)]
struct LiveTuple<const N: usize> {
    pattern: [CellDomain; N],
}

impl<const N: usize> LiveTuple<N> {
    fn new(start: usize, digit_mask: CellDomain) -> Self {
        let len = digit_mask.count_ones() as usize;
        let mut pattern = [0 as CellDomain; N];
        pattern[start] = BlackSolverState::<N>::BLACK;
        pattern[(start + len + 1) % N] = BlackSolverState::<N>::BLACK;
        for p in 0..len {
            pattern[(start + p + 1) % N] = digit_mask;
        }
        for p in 0..N - 2 - len {
            pattern[(start + len + p + 2) % N] = BlackSolverState::<N>::DIGITS & !digit_mask;
        }

        Self { pattern }
    }
}

// ── BlackSolverState ──────────────────────────────────────────────────────────

/// State of the black solver, generic over the propagation event recorder
/// (see [`crate::recorder`]).  Defaults to [`SearchNodes`], the cheapest
/// recorder that still tracks search-tree size.
#[derive(Clone)]
pub struct BlackSolverState<const N: usize, R: Recorder = SearchNodes> {
    pub puzzle: Puzzle<N>,
    domains: [[CellDomain; N]; N],
    /// FIFO queue of pending domain-bit removals.  We use `VecDeque` so the
    /// `propagate()` loop can drain entries in insertion order and detect
    /// "wave" boundaries (one `on_step_start` per wave) — see
    /// [`Self::propagate`].
    queue: VecDeque<(usize, usize, CellDomain)>,

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

    recorder: R,
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

impl<const N: usize, R: Recorder> std::fmt::Debug for BlackSolverState<N, R> {
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
        writeln!(f, "  domain_size =")?;
        for r in 0..N {
            write!(f, "   ")?;
            for c in 0..N {
                write!(f, " ")?;
                write!(f, "{}", self.domain_size[r][c])?;
            }
            writeln!(f)?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

impl<const N: usize, R: Recorder> BlackSolverState<N, R> {
    const BLACK: CellDomain = 1 << 0;
    const DIGITS: CellDomain = ((1 << (N - 2)) - 1) << 1;
    const FULL_DOMAIN: CellDomain = Self::BLACK | Self::DIGITS;

    #[instrument(skip(puzzle))]
    pub fn with_recorder(puzzle: Puzzle<N>) -> Self {
        // Initialize counters from the full domain (before any clear_mask).
        // Domains can be the
        let domain_size: [[u8; N]; N] = [[Self::FULL_DOMAIN.count_ones() as u8; N]; N];

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
            queue: VecDeque::new(),
            domain_size,
            row_candidates,
            col_candidates,
            live_tuples_row,
            live_tuples_col,
            tuple_support_row,
            tuple_support_col,
            recorder: R::default(),
        };

        state.init_live_tuples();
        state.recorder.on_step_start();
        state.seed_queue();
        state.propagate();

        state
    }

    /// The recorder this state writes events to.
    pub fn recorder(&self) -> &R {
        &self.recorder
    }

    /// Enumerate the live tuples
    fn init_live_tuples(&mut self) {
        let tables = Tables::build(N - 2);

        for r in 0..N {
            let target = self.puzzle.row_targets[r] as usize;

            for (len, digit_mask) in tables.valid_tuples_for_target(target) {
                for start in 0..N {
                    if start + len + 2 <= N {
                        let t = LiveTuple::new(start, digit_mask);
                        trace!(
                            row = r,
                            start = start,
                            bits = format_args!("{:0width$b}", digit_mask, width = N - 1),
                            "row tuple live"
                        );
                        self.live_tuples_row[r].push(t);
                    }
                }
            }
        }

        for c in 0..N {
            let target = self.puzzle.col_targets[c] as usize;

            for (len, digit_mask) in tables.valid_tuples_for_target(target) {
                for start in 0..N {
                    if start + len + 2 <= N {
                        let t = LiveTuple::new(start, digit_mask);
                        trace!(
                            col = c,
                            start = start,
                            bits = format_args!("{:0width$b}", digit_mask, width = N - 1),
                            "col tuple live"
                        );
                        self.live_tuples_col[c].push(t);
                    }
                }
            }
        }

        // Initialize support counts from live tuples.
        for r in 0..N {
            for t in &self.live_tuples_row[r] {
                for c in 0..N {
                    let mut bits = t.pattern[c];
                    while bits != 0 {
                        let b = bits.trailing_zeros() as usize;
                        bits &= bits - 1;
                        self.tuple_support_row[r][c][b] += 1;
                    }
                }
            }
        }

        for c in 0..N {
            for t in &self.live_tuples_col[c] {
                for r in 0..N {
                    let mut bits = t.pattern[r];
                    while bits != 0 {
                        let b = bits.trailing_zeros() as usize;
                        bits &= bits - 1;
                        self.tuple_support_col[r][c][b] += 1;
                    }
                }
            }
        }
    }

    /// Seed queue with bits that have no support.
    fn seed_queue(&mut self) {
        // TODO: I'm essentially OR-ing all of self.live_tuples_row[r].
        // There is probably a shorter way to do that.
        let mut row_tuple_supported_bits: [[CellDomain; N]; N] = [[0; N]; N];
        for r in 0..N {
            for t in &self.live_tuples_row[r] {
                for c in 0..N {
                    row_tuple_supported_bits[r][c] |= t.pattern[c];
                }
            }
        }
        let mut col_tuple_supported_bits: [[CellDomain; N]; N] = [[0; N]; N];
        for c in 0..N {
            for t in &self.live_tuples_col[c] {
                for r in 0..N {
                    col_tuple_supported_bits[r][c] |= t.pattern[r];
                }
            }
        }
        for r in 0..N {
            for c in 0..N {
                let supported = row_tuple_supported_bits[r][c] & col_tuple_supported_bits[r][c];
                // Bits with no live-tuple support before any propagation has
                // run: distinct from `ArcConsistency`, which fires later when
                // a tuple's last support disappears.
                self.clear_mask(r, c, !supported & Self::FULL_DOMAIN, Rule::TargetTuples);
            }
        }
    }

    // ── Core mutation primitives ──────────────────────────────────────────────

    #[instrument(skip(self), fields(mask = format_args!("{mask:0width$b}", width = N - 1)))]
    fn clear_mask(&mut self, row: usize, col: usize, mask: CellDomain, rule: Rule) {
        let before = self.domains[row][col];
        self.domains[row][col] &= !mask;
        let after = self.domains[row][col];
        let removed = before & !after;
        if removed != 0 {
            self.recorder.on_bits_removed(row, col, before, after, rule);
        }
        let mut bits = removed;
        while bits != 0 {
            let b = bits & bits.wrapping_neg();
            bits &= bits - 1;
            trace!(b = format_args!("{}", BitName::<N>(b)), "bit removed");
            self.queue.push_back((row, col, b));
        }
    }

    #[instrument(skip(self), fields(bit = format_args!("{}", BitName::<N>(bit))))]
    fn set_cell(&mut self, row: usize, col: usize, bit: CellDomain, rule: Rule) {
        debug_assert_eq!(bit.count_ones(), 1, "set_cell requires exactly one bit");
        debug_assert_eq!(
            bit & !Self::FULL_DOMAIN,
            0,
            "set_cell requires a domain bit"
        );
        trace!(bit = format_args!("{}", BitName::<N>(bit)), "setting cell");

        self.clear_mask(row, col, !bit & Self::FULL_DOMAIN, rule);

        if bit & Self::DIGITS != 0 {
            for c in (0..N).filter(|&c| c != col) {
                self.clear_mask(row, c, bit, rule);
            }
            for r in (0..N).filter(|&r| r != row) {
                self.clear_mask(r, col, bit, rule);
            }
        } else {
            // We've set a black cell. Check if the row/column now has exactly
            // two black bits. If so, remove black from all other cells.
            if let Some(row_black2) = (0..N).find(|&c| c != col && self.domains[row][c] == bit) {
                for c in (0..N).filter(|&c| c != col && c != row_black2) {
                    self.clear_mask(row, c, bit, rule);
                }
            }
            if let Some(col_black2) = (0..N).find(|&r| r != row && self.domains[r][col] == bit) {
                for r in (0..N).filter(|&r| r != row && r != col_black2) {
                    self.clear_mask(r, col, bit, rule);
                }
            }
        }
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
    fn update_hidden_singles(&mut self, row: usize, col: usize, bit: CellDomain) {
        let b = bit.trailing_zeros() as usize;

        self.row_candidates[row][b] -= 1;
        if (bit & Self::DIGITS != 0)
            && (self.row_candidates[row][b] == 1)
            && let Some(c) = (0..N).find(|&c| self.domains[row][c] & bit != 0)
        {
            self.set_cell(row, c, bit, Rule::HiddenSingle);
        }
        if (bit == Self::BLACK)
            && (self.row_candidates[row][b] == 2)
            && let Some(c1) = (0..N).find(|&c| self.domains[row][c] & bit != 0)
            && let Some(c2) = (c1 + 1..N).find(|&c| self.domains[row][c] & bit != 0)
        {
            self.set_cell(row, c1, bit, Rule::HiddenSingle);
            self.set_cell(row, c2, bit, Rule::HiddenSingle);
        }

        self.col_candidates[col][b] -= 1;
        if (bit & Self::DIGITS != 0)
            && (self.col_candidates[col][b] == 1)
            && let Some(r) = (0..N).find(|&r| self.domains[r][col] & bit != 0)
        {
            self.set_cell(r, col, bit, Rule::HiddenSingle);
        }
        if (bit == Self::BLACK)
            && (self.col_candidates[col][b] == 2)
            && let Some(r1) = (0..N).find(|&r| self.domains[r][col] & bit != 0)
            && let Some(r2) = (r1 + 1..N).find(|&r| self.domains[r][col] & bit != 0)
        {
            self.set_cell(r1, col, bit, Rule::HiddenSingle);
            self.set_cell(r2, col, bit, Rule::HiddenSingle);
        }
    }

    #[instrument(skip(self), fields(bit = format_args!("{}", BitName::<N>(bit))))]
    fn update_arc(&mut self, row: usize, col: usize, bit: CellDomain) {
        // Row direction: check tuples in live_row[row] that cover column col.
        let mut i = 0;
        while i < self.live_tuples_row[row].len() {
            let t = &self.live_tuples_row[row][i];
            if t.pattern[col] & bit == 0 {
                i += 1;
                continue;
            }
            // Check liveness: do the cell domains still support this pattern?
            // We check two things here: the tuple might be dead because the current
            // cell no longer supports it, or because there is no more cell that
            // has `bit`.
            if (self.domains[row][col] & t.pattern[col] != 0)
                && (0..N).any(|c| self.domains[row][c] & t.pattern[c] & bit != 0)
            {
                i += 1;
                continue;
            }
            // Tuple is dead: remove and update support counts.
            trace!("row tuple killed");
            let dead = self.live_tuples_row[row].swap_remove(i);
            for c in 0..N {
                let mut bits = dead.pattern[c];
                while bits != 0 {
                    let b = bits.trailing_zeros() as usize;
                    bits &= bits - 1;
                    self.tuple_support_row[row][c][b] -= 1;
                    if self.tuple_support_row[row][c][b] == 0 {
                        self.clear_mask(row, c, 1 << b, Rule::ArcConsistency);
                    }
                }
            }
            // Don't increment i: swap_remove moved the last element here.
        }

        // Column direction: check tuples in live_col[c] that cover row r.
        let mut i = 0;
        while i < self.live_tuples_col[col].len() {
            let t = &self.live_tuples_col[col][i];
            if t.pattern[row] & bit == 0 {
                i += 1;
                continue;
            }
            if (self.domains[row][col] & t.pattern[row] != 0)
                && (0..N).any(|r| self.domains[r][col] & t.pattern[r] & bit != 0)
            {
                i += 1;
                continue;
            }
            trace!("col tuple killed");
            let dead = self.live_tuples_col[col].swap_remove(i);
            for r in 0..N {
                let mut bits = dead.pattern[r];
                while bits != 0 {
                    let b = bits.trailing_zeros() as usize;
                    bits &= bits - 1;
                    self.tuple_support_col[r][col][b] -= 1;
                    if self.tuple_support_col[r][col][b] == 0 {
                        self.clear_mask(r, col, 1 << b, Rule::ArcConsistency);
                    }
                }
            }
        }
    }

    // ── Propagation ───────────────────────────────────────────────────────────

    pub fn propagate(&mut self) {
        while !self.queue.is_empty() {
            self.recorder.on_step_start();
            let mut wave = self.queue.len();
            while wave > 0 {
                let (r, c, bit) = self.queue.pop_front().unwrap();
                wave -= 1;
                if self.domains[r][c] == 0 {
                    return;
                }
                self.update(r, c, bit);
            }
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
// change as a `Backtracking` decision so the recorder attributes it
// appropriately.

impl<const N: usize> BlackSolverState<N, SearchNodes> {
    /// Construct a solver with the default [`SearchNodes`] recorder.
    pub fn new(puzzle: Puzzle<N>) -> Self {
        Self::with_recorder(puzzle)
    }
}

impl<const N: usize, R: Recorder> Solver<N> for BlackSolverState<N, R> {
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
        self.set_cell(r, c, bit, Rule::Backtracking);
    }

    fn reject_branch(&mut self, r: usize, c: usize, bit: CellDomain) {
        self.recorder.on_step_start();
        self.clear_mask(r, c, bit, Rule::Backtracking);
    }

    fn solved_cells(&self) -> Option<[[i8; N]; N]> {
        Self::solved_cells(self)
    }
}

impl<const N: usize, R: Recorder> std::fmt::Display for BlackSolverState<N, R> {
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
    fn live_tuple_init_works_for_zero_digits() {
        // A zero-digit tuple: [BLACK, BLACK]
        let t = LiveTuple::<6>::new(0, 0);
        assert_eq!(
            t.pattern,
            [
                BlackSolverState::<6>::BLACK,
                BlackSolverState::<6>::BLACK,
                BlackSolverState::<6>::DIGITS,
                BlackSolverState::<6>::DIGITS,
                BlackSolverState::<6>::DIGITS,
                BlackSolverState::<6>::DIGITS
            ]
        )
    }

    #[test]
    fn live_tuple_init_works_with_wrapping() {
        // A one-digit wrapping tuple: [BLACK, digit(1), BLACK]
        let t = LiveTuple::<6>::new(5, 1 << 1);
        assert_eq!(
            t.pattern,
            [
                1 << 1,
                BlackSolverState::<6>::BLACK,
                1 << 2 | 1 << 3 | 1 << 4,
                1 << 2 | 1 << 3 | 1 << 4,
                1 << 2 | 1 << 3 | 1 << 4,
                BlackSolverState::<6>::BLACK
            ]
        )
    }

    #[test]
    fn black_solver_state_initializes_correctly() {
        let _ = tracing_subscriber::fmt::try_init();
        let state = BlackSolverState::new(Puzzle::new([0; 4], [0; 4]));

        let expected_domains: [[CellDomain; 4]; 4] = [[BlackSolverState::<4>::FULL_DOMAIN; 4]; 4];
        assert_eq!(state.domains, expected_domains);

        // domains are BLACK and digits 1 and 2.
        assert_eq!(state.domain_size, [[3; 4]; 4]);

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
    fn black_solver_can_fully_propagate() {
        let _ = tracing_subscriber::fmt::try_init();

        // Black solver used to require backtracking on: 2 0 0 3 6 3 0 0 2 0
        let state = BlackSolverState::new(Puzzle::new([2, 0, 0, 3, 6], [3, 0, 0, 2, 0]));
        assert!(state.is_solved());
        assert_eq!(
            state.to_string(),
            concat!(
                "     3   0   0   2   0\n",
                "   +---+---+---+---+---+\n",
                " 2 | 3 | # | 2 | # | 1 |\n",
                "   +---+---+---+---+---+\n",
                " 0 | # | # | 1 | 2 | 3 |\n",
                "   +---+---+---+---+---+\n",
                " 0 | 1 | 3 | # | # | 2 |\n",
                "   +---+---+---+---+---+\n",
                " 3 | 2 | 1 | # | 3 | # |\n",
                "   +---+---+---+---+---+\n",
                " 6 | # | 2 | 3 | 1 | # |\n",
                "   +---+---+---+---+---+\n"
            )
        );
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

    #[test]
    fn stats_track_bits_removed_and_search_nodes() {
        use crate::recorder::FullStats;
        let state = BlackSolverState::<6, FullStats>::with_recorder(Puzzle::new(
            [8, 2, 3, 8, 9, 0],
            [0, 0, 5, 9, 0, 4],
        ));
        let _ = state.solve();
        let s = state.recorder().snapshot();
        assert!(s.search_nodes >= 1, "search_nodes = {}", s.search_nodes);
        // `seed_queue` always removes some bits via `Rule::TargetTuples`
        // for any non-trivial puzzle.
        assert!(
            s.bits_target_tuples > 0,
            "bits_target_tuples = {}",
            s.bits_target_tuples
        );
    }

    #[test]
    fn explain_records_steps_for_solvable_puzzle() {
        use crate::recorder::Explain;
        // Newspaper puzzle with a unique solution.
        let state = BlackSolverState::<6, Explain>::with_recorder(Puzzle::new(
            [8u8, 2, 3, 8, 9, 0],
            [0u8, 0, 5, 9, 0, 4],
        ));
        let outcome = state.solve();
        assert!(
            matches!(&outcome, SolveOutcome::Unique(_)),
            "expected Unique solution"
        );
        let steps = state.recorder().steps();
        // Propagation always produces at least the seed step.
        assert!(!steps.is_empty(), "no steps recorded");
        // Every recorded step must have at least one event (empty steps are discarded).
        for (i, step) in steps.iter().enumerate() {
            assert!(!step.events.is_empty(), "step {i} has no events");
        }
        // At least one step must be attributed to TargetTuples (seed pass).
        let has_target_tuples = steps
            .iter()
            .flat_map(|s| &s.events)
            .any(|e| e.rule == crate::recorder::Rule::TargetTuples);
        assert!(has_target_tuples, "no TargetTuples events recorded");
    }
}
