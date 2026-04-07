/// Runs both solvers on a set of known puzzles and asserts they agree on
/// the solution count.

use rublock::queue_solver::QueueSolverState;
use rublock::solver::{Puzzle, SolverState};

fn check<const N: usize>(puzzle: Puzzle<N>, label: &str) {
    let old_count = SolverState::new(puzzle.clone()).count_solutions(2);
    let new_count = QueueSolverState::new(puzzle).count_solutions(2);
    assert_eq!(
        old_count, new_count,
        "{label}: old solver found {old_count} solutions, queue solver found {new_count}"
    );
    println!("{label}: both solvers agree — {old_count} solution(s)");
}

fn main() {
    // From main.rs / existing tests.
    check(
        Puzzle::new([3, 3, 5, 0, 7, 0], [5, 0, 2, 6, 5, 10]),
        "6×6 sample",
    );

    // A puzzle known to have a unique solution (from solver tests).
    check(
        Puzzle::new([3, 5, 2, 8, 5, 7], [5, 3, 8, 2, 7, 5]),
        "6×6 unique",
    );

    // A trivially unsatisfiable puzzle (targets impossible).
    // row sums and col sums can't both be 0 with non-empty digit set
    // so just use a degenerate one from the test suite.
    check(Puzzle::new([0u8; 6], [0u8; 6]), "6×6 zeros");

    println!("All checks passed.");
}
