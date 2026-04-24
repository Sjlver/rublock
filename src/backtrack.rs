//! Generic backtracking search over anything that implements [`Solver`].
//!
//! The search loop is the same for every solver implementation: propagate to a
//! fixpoint, return early on contradiction or a complete assignment, otherwise
//! pick a cell, guess one of its remaining bits, and recurse on both branches.
//! Centralising it here keeps each solver focused on its own propagation
//! machinery and removes the near-duplicate copies that used to live in both
//! `basic_solver.rs` and `queue_solver.rs`.
//!
//! These are free functions rather than methods because the [`Solver`] trait's
//! default methods delegate to them — callers still write `state.solve()` at
//! the call site, but the implementation only lives here.

use crate::solver::{SolveOutcome, Solver};

/// Count the number of distinct solutions, stopping once `max` is reached.
///
/// Returns the number of solutions found, which is at most `max`.
///
/// Typical usage:
/// - `max = 1` — satisfiability test.
/// - `max = 2` — uniqueness test: `1` means unique, `2` means multiple.
///
/// The solver is cloned before each guess so sibling branches don't see each
/// other's propagation side-effects.
pub fn count_solutions<S, const N: usize>(solver: &S, max: usize) -> usize
where
    S: Solver<N>,
{
    if max == 0 {
        return 0;
    }

    solver.stats_handle().incr_node();

    let mut state = solver.clone();
    state.propagate();

    if state.is_contradiction() {
        return 0;
    }
    if state.is_solved() {
        return 1;
    }

    let Some((row, col)) = state.pick_branching_cell() else {
        // Propagation stalled but the grid is neither solved nor contradicted.
        // This shouldn't happen — the propagation rules are supposed to be
        // complete enough to always leave a branchable cell.
        panic!("Propagation stalled");
    };

    let bit = state.pick_branching_bit(row, col);
    let mut branch = state.clone();
    branch.take_branch(row, col, bit);
    let branch_solutions = count_solutions(&branch, max);

    state.reject_branch(row, col, bit);
    branch_solutions + count_solutions(&state, max - branch_solutions)
}

/// Run backtracking search and fill `out` with up to `limit` solved states.
///
/// Same shape as [`count_solutions`] but keeps the solved states so the caller
/// can display them.  Stops as soon as `out.len() == limit`.
pub fn collect_solutions<S, const N: usize>(solver: &S, limit: usize, out: &mut Vec<S>)
where
    S: Solver<N>,
{
    if out.len() >= limit {
        return;
    }

    solver.stats_handle().incr_node();

    let mut state = solver.clone();
    state.propagate();

    if state.is_contradiction() {
        return;
    }
    if state.is_solved() {
        out.push(state);
        return;
    }

    let Some((row, col)) = state.pick_branching_cell() else {
        panic!("Propagation stalled");
    };

    let bit = state.pick_branching_bit(row, col);

    let mut branch = state.clone();
    branch.take_branch(row, col, bit);
    collect_solutions(&branch, limit, out);

    if out.len() >= limit {
        return;
    }

    state.reject_branch(row, col, bit);
    collect_solutions(&state, limit, out);
}

/// Solve the puzzle, reporting uniqueness.
///
/// Searches for up to two solutions so the outcome can distinguish `Unique`
/// from `Multiple` without enumerating the full solution space.
pub fn solve<S, const N: usize>(solver: &S) -> SolveOutcome<S>
where
    S: Solver<N>,
{
    let mut found: Vec<S> = Vec::with_capacity(2);
    collect_solutions(solver, 2, &mut found);

    // Destructure the Vec via an iterator — idiomatic for "give me up to the
    // first two elements".
    let mut it = found.into_iter();
    match (it.next(), it.next()) {
        (None, _) => SolveOutcome::Unsolvable,
        (Some(s), None) => SolveOutcome::Unique(s),
        (Some(s), Some(_)) => SolveOutcome::Multiple(s),
    }
}
