//! Shared solver vocabulary.
//!
//! This module owns the types that both solver implementations
//! (`BasicSolverState`, `QueueSolverState`) agree on — the puzzle input, the
//! solver output, the bit layout of a cell's domain, and the precomputed
//! digit-subset tables used by arc-consistency.
//!
//! It also defines the [`Solver`] trait: the interface backtracking search
//! needs from any solver implementation.  We use the trait purely for static
//! dispatch (generic functions bounded by `S: Solver<N>`), not `dyn Solver` —
//! see the trait docs below for why.

use std::fmt;

use crate::backtrack;
use crate::recorder::Recorder;

// ── Puzzle ────────────────────────────────────────────────────────────────────

/// The input to the solver: the target number attached to each row and column.
///
/// The grid size is carried in the type via the const generic `N`, so a
/// `Puzzle<6>` and a `Puzzle<7>` are distinct types and can never be confused
/// at call sites.
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

// ── SolveOutcome ──────────────────────────────────────────────────────────────

/// The result of a `solve()` call on a solver state.
///
/// Generic over the state type so every solver implementation can report its
/// own concrete state in the `Unique` / `Multiple` variants.  When there are
/// multiple solutions, we return the **first** one found — enough to display
/// a witness, while still flagging non-uniqueness.
#[derive(Debug, Clone)]
pub enum SolveOutcome<S> {
    /// No assignment satisfies every constraint.
    Unsolvable,
    /// Exactly one solution exists.
    Unique(S),
    /// At least two solutions exist; the enclosed state is the first one found.
    Multiple(S),
}

// ── CellDomain ────────────────────────────────────────────────────────────────
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
//
// In principle, u16 suffices for puzzles up to N=13, and would lead to a smaller
// solver state and faster cloning. However, performance on puzzles that don't
// backtrack seems to decrease for reasons I don't fully understand. Hence
// use u64.
pub type CellDomain = u64;

// ── Tables ────────────────────────────────────────────────────────────────────
//
// `Tables` holds data derived purely from the grid size that is cheap to build
// but reused on every propagation pass.  Both solvers use it; the basic solver
// builds one per `new()` (shared across backtracking clones via `Arc`), and the
// queue solver rebuilds one once in `new()` to seed its live-tuple list.
//
// All fields are `Vec`-based because their sizes depend on `num_digits = N-2`,
// which is only known at runtime.

#[derive(Debug)]
pub(crate) struct Tables {
    /// For each (target, size) pair, the list of valid digit-set bitmasks.
    ///
    /// A valid digit-set for cage target `t` and size `k` is any k-element
    /// subset of the digit set whose elements sum to `t`.  Each set is encoded
    /// as a `CellDomain` with bit `d` set (i.e. `1 << d`) if digit `d` belongs to
    /// the set — the same layout used for cell domains.
    ///
    /// Indexed as `valid_tuples[target][size]`.
    valid_tuples: Vec<Vec<Vec<CellDomain>>>,

    /// Maximum achievable cage sum (= 1 + 2 + ... + num_digits).
    pub(crate) max_sum: usize,
}

impl Tables {
    /// Build tables for a grid whose rows/columns contain `num_digits` distinct
    /// digit values (i.e. `num_digits = N - 2` for an N×N grid).
    pub(crate) fn build(num_digits: usize) -> Self {
        // Digits are 1..=num_digits; max achievable cage sum is their total.
        let max_target: usize = (1..=num_digits).sum();
        let num_targets = max_target + 1;

        // valid_tuples[target][size]: one Vec per (target, size) pair.
        let mut valid_tuples: Vec<Vec<Vec<CellDomain>>> =
            vec![vec![vec![]; num_digits + 1]; num_targets];

        // Iterate over every subset of the digit set {1, …, num_digits}.
        // For each subset, its size and sum determine exactly which slot it
        // belongs in — no inner loops or filtering needed.
        for subset in 0 as CellDomain..(1 as CellDomain) << num_digits {
            let size = subset.count_ones() as usize;
            let target: usize = (0..num_digits)
                .filter(|&b| subset & (1 << b) != 0)
                .map(|b| b + 1) // bit b represents digit b+1
                .sum();
            // Shift left by 1: bit b (digit b+1) → bit b+1 in the domain mask.
            valid_tuples[target][size].push(subset << 1);
        }

        Self {
            valid_tuples,
            max_sum: max_target,
        }
    }

    /// Returns `(size, tuple)` for every valid tuple with the given `target`.
    pub(crate) fn valid_tuples_for_target(
        &self,
        target: usize,
    ) -> impl Iterator<Item = (usize, CellDomain)> {
        self.valid_tuples[target]
            .iter()
            .enumerate()
            .flat_map(|(l, ts)| ts.iter().map(move |&t| (l, t)))
    }
}

// ── Solver trait ──────────────────────────────────────────────────────────────

/// The operations every solver implementation exposes to the rest of the crate.
///
/// The trait serves two purposes:
///
/// 1.  It documents the **shared interface** of the solver implementations.
///     Anything that is conceptually a "solver" has to be able to propagate
///     constraints, decide when the grid is solved or contradictory, and pick
///     the next branching point for backtracking search.
/// 2.  It lets generic code (notably [`crate::backtrack`]) drive any solver
///     without knowing its concrete type.
///
/// ## Why not `dyn Solver`?
///
/// The trait is `Sized`: implementors are cloned during backtracking (to save
/// a snapshot before each guess).  Method signatures also return `Option<(usize,
/// usize)>` and take `CellDomain` rather than boxed abstractions — they are
/// designed to be inlined.  Consumers use static dispatch: `fn f<S: Solver<N>,
/// const N: usize>(s: &S)`, which monomorphises per concrete solver and keeps
/// the hot per-bit operations as fast as the hand-coded versions were.
///
/// There is no `dyn Solver` anywhere in the crate, and none is needed.
pub trait Solver<const N: usize>: Sized + Clone + fmt::Display {
    /// The recorder type the solver state holds.  Defaults to
    /// `recorder::SearchNodes` on each concrete state struct (set via the
    /// type parameter default), so callers that don't care about stats just
    /// write `BlackSolverState::<N>::new(puzzle)` and pay only for a single
    /// search-node counter.
    type Recorder: Recorder;

    /// Build a fresh solver for `puzzle`.
    ///
    /// Implementations may do some upfront work (e.g. the queue solver seeds
    /// its propagation queue from arc-consistency), but must leave the state
    /// in a consistent, propagatable form.
    fn new(puzzle: Puzzle<N>) -> Self;

    /// The recorder the state writes propagation events to.  `backtrack`
    /// calls `recorder().on_search_node()` at every branch; user code can
    /// downcast to a concrete type (e.g. `FullStats`) to read its data.
    fn recorder(&self) -> &Self::Recorder;

    /// Run all propagation rules to a fixpoint (no rule shrinks a domain
    /// further).  After `propagate`, the state is either solved, contradictory,
    /// or stuck — in which case the caller must branch.
    fn propagate(&mut self);

    /// Every cell has been uniquely determined.
    fn is_solved(&self) -> bool;

    /// Some cell's domain is empty: the current partial assignment cannot be
    /// completed, and the caller should prune this branch.
    fn is_contradiction(&self) -> bool;

    /// The most-constrained cell with more than one remaining choice, or
    /// `None` if every cell is fully determined.
    fn pick_branching_cell(&self) -> Option<(usize, usize)>;

    /// A single bit from the cell's domain to commit to next.
    ///
    /// Takes `&mut self` because the queue solver reads support counts through
    /// a mutable accessor.  The basic solver doesn't need to mutate, but the
    /// borrow costs nothing during backtracking, where we already hold the
    /// state as `mut`.
    fn pick_branching_bit(&mut self, row: usize, col: usize) -> CellDomain;

    /// Commit to `bit` at `(r, c)` as a branching decision, propagating any
    /// immediate consequences (e.g. clearing the bit elsewhere in the row/col).
    ///
    /// Distinct from the solver's internal `set_cell`, which is the
    /// *propagation* primitive: the trait names the intent at the search-tree
    /// level, and each implementation forwards with `Rule::Backtracking`.
    fn take_branch(&mut self, r: usize, c: usize, bit: CellDomain);

    /// Exclude `bit` from `(r, c)` as a branching decision (the complement
    /// path), propagating consequences.  See [`take_branch`](Self::take_branch).
    fn reject_branch(&mut self, r: usize, c: usize, bit: CellDomain);

    /// Return the solved grid as `-1` for black and positive digits otherwise.
    ///
    /// Returns `None` when the state is not fully solved.
    fn solved_cells(&self) -> Option<[[i8; N]; N]>;

    // ── Provided backtracking entry points ────────────────────────────────────
    //
    // These forward to free generic functions in `crate::backtrack` so the
    // search loop lives in one place.  They're on the trait so call sites can
    // write `state.count_solutions(max)` rather than `backtrack::count_solutions(&state, max)`.

    /// See [`backtrack::count_solutions`].
    fn count_solutions(&self, max: usize) -> usize {
        backtrack::count_solutions(self, max)
    }

    /// See [`backtrack::solve`].
    fn solve(&self) -> SolveOutcome<Self> {
        backtrack::solve(self)
    }
}
