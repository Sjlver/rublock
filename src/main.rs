use rublock::solver::{Puzzle, SolverState};

fn main() {
    tracing_subscriber::fmt::init();
    let puzzle = Puzzle::new([3, 3, 5, 0, 7, 0], [5, 0, 2, 6, 5, 10]);
    let mut state = SolverState::new(puzzle);
    state.propagate();
    println!("{}", state);
}
