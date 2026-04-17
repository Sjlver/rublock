use tracing::{instrument, trace};

use crate::solver::{Puzzle, Tables};

type CellDomain = u16;

// ── LiveTuple ─────────────────────────────────────────────────────────────────

#[derive(Clone, Debug)]
struct LiveTuple<const N: usize> {
    start: u8,
    len: u8,
    pattern: [CellDomain; N],
}

impl<const N: usize> LiveTuple<N> {
    fn new(start: usize, len: usize, digit_mask: CellDomain, black1: CellDomain, black2: CellDomain) -> Self {
        let mut pattern = [0 as CellDomain; N];
        pattern[0] = black1;
        for i in 1..=len {
            pattern[i] = digit_mask;
        }
        pattern[len + 1] = black2;
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

// ── QueueSolverState ──────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct QueueSolverState<const N: usize> {
    pub puzzle: Puzzle<N>,
    domains: [[CellDomain; N]; N],
    queue: Vec<(usize, usize, CellDomain)>,

    // ── Singleton constraint ──────────────────────────────────────────────────
    // How many value-choices does this cell have in the row / col view?
    row_domain_size: [[u8; N]; N],
    col_domain_size: [[u8; N]; N],

    // ── Hidden singles constraint ─────────────────────────────────────────────
    // Number of cells in row r (col c) whose domain has a given bit set.
    // Split into digit and black arrays, analogous to tuple_support_*.
    row_candidates_digit: [[u8; N]; N],
    row_candidates_black: [[u8; 2]; N],
    col_candidates_digit: [[u8; N]; N],
    col_candidates_black: [[u8; 2]; N],

    // ── Black consistency constraint ──────────────────────────────────────────
    row_blacks_left: [[u8; N]; N],
    col_blacks_left: [[u8; N]; N],

    // ── General arc consistency constraint ────────────────────────────────────
    live_tuples_row: [Vec<LiveTuple<N>>; N],
    live_tuples_col: [Vec<LiveTuple<N>>; N],

    // tuple_support_row_digit[r][c][p] = number of live row-direction tuples in row r
    //   whose pattern at column c includes digit bit (1 << p). p = trailing_zeros.
    // tuple_support_row_black[r][c][k] = same for row-black bits:
    //   k = 0 for BLACK1_ROW, k = 1 for BLACK2_ROW.
    // Analogously for col (BLACK1_COL / BLACK2_COL).
    //
    // This could be a single array indexed by p; however, that would have size N+3,
    // which stable Rust does not allow. The original implementation used a Vec of
    // size N+3; the current version handles black bits separately.
    tuple_support_row_digit: [[[u16; N]; N]; N],
    tuple_support_row_black: [[[u16; 2]; N]; N],
    tuple_support_col_digit: [[[u16; N]; N]; N],
    tuple_support_col_black: [[[u16; 2]; N]; N],
}

struct BitName<const N: usize>(CellDomain);

impl<const N: usize> std::fmt::Display for BitName<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let b = self.0;
        if b == QueueSolverState::<N>::BLACK1_ROW {
            write!(f, "BLACK1_ROW")
        } else if b == QueueSolverState::<N>::BLACK2_ROW {
            write!(f, "BLACK2_ROW")
        } else if b == QueueSolverState::<N>::BLACK1_COL {
            write!(f, "BLACK1_COL")
        } else if b == QueueSolverState::<N>::BLACK2_COL {
            write!(f, "BLACK2_COL")
        } else if b & QueueSolverState::<N>::ALL_DIGITS != 0 {
            write!(f, "DIGIT_{}", b.trailing_zeros())
        } else {
            panic!("BitName: {b:#b} is not a valid single bit")
        }
    }
}

impl<const N: usize> std::fmt::Debug for QueueSolverState<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "QueueSolverState {{")?;
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
                write!(f, "{:0width$b}", self.domains[r][c], width = N + 3)?;
            }
            writeln!(f)?;
        }
        writeln!(f, "}}")?;
        Ok(())
    }
}

impl<const N: usize> QueueSolverState<N> {
    const BLACK1_ROW: CellDomain = 1 << (N - 1);
    const BLACK2_ROW: CellDomain = 1 << N;
    const BLACK1_COL: CellDomain = 1 << (N + 1);
    const BLACK2_COL: CellDomain = 1 << (N + 2);

    const ALL_DIGITS: CellDomain = ((1 << (N - 2)) - 1) << 1;
    const ROW_BLACKS: CellDomain = Self::BLACK1_ROW | Self::BLACK2_ROW;
    const COL_BLACKS: CellDomain = Self::BLACK1_COL | Self::BLACK2_COL;
    const ALL_BLACKS: CellDomain = Self::ROW_BLACKS | Self::COL_BLACKS;

    #[instrument(skip(puzzle))]
    pub fn new(puzzle: Puzzle<N>) -> Self {
        let full_cell: CellDomain = Self::ALL_DIGITS | Self::ALL_BLACKS;

        // Initialize counters from the full domain (before any clear_mask).
        let row_domain_size: [[u8; N]; N] = [[N as u8; N]; N];
        let col_domain_size: [[u8; N]; N] = [[N as u8; N]; N];
        let row_blacks_left: [[u8; N]; N] = [[2; N]; N];
        let col_blacks_left: [[u8; N]; N] = [[2; N]; N];

        // Digit indices 1..=N-2 are valid; 0 and N-1 are not digit bits.
        let candidates_digit_line: [u8; N] = std::array::from_fn(|p| {
            if p >= 1 && p <= N - 2 { N as u8 } else { 0 }
        });
        let row_candidates_digit: [[u8; N]; N] = [candidates_digit_line; N];
        let row_candidates_black: [[u8; 2]; N] = [[N as u8; 2]; N];
        let col_candidates_digit: [[u8; N]; N] = [candidates_digit_line; N];
        let col_candidates_black: [[u8; 2]; N] = [[N as u8; 2]; N];

        let live_tuples_row: [Vec<LiveTuple<N>>; N] = std::array::from_fn(|_| Vec::new());
        let live_tuples_col: [Vec<LiveTuple<N>>; N] = std::array::from_fn(|_| Vec::new());

        let tuple_support_row_digit: [[[u16; N]; N]; N] = [[[0u16; N]; N]; N];
        let tuple_support_row_black: [[[u16; 2]; N]; N] = [[[0u16; 2]; N]; N];
        let tuple_support_col_digit: [[[u16; N]; N]; N] = [[[0u16; N]; N]; N];
        let tuple_support_col_black: [[[u16; 2]; N]; N] = [[[0u16; 2]; N]; N];

        let mut state = Self {
            puzzle,
            domains: [[full_cell; N]; N],
            queue: Vec::new(),
            row_domain_size,
            col_domain_size,
            row_candidates_digit,
            row_candidates_black,
            col_candidates_digit,
            col_candidates_black,
            row_blacks_left,
            col_blacks_left,
            live_tuples_row,
            live_tuples_col,
            tuple_support_row_digit,
            tuple_support_row_black,
            tuple_support_col_digit,
            tuple_support_col_black,
        };

        state.init_live_tuples();
        state.seed_queue();
        state.propagate();

        state
    }

    /// Enumerate the live tuples
    fn init_live_tuples(&mut self) {
        let tables = Tables::build(N - 2);

        for r in 0..N {
            let inside_target = self.puzzle.row_targets[r] as usize;
            let outside_target = tables.max_sum - inside_target;

            // Inside (non-wrapping): [BLACK1_ROW, digit..., BLACK2_ROW]
            for (len, digit_mask) in tables.valid_tuples_for_target(inside_target) {
                for start in 0..N {
                    if start + len + 2 <= N {
                        let t = LiveTuple::new(
                            start,
                            len,
                            digit_mask,
                            Self::BLACK1_ROW,
                            Self::BLACK2_ROW,
                        );
                        trace!(
                            row = r,
                            start = start,
                            bits = format_args!(
                                "{:0width$b}",
                                if t.len > 2 { t.pattern[1] } else { 0 },
                                width = N + 3
                            ),
                            "inside row tuple live"
                        );
                        self.live_tuples_row[r].push(t);
                    }
                }
            }

            // Outside (wrapping): [BLACK2_ROW, digit..., BLACK1_ROW]
            for (len, digit_mask) in tables.valid_tuples_for_target(outside_target) {
                for start in 0..N {
                    if start + len + 2 > N {
                        let t = LiveTuple::new(
                            start,
                            len,
                            digit_mask,
                            Self::BLACK2_ROW,
                            Self::BLACK1_ROW,
                        );
                        trace!(
                            row = r,
                            start = start,
                            bits = format_args!(
                                "{:0width$b}",
                                if t.len > 2 { t.pattern[1] } else { 0 },
                                width = N + 3
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

            // Inside (non-wrapping): [BLACK1_COL, digit..., BLACK2_COL]
            for (len, digit_mask) in tables.valid_tuples_for_target(inside_target) {
                for start in 0..N {
                    if start + len + 2 <= N {
                        let t = LiveTuple::new(
                            start,
                            len,
                            digit_mask,
                            Self::BLACK1_COL,
                            Self::BLACK2_COL,
                        );
                        trace!(
                            col = c,
                            start = start,
                            bits = format_args!(
                                "{:0width$b}",
                                if t.len > 2 { t.pattern[1] } else { 0 },
                                width = N + 3
                            ),
                            "inside col tuple live"
                        );
                        self.live_tuples_col[c].push(t);
                    }
                }
            }

            // Outside (wrapping): [BLACK2_COL, digit..., BLACK1_COL]
            for (len, digit_mask) in tables.valid_tuples_for_target(outside_target) {
                for start in 0..N {
                    if start + len + 2 > N {
                        let t = LiveTuple::new(
                            start,
                            len,
                            digit_mask,
                            Self::BLACK2_COL,
                            Self::BLACK1_COL,
                        );
                        trace!(
                            col = c,
                            start = start,
                            bits = format_args!(
                                "{:0width$b}",
                                if t.len > 2 { t.pattern[1] } else { 0 },
                                width = N + 3
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
            // Temporarily move self.live_tuples_row[r]. This prevents a borrow,
            // which is incompatible with the self.tuple_support_row call below.
            let live_tuples = std::mem::take(&mut self.live_tuples_row[r]);
            for t in &live_tuples {
                for (c2, mut bits) in t.cells() {
                    while bits != 0 {
                        let b = bits & bits.wrapping_neg();
                        bits &= bits - 1;
                        *self.tuple_support_row(r, c2, b) += 1;
                    }
                }
            }
            self.live_tuples_row[r] = live_tuples;
        }

        for c in 0..N {
            let live_tuples = std::mem::take(&mut self.live_tuples_col[c]);
            for t in &live_tuples {
                for (r2, mut bits) in t.cells() {
                    while bits != 0 {
                        let b = bits & bits.wrapping_neg();
                        bits &= bits - 1;
                        *self.tuple_support_col(r2, c, b) += 1;
                    }
                }
            }
            self.live_tuples_col[c] = live_tuples;
        }
    }

    /// Seed queue with bits that have no support.
    fn seed_queue(&mut self) {
        let full_cell: CellDomain = Self::ALL_DIGITS | Self::ALL_BLACKS;

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
                let supported = (row_tuple_supported_bits[r][c] & Self::ROW_BLACKS)
                    | (col_tuple_supported_bits[r][c] & Self::COL_BLACKS)
                    | (row_tuple_supported_bits[r][c]
                        & col_tuple_supported_bits[r][c]
                        & Self::ALL_DIGITS);
                self.clear_mask(r, c, !supported & full_cell);
            }
        }
    }

    // ── Core mutation primitives ──────────────────────────────────────────────

    #[instrument(skip(self), fields(mask = format_args!("{mask:0width$b}", width = N + 3)))]
    fn clear_mask(&mut self, r: usize, c: usize, mask: CellDomain) {
        let before = self.domains[r][c];
        self.domains[r][c] &= !mask;
        let removed = before & !self.domains[r][c];
        let mut bits = removed;
        while bits != 0 {
            let b = bits & bits.wrapping_neg();
            bits &= bits - 1;
            trace!(b = format_args!("{}", BitName::<N>(b)), "bit removed");
            self.queue.push((r, c, b));
        }
    }

    #[instrument(skip(self), fields(bit = format_args!("{}", BitName::<N>(bit))))]
    fn set_cell(&mut self, r: usize, c: usize, bit: CellDomain) {
        debug_assert_eq!(bit.count_ones(), 1, "set_cell requires exactly one bit");
        trace!(bit = format_args!("{}", BitName::<N>(bit)), "setting cell");

        if bit & Self::ALL_DIGITS != 0 {
            for col in (0..N).filter(|&col| col != c) {
                self.clear_mask(r, col, bit);
            }
            for row in (0..N).filter(|&row| row != r) {
                self.clear_mask(row, c, bit);
            }
            self.clear_mask(r, c, !bit);
        } else if bit & Self::ROW_BLACKS != 0 {
            for col in (0..N).filter(|&col| col != c) {
                self.clear_mask(r, col, bit);
            }
            self.clear_mask(r, c, Self::ALL_DIGITS | (Self::ROW_BLACKS & !bit));
        } else if bit & Self::COL_BLACKS != 0 {
            for row in (0..N).filter(|&row| row != r) {
                self.clear_mask(row, c, bit);
            }
            self.clear_mask(r, c, Self::ALL_DIGITS | (Self::COL_BLACKS & !bit));
        }

        if bit & Self::ALL_BLACKS != 0 {
            for left in 0..c {
                self.clear_mask(r, left, Self::BLACK2_ROW);
            }
            for right in c + 1..N {
                self.clear_mask(r, right, Self::BLACK1_ROW);
            }
            for above in 0..r {
                self.clear_mask(above, c, Self::BLACK2_COL);
            }
            for below in r + 1..N {
                self.clear_mask(below, c, Self::BLACK1_COL);
            }
        }
    }

    // ── Support count accessors ───────────────────────────────────────────────

    fn tuple_support_row(&mut self, r: usize, c: usize, b: CellDomain) -> &mut u16 {
        if b & Self::ALL_DIGITS != 0 {
            &mut self.tuple_support_row_digit[r][c][b.trailing_zeros() as usize]
        } else {
            &mut self.tuple_support_row_black[r][c][(b == Self::BLACK2_ROW) as usize]
        }
    }

    fn tuple_support_col(&mut self, r: usize, c: usize, b: CellDomain) -> &mut u16 {
        if b & Self::ALL_DIGITS != 0 {
            &mut self.tuple_support_col_digit[r][c][b.trailing_zeros() as usize]
        } else {
            &mut self.tuple_support_col_black[r][c][(b == Self::BLACK2_COL) as usize]
        }
    }

    fn row_candidates_for(&mut self, r: usize, bit: CellDomain) -> &mut u8 {
        if bit & Self::ALL_DIGITS != 0 {
            &mut self.row_candidates_digit[r][bit.trailing_zeros() as usize]
        } else {
            &mut self.row_candidates_black[r][(bit == Self::BLACK2_ROW) as usize]
        }
    }

    fn col_candidates_for(&mut self, c: usize, bit: CellDomain) -> &mut u8 {
        if bit & Self::ALL_DIGITS != 0 {
            &mut self.col_candidates_digit[c][bit.trailing_zeros() as usize]
        } else {
            &mut self.col_candidates_black[c][(bit == Self::BLACK2_COL) as usize]
        }
    }

    // ── Update handlers ───────────────────────────────────────────────────────

    fn update(&mut self, r: usize, c: usize, bit: CellDomain) {
        trace!(
            r = r,
            c = c,
            bit = format_args!("{}", BitName::<N>(bit)),
            "update"
        );
        self.update_singleton(r, c, bit);
        self.update_hidden_singles(r, c, bit);
        self.update_black_consistency(r, c, bit);
        self.update_arc(r, c, bit);
    }

    #[instrument(skip(self), fields(bit = format_args!("{}", BitName::<N>(bit))))]
    fn update_singleton(&mut self, r: usize, c: usize, bit: CellDomain) {
        if bit & (Self::ALL_DIGITS | Self::ROW_BLACKS) != 0 {
            self.row_domain_size[r][c] -= 1;
            if self.row_domain_size[r][c] == 1 {
                let row_domain = self.domains[r][c] & (Self::ALL_DIGITS | Self::ROW_BLACKS);
                if row_domain.count_ones() == 1 {
                    self.set_cell(r, c, row_domain);
                }
            }
        }

        if bit & (Self::ALL_DIGITS | Self::COL_BLACKS) != 0 {
            self.col_domain_size[r][c] -= 1;
            if self.col_domain_size[r][c] == 1 {
                let col_domain = self.domains[r][c] & (Self::ALL_DIGITS | Self::COL_BLACKS);
                if col_domain.count_ones() == 1 {
                    self.set_cell(r, c, col_domain);
                }
            }
        }
    }

    #[instrument(skip(self), fields(bit = format_args!("{}", BitName::<N>(bit))))]
    fn update_hidden_singles(&mut self, r: usize, c: usize, bit: CellDomain) {
        if bit & (Self::ALL_DIGITS | Self::ROW_BLACKS) != 0 {
            *self.row_candidates_for(r, bit) -= 1;
            if *self.row_candidates_for(r, bit) == 1 {
                if let Some(c2) = (0..N).find(|&col| self.domains[r][col] & bit != 0) {
                    self.set_cell(r, c2, bit);
                }
            }
        }

        if bit & (Self::ALL_DIGITS | Self::COL_BLACKS) != 0 {
            *self.col_candidates_for(c, bit) -= 1;
            if *self.col_candidates_for(c, bit) == 1 {
                if let Some(r2) = (0..N).find(|&row| self.domains[row][c] & bit != 0) {
                    self.set_cell(r2, c, bit);
                }
            }
        }
    }

    #[instrument(skip(self), fields(bit = format_args!("{}", BitName::<N>(bit))))]
    fn update_black_consistency(&mut self, r: usize, c: usize, bit: CellDomain) {
        if bit & Self::ROW_BLACKS != 0 {
            self.row_blacks_left[r][c] -= 1;
            if self.row_blacks_left[r][c] == 0 {
                self.clear_mask(r, c, Self::COL_BLACKS);
            }
        }

        if bit & Self::COL_BLACKS != 0 {
            self.col_blacks_left[r][c] -= 1;
            if self.col_blacks_left[r][c] == 0 {
                self.clear_mask(r, c, Self::ROW_BLACKS);
            }
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
                    width = N + 3
                ),
                "row tuple killed"
            );
            let dead = self.live_tuples_row[r].swap_remove(i);
            for (c2, mut bits) in dead.cells() {
                while bits != 0 {
                    let b = bits & bits.wrapping_neg();
                    bits &= bits - 1;
                    *self.tuple_support_row(r, c2, b) -= 1;
                    if *self.tuple_support_row(r, c2, b) == 0 {
                        self.clear_mask(r, c2, b);
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
                    width = N + 3
                ),
                "col tuple killed"
            );
            let dead = self.live_tuples_col[c].swap_remove(i);
            for (r2, mut bits) in dead.cells() {
                while bits != 0 {
                    let b = bits & bits.wrapping_neg();
                    bits &= bits - 1;
                    *self.tuple_support_col(r2, c, b) -= 1;
                    if *self.tuple_support_col(r2, c, b) == 0 {
                        self.clear_mask(r2, c, b);
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
        self.domains.iter().flatten().all(|&d| {
            let digits = d & Self::ALL_DIGITS;
            let row_blacks = d & Self::ROW_BLACKS;
            let col_blacks = d & Self::COL_BLACKS;
            if digits != 0 {
                digits.count_ones() == 1 && row_blacks == 0 && col_blacks == 0
            } else {
                row_blacks.count_ones() == 1 && col_blacks.count_ones() == 1
            }
        })
    }

    fn branching_bits(domain: CellDomain) -> CellDomain {
        let primary = domain & (Self::ALL_DIGITS | Self::ROW_BLACKS);
        if primary.count_ones() > 1 { primary } else { 0 }
    }

    fn pick_branching_cell(&self) -> Option<(usize, usize)> {
        let mut best: Option<(usize, usize, u32)> = None;
        for r in 0..N {
            for c in 0..N {
                let bits = Self::branching_bits(self.domains[r][c]);
                let freedom = bits.count_ones();
                if freedom > 1 && best.map_or(true, |b| freedom < b.2) {
                    if freedom == 2 {
                        return Some((r, c));
                    }
                    best = Some((r, c, freedom));
                }
            }
        }
        best.map(|(r, c, _)| (r, c))
    }

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
            dbg!(state);
            panic!("Propagation stalled");
        };

        let bits = Self::branching_bits(state.domains[row][col]);
        let bit = 1 << bits.trailing_zeros();
        let mut branch = state.clone();
        branch.set_cell(row, col, bit);
        let branch_solutions = branch.count_solutions(max);

        state.clear_mask(row, col, bit);
        branch_solutions + state.count_solutions(max - branch_solutions)
    }
}

impl<const N: usize> std::fmt::Display for QueueSolverState<N> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let ct = &self.puzzle.col_targets;
        write!(f, "    ")?;
        for c in 0..N {
            if c > 0 {
                write!(f, "  ")?;
            }
            write!(f, "{:2}", ct[c])?;
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

    #[test]
    fn live_tuple_pos_at_works() {
        // A zero-digit tuple: [BLACK1_ROW, BLACK2_ROW]
        let t0 = LiveTuple::<6>::new(
            0,
            0,
            0,
            QueueSolverState::<6>::BLACK1_ROW,
            QueueSolverState::<6>::BLACK2_ROW,
        );
        assert_eq!(t0.pos_of(0), Some(0));
        assert_eq!(t0.pos_of(1), Some(1));
        assert_eq!(t0.pos_of(2), None);
        assert_eq!(t0.pos_of(3), None);
        assert_eq!(t0.pos_of(4), None);
        assert_eq!(t0.pos_of(5), None);
    }

    #[test]
    fn live_tuple_pos_at_works_with_wrapping() {
        // A one-digit wrapping tuple: [BLACK2_ROW, digit(1), BLACK1_ROW]
        let t0 = LiveTuple::<6>::new(
            5,
            1,
            1 << 1,
            QueueSolverState::<6>::BLACK2_ROW,
            QueueSolverState::<6>::BLACK1_ROW,
        );
        assert_eq!(t0.pos_of(0), Some(1));
        assert_eq!(t0.pos_of(1), Some(2));
        assert_eq!(t0.pos_of(2), None);
        assert_eq!(t0.pos_of(3), None);
        assert_eq!(t0.pos_of(4), None);
        assert_eq!(t0.pos_of(5), Some(0));
    }

    #[test]
    fn queue_solver_state_initializes_correctly() {
        let _ = tracing_subscriber::fmt::try_init();
        let state = QueueSolverState::new(Puzzle::new([0; 4], [0; 4]));

        let b = |cond, val: CellDomain| if cond { val } else { 0 };
        let expected_domains: [[CellDomain; 4]; 4] = std::array::from_fn(|r| {
            std::array::from_fn(|c| {
                QueueSolverState::<4>::ALL_DIGITS
                    | b(c != 0, QueueSolverState::<4>::BLACK2_ROW)
                    | b(c != 3, QueueSolverState::<4>::BLACK1_ROW)
                    | b(r != 0, QueueSolverState::<4>::BLACK2_COL)
                    | b(r != 3, QueueSolverState::<4>::BLACK1_COL)
            })
        });
        assert_eq!(state.domains, expected_domains);

        assert_eq!(state.row_domain_size, [[3, 4, 4, 3]; 4]);
        assert_eq!(state.col_domain_size, [[3; 4], [4; 4], [4; 4], [3; 4]]);

        // row_candidates: digits 1,2 have 4 candidates each; blacks have 3,3
        assert_eq!(state.row_candidates_digit, [[0, 4, 4, 0]; 4]);
        assert_eq!(state.row_candidates_black, [[3, 3]; 4]);
        // col_candidates: digits 1,2 have 4 candidates each; blacks have 3,3
        assert_eq!(state.col_candidates_digit, [[0, 4, 4, 0]; 4]);
        assert_eq!(state.col_candidates_black, [[3, 3]; 4]);

        assert_eq!(state.row_blacks_left, [[1, 2, 2, 1]; 4]);
        assert_eq!(state.col_blacks_left, [[1; 4], [2; 4], [2; 4], [1; 4]]);
    }

    #[test]
    fn propagation_target_9() {
        let _ = tracing_subscriber::fmt::try_init();
        // With target = 9, black-1 may only be at positions 0 and 1.
        // Similarly, the value 1 must be in the outside cage at the end of the row.
        let state = QueueSolverState::new(Puzzle::new([9, 0, 0, 0, 0, 0], [0; 6]));

        assert_eq!(
            state.domains[0].map(|d| d & QueueSolverState::<6>::ROW_BLACKS),
            [
                QueueSolverState::<6>::BLACK1_ROW,
                QueueSolverState::<6>::BLACK1_ROW,
                0,
                0,
                QueueSolverState::<6>::BLACK2_ROW,
                QueueSolverState::<6>::BLACK2_ROW,
            ]
        );
        let bit1 = 1 << 1;
        assert_eq!(state.domains[0].map(|d| d & bit1), [bit1, 0, 0, 0, 0, bit1]);
    }

    #[test]
    fn sample_puzzle() {
        // puzzle = [5, 7, 4, 0, 0, 6] [6, 0, 0, 7, 0, 6]
        // used to stall at:
        // domains =
        //   000010000 010100000 010100000 000000100 011000000 011000000
        //   010100000 110100000 110100000 011000000 101000000 011000000
        //   010100000 110100000 111000000 011000000 000000100 000000010
        //   000000010 100100000 101000000 000001000 010100000 101000000
        //   000000100 000000010 000001000 100000000 111000000 101000000
        //   100100000 000001000 000000100 101000000 101000000 000010000

        let state = QueueSolverState::new(Puzzle::new([5, 7, 4, 0, 0, 6], [6, 0, 0, 7, 0, 6]));
        assert_eq!(state.count_solutions(2), 2);
    }
}
