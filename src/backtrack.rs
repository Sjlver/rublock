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

use crate::recorder::Recorder;
use crate::solver::{SolveOutcome, Solver};

/// Core backtracking search.
///
/// Returns the number of solutions found during this call, up to `max`.
/// If `out` is `Some`, each solved state is appended to it as it is found.
/// For an initially-empty `out`, the return value equals `out.len()` after
/// the call.
fn search<S, const N: usize>(state: &mut S, max: usize, mut out: Option<&mut Vec<S>>) -> usize
where
    S: Solver<N>,
{
    if max == 0 {
        return 0;
    }

    state.propagate();

    if state.is_contradiction() {
        return 0;
    }
    if state.is_solved() {
        if let Some(out) = out.as_deref_mut() {
            out.push(state.clone());
        }
        return 1;
    }

    let Some((row, col)) = state.pick_branching_cell() else {
        // Propagation stalled but the grid is neither solved nor contradicted.
        // This shouldn't happen — the propagation rules are supposed to be
        // complete enough to always leave a branchable cell.
        panic!("Propagation stalled");
    };

    let bit = state.pick_branching_bit(row, col);
    state.recorder().on_search_node();
    let mut branch = state.clone();
    branch.take_branch(row, col, bit);
    let left = search(&mut branch, max, out.as_deref_mut());

    state.reject_branch(row, col, bit);
    left + search(state, max - left, out)
}

/// Count the number of distinct solutions, stopping once `max` is reached.
///
/// Returns the number of solutions found, which is at most `max`.
///
/// Typical usage:
/// - `max = 1` — satisfiability test.
/// - `max = 2` — uniqueness test: `1` means unique, `2` means multiple.
pub fn count_solutions<S, const N: usize>(solver: &S, max: usize) -> usize
where
    S: Solver<N>,
{
    solver.recorder().on_search_node();
    let mut state = solver.clone();
    search(&mut state, max, None)
}

/// Solve the puzzle, reporting uniqueness.
///
/// Searches for up to two solutions so the outcome can distinguish `Unique`
/// from `Multiple` without enumerating the full solution space.
pub fn solve<S, const N: usize>(solver: &S) -> SolveOutcome<S>
where
    S: Solver<N>,
{
    solver.recorder().on_search_node();
    let mut state = solver.clone();
    let mut found: Vec<S> = Vec::with_capacity(2);
    search(&mut state, 2, Some(&mut found));

    let mut it = found.into_iter();
    match (it.next(), it.next()) {
        (None, _) => SolveOutcome::Unsolvable,
        (Some(s), None) => SolveOutcome::Unique(s),
        (Some(s), Some(_)) => SolveOutcome::Multiple(s),
    }
}
