//! Difficulty classification for user-authored puzzles.
//!
//! The Create tab calls into [`classify`] on every edit and shows a live label
//! for the current draft. Internally this is just a [`BlackSolverState`] solve
//! with the default [`SearchNodes`](crate::recorder::SearchNodes) recorder —
//! the variant comes from `solve()`, the node count from the recorder.
//!
//! The label mapping (`Normal` / `Hard` / …) lives in the web UI; this module
//! only reports the raw signals it needs.

use crate::black_solver::BlackSolverState;
use crate::grid::Grid;
use crate::recorder::Recorder;
use crate::solver::{Puzzle, SolveOutcome, Solver};

/// Solver outcome variant for a puzzle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassifyVariant {
    /// The puzzle has no solution.
    Unsolvable,
    /// The puzzle has exactly one solution.
    Unique,
    /// The puzzle has at least two solutions.
    Multiple,
}

/// Result of classifying a puzzle: variant, search-node count, propagation
/// wave count, plus a witness grid when one exists.  For `Unsolvable`, `cells`
/// is `None`.
///
/// `propagation_waves` is the finer-grained difficulty signal within the
/// propagation-only regime (`search_nodes == 1`): a puzzle whose first wave
/// already collapses several domains feels easier than one that drips out
/// deductions over a dozen waves of arc consistency.  See issue #46.
pub struct Classification<const N: usize> {
    pub variant: ClassifyVariant,
    pub search_nodes: u64,
    pub propagation_waves: u64,
    pub cells: Option<Grid<N>>,
}

/// Solve `puzzle` and return its variant, search-node count, propagation-wave
/// count, and a witness grid.
///
/// Mirrors the `summarize` helper in `src/bin/compare.rs`.
pub fn classify<const N: usize>(puzzle: Puzzle<N>) -> Classification<N> {
    let state = BlackSolverState::<N>::new(puzzle);
    let outcome = state.solve();
    let search_nodes = state.recorder().search_nodes();
    let propagation_waves = state.recorder().propagation_waves();
    let (variant, cells) = match outcome {
        SolveOutcome::Unsolvable => (ClassifyVariant::Unsolvable, None),
        SolveOutcome::Unique(s) => (ClassifyVariant::Unique, s.solved_cells()),
        SolveOutcome::Multiple(s) => (ClassifyVariant::Multiple, s.solved_cells()),
    };
    Classification {
        variant,
        search_nodes,
        propagation_waves,
        cells,
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsolvable_puzzle_classifies_as_unsolvable() {
        // Every target equal to 1: no consistent placement exists.
        let c = classify(Puzzle::<6>::new([1; 6], [1; 6]));
        assert_eq!(c.variant, ClassifyVariant::Unsolvable);
        assert!(c.cells.is_none());
    }

    #[test]
    fn unique_puzzle_classifies_as_unique_with_witness() {
        // Newspaper puzzle (see black_solver tests).
        let c = classify(Puzzle::<6>::new([8, 2, 3, 8, 9, 0], [0, 0, 5, 9, 0, 4]));
        assert_eq!(c.variant, ClassifyVariant::Unique);
        assert!(c.cells.is_some());
    }

    #[test]
    fn multiple_solutions_classify_as_multiple() {
        // Underconstrained puzzle (see black_solver tests).
        let c = classify(Puzzle::<6>::new([5, 7, 4, 0, 0, 6], [6, 0, 0, 7, 0, 6]));
        assert_eq!(c.variant, ClassifyVariant::Multiple);
        assert!(c.cells.is_some());
    }

    #[test]
    fn pure_propagation_puzzle_uses_one_search_node() {
        // From `propagation_target_9` / `black_solver_can_fully_propagate`:
        // this puzzle is solvable by propagation alone.
        let c = classify(Puzzle::<5>::new([2, 0, 0, 3, 6], [3, 0, 0, 2, 0]));
        assert_eq!(c.variant, ClassifyVariant::Unique);
        assert_eq!(
            c.search_nodes, 1,
            "propagation-only puzzle should have search_nodes == 1, got {}",
            c.search_nodes
        );
        assert!(
            c.propagation_waves >= 1,
            "propagation-only puzzle should report at least one productive wave, \
             got propagation_waves = {}",
            c.propagation_waves
        );
    }

    #[test]
    fn backtracking_puzzle_uses_more_than_one_search_node() {
        // The underconstrained puzzle from `sample_puzzle` has multiple
        // solutions, so the solver must explore more than one branch.
        let c = classify(Puzzle::<6>::new([5, 7, 4, 0, 0, 6], [6, 0, 0, 7, 0, 6]));
        assert_eq!(c.variant, ClassifyVariant::Multiple);
        assert!(
            c.search_nodes > 1,
            "puzzle should require backtracking, got search_nodes = {}",
            c.search_nodes
        );
    }
}
