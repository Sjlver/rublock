use rublock::enumerate::SolverChoice;
use rublock::queue_solver::QueueSolverState;
use rublock::solver::{Puzzle, SolveOutcome, SolverState};
use rublock::stats::Stats;

fn usage() -> ! {
    eprintln!("Usage: rublock [--solver=basic|queue] <2N numbers>");
    eprintln!("  The 2N numbers are the row targets followed by the column targets.");
    eprintln!("  N must be between 3 and 11.");
    std::process::exit(1);
}

fn parse_args() -> (SolverChoice, Vec<u8>) {
    let mut solver = SolverChoice::Queue;
    let mut nums: Vec<u8> = Vec::new();

    for arg in std::env::args().skip(1) {
        if let Some(val) = arg.strip_prefix("--solver=") {
            solver = match val {
                "basic" => SolverChoice::Basic,
                "queue" => SolverChoice::Queue,
                _ => usage(),
            };
        } else {
            match arg.parse::<u8>() {
                Ok(n) => nums.push(n),
                Err(_) => usage(),
            }
        }
    }

    let count = nums.len();
    if count < 6 || count > 22 || count % 2 != 0 {
        eprintln!("Expected an even number of targets between 6 and 22, got {count}.");
        std::process::exit(1);
    }

    (solver, nums)
}

/// Print the solve result followed by a one-line status and the stats
/// collected during the search.  Kept generic over the solver state so the
/// two backends share the same reporting path.
fn report<S: std::fmt::Display>(outcome: SolveOutcome<S>, stats: Stats) {
    match outcome {
        SolveOutcome::Unsolvable => {
            println!("no solution");
        }
        SolveOutcome::Unique(state) => {
            println!("{state}");
            println!("unique solution");
        }
        SolveOutcome::Multiple(state) => {
            println!("{state}");
            println!("multiple solutions (showing one)");
        }
    }
    println!();
    println!("{stats}");
}

fn run<const N: usize>(nums: &[u8], solver: SolverChoice) {
    let row_t: [u8; N] = nums[..N].try_into().unwrap();
    let col_t: [u8; N] = nums[N..].try_into().unwrap();
    let puzzle = Puzzle::new(row_t, col_t);
    match solver {
        SolverChoice::Basic => {
            let state = SolverState::new(puzzle);
            let outcome = state.solve();
            report(outcome, state.stats());
        }
        SolverChoice::Queue => {
            let state = QueueSolverState::new(puzzle);
            let outcome = state.solve();
            report(outcome, state.stats());
        }
    }
}

fn main() {
    tracing_subscriber::fmt::init();
    let (solver, nums) = parse_args();
    let n = nums.len() / 2;
    match n {
        3 => run::<3>(&nums, solver),
        4 => run::<4>(&nums, solver),
        5 => run::<5>(&nums, solver),
        6 => run::<6>(&nums, solver),
        7 => run::<7>(&nums, solver),
        8 => run::<8>(&nums, solver),
        9 => run::<9>(&nums, solver),
        10 => run::<10>(&nums, solver),
        11 => run::<11>(&nums, solver),
        _ => unreachable!(), // validated in parse_args
    }
}
