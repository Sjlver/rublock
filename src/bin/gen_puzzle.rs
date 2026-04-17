// Generates a single random puzzle with a unique solution, optionally
// filtered by difficulty (minimum number of search-tree nodes the solver
// needs to visit).
//
// Usage:
//   cargo run --release --bin gen_puzzle -- --size=6 --min-nodes=50
//
// Strategy: repeatedly fill an empty grid with a randomised DFS, derive the
// row/col targets from the resulting grid, then run `QueueSolverState::solve`
// on those targets.  Keep the puzzle if it has a unique solution and the
// solve visited at least `--min-nodes` search-tree nodes.

use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};
use rand::seq::SliceRandom;
use rublock::grid::{Cell, Grid};
use rublock::queue_solver::QueueSolverState;
use rublock::solver::{Puzzle, SolveOutcome};

fn usage() -> ! {
    eprintln!("Usage: gen_puzzle [--size=N] [--min-nodes=K]");
    eprintln!("  --size       grid side length, 3–11 (default: 6)");
    eprintln!("  --min-nodes  minimum search-tree nodes the solver must visit (default: 0)");
    std::process::exit(1);
}

fn parse_args() -> (usize, u64) {
    let mut size = 6usize;
    let mut min_nodes = 0u64;

    for arg in std::env::args().skip(1) {
        if let Some(val) = arg.strip_prefix("--size=") {
            size = val.parse().unwrap_or_else(|_| usage());
        } else if let Some(val) = arg.strip_prefix("--min-nodes=") {
            min_nodes = val.parse().unwrap_or_else(|_| usage());
        } else {
            usage();
        }
    }

    if !(3..=11).contains(&size) {
        eprintln!("--size must be between 3 and 11");
        std::process::exit(1);
    }

    (size, min_nodes)
}

fn main() {
    let (size, min_nodes) = parse_args();
    match size {
        3 => run::<3>(min_nodes),
        4 => run::<4>(min_nodes),
        5 => run::<5>(min_nodes),
        6 => run::<6>(min_nodes),
        7 => run::<7>(min_nodes),
        8 => run::<8>(min_nodes),
        9 => run::<9>(min_nodes),
        10 => run::<10>(min_nodes),
        11 => run::<11>(min_nodes),
        _ => unreachable!(), // validated in parse_args
    }
}

fn run<const N: usize>(min_nodes: u64) {
    let mut rng = rand::rng();

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::with_template("{spinner} tried {pos} grids, best so far: {msg} nodes")
            .unwrap(),
    );
    pb.enable_steady_tick(Duration::from_millis(100));

    let mut attempts: u64 = 0;
    let mut best_nodes: u64 = 0;
    pb.set_message("0");

    loop {
        attempts += 1;

        let mut cells = [[Cell::Empty; N]; N];
        let Some(grid) = dfs::<N>(&mut cells, 0, &mut rng) else {
            // Dead end: try another random fill.
            pb.set_position(attempts);
            continue;
        };

        let (row_t, col_t) = grid.compute_targets();
        let puzzle = Puzzle::new(row_t, col_t);
        let state = QueueSolverState::new(puzzle);
        let outcome = state.solve();

        if let SolveOutcome::Unique(solved) = outcome {
            let nodes = solved.stats().search_nodes;
            if nodes > best_nodes {
                best_nodes = nodes;
                pb.set_message(best_nodes.to_string());
            }
            if nodes >= min_nodes {
                pb.finish_and_clear();
                report::<N>(&row_t, &col_t, &solved, nodes, attempts);
                return;
            }
        }

        pb.set_position(attempts);
    }
}

fn report<const N: usize>(
    row_t: &[u8; N],
    col_t: &[u8; N],
    solved: &QueueSolverState<N>,
    nodes: u64,
    attempts: u64,
) {
    // Targets line: row targets followed by column targets, ready to pipe
    // into `cargo run -- <targets>`.
    let mut nums: Vec<String> = Vec::with_capacity(2 * N);
    nums.extend(row_t.iter().map(|n| n.to_string()));
    nums.extend(col_t.iter().map(|n| n.to_string()));
    println!("{}", nums.join(" "));
    println!();
    print!("{solved}");
    println!();
    println!("search nodes: {nodes}  (after {attempts} attempts)");
}

// Attempt to fill `cells` from `pos` onward, trying candidates in a random
// order at each position. Returns `Some(Grid)` if a complete grid was reached,
// or `None` if every candidate at some position was exhausted (dead end).
fn dfs<const N: usize>(
    cells: &mut [[Cell; N]; N],
    pos: usize,
    rng: &mut impl rand::Rng,
) -> Option<Grid<N>> {
    if pos == N * N {
        return Some(Grid { cells: *cells });
    }

    let row = pos / N;
    let col = pos % N;

    let row_blacks = (0..col).filter(|&c| cells[row][c] == Cell::Black).count();
    let col_blacks = (0..row).filter(|&r| cells[r][col] == Cell::Black).count();
    let row_digit_mask: u64 = (0..col)
        .filter_map(|c| {
            if let Cell::Number(n) = cells[row][c] {
                Some(1u64 << n)
            } else {
                None
            }
        })
        .fold(0, |a, b| a | b);
    let col_digit_mask: u64 = (0..row)
        .filter_map(|r| {
            if let Cell::Number(n) = cells[r][col] {
                Some(1u64 << n)
            } else {
                None
            }
        })
        .fold(0, |a, b| a | b);

    let digits: u8 = (N - 2) as u8;
    let mut candidates: Vec<Cell> = std::iter::once(Cell::Black)
        .chain((1..=digits).map(Cell::Number))
        .filter(|&c| match c {
            Cell::Black => row_blacks < 2 && col_blacks < 2,
            Cell::Number(d) => {
                let bit = 1u64 << d;
                row_digit_mask & bit == 0 && col_digit_mask & bit == 0
            }
            Cell::Empty => unreachable!(),
        })
        .collect();

    candidates.shuffle(rng);

    for candidate in candidates {
        cells[row][col] = candidate;
        if let Some(grid) = dfs(cells, pos + 1, rng) {
            return Some(grid);
        }
    }

    cells[row][col] = Cell::Empty;
    None
}
