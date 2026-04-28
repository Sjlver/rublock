/// Differential test of the three solver implementations.
///
/// Loops forever, generating random puzzles via `random_grid`'s DFS and
/// solving each one with `BasicSolverState`, `QueueSolverState`, and
/// `BlackSolverState`.  Exits non-zero on any disagreement about
/// outcome, solved cells, or whether backtracking was required.
use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};
use rublock::basic_solver::BasicSolverState;
use rublock::black_solver::BlackSolverState;
use rublock::grid::random_grid;
use rublock::queue_solver::QueueSolverState;
use rublock::solver::{Puzzle, SolveOutcome, Solver};

fn usage() -> ! {
    eprintln!("Usage: compare [--size=N]   (N in 3..=11, default 6)");
    std::process::exit(1);
}

fn parse_size() -> usize {
    let mut size = 6;
    for arg in std::env::args().skip(1) {
        if let Some(v) = arg.strip_prefix("--size=") {
            size = v.parse().unwrap_or_else(|_| usage());
        } else {
            usage();
        }
    }
    if !(3..=11).contains(&size) {
        usage();
    }
    size
}

#[derive(Debug, PartialEq, Eq)]
enum Variant {
    Unsolvable,
    Unique,
    Multiple,
}

struct SolveSummary<const N: usize> {
    variant: Variant,
    cells: Option<[[i8; N]; N]>,
    search_nodes: u64,
}

fn summarize<S: Solver<N>, const N: usize>(state: &S) -> SolveSummary<N> {
    let outcome = state.solve();
    let (variant, cells) = match outcome {
        SolveOutcome::Unsolvable => (Variant::Unsolvable, None),
        SolveOutcome::Unique(s) => (Variant::Unique, s.solved_cells()),
        SolveOutcome::Multiple(s) => (Variant::Multiple, s.solved_cells()),
    };
    SolveSummary {
        variant,
        cells,
        search_nodes: state.stats().search_nodes,
    }
}

fn print_targets<const N: usize>(puzzle: &Puzzle<N>) {
    let mut nums: Vec<String> = Vec::with_capacity(2 * N);
    nums.extend(puzzle.row_targets.iter().map(|n| n.to_string()));
    nums.extend(puzzle.col_targets.iter().map(|n| n.to_string()));
    println!("targets: {}", nums.join(" "));
    println!(
        "link:  https://dev.purpureus.net/rublock?p={}",
        nums.join(",")
    )
}

fn print_summary<const N: usize>(label: &str, s: &SolveSummary<N>) {
    println!(
        "  {label}: variant={:?} search_nodes={}",
        s.variant, s.search_nodes
    );
    if let Some(cells) = &s.cells {
        for row in cells {
            print!("    ");
            for &v in row {
                if v < 0 {
                    print!(" #");
                } else {
                    print!(" {v}");
                }
            }
            println!();
        }
    }
}

fn report_mismatch<const N: usize>(
    pb: &ProgressBar,
    reason: &str,
    puzzle: &Puzzle<N>,
    basic: &SolveSummary<N>,
    queue: &SolveSummary<N>,
    black: &SolveSummary<N>,
) -> ! {
    pb.finish();
    println!("MISMATCH: {reason}");
    print_targets(puzzle);
    print_summary("basic", basic);
    print_summary("queue", queue);
    print_summary("black", black);
    std::process::exit(1);
}

fn run<const N: usize>() -> ! {
    let mut rng = rand::rng();
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template(&format!("{{spinner}} N={N}: {{pos}} puzzles compared"))
            .unwrap(),
    );
    pb.enable_steady_tick(Duration::from_millis(100));

    loop {
        let grid = random_grid::<N>(&mut rng);
        let (rows, cols) = grid.compute_targets();
        let puzzle = Puzzle::new(rows, cols);

        let basic = summarize(&BasicSolverState::<N>::new(puzzle.clone()));
        let queue = summarize(&QueueSolverState::<N>::new(puzzle.clone()));
        let black = summarize(&BlackSolverState::<N>::new(puzzle.clone()));

        if basic.variant != queue.variant || basic.variant != black.variant {
            report_mismatch(
                &pb,
                "outcome variants disagree",
                &puzzle,
                &basic,
                &queue,
                &black,
            );
        }

        if basic.variant == Variant::Unique {
            if basic.cells != queue.cells || basic.cells != black.cells {
                report_mismatch(
                    &pb,
                    "unique solution cells disagree",
                    &puzzle,
                    &basic,
                    &queue,
                    &black,
                );
            }
        }

        let basic_branched = basic.search_nodes > 1;
        let queue_branched = queue.search_nodes > 1;
        let black_branched = black.search_nodes > 1;
        if basic_branched != queue_branched || basic_branched != black_branched {
            report_mismatch(
                &pb,
                "propagation-only vs branched disagree",
                &puzzle,
                &basic,
                &queue,
                &black,
            );
        }

        pb.inc(1);
    }
}

fn main() {
    match parse_size() {
        3 => run::<3>(),
        4 => run::<4>(),
        5 => run::<5>(),
        6 => run::<6>(),
        7 => run::<7>(),
        8 => run::<8>(),
        9 => run::<9>(),
        10 => run::<10>(),
        11 => run::<11>(),
        _ => unreachable!(),
    }
}
