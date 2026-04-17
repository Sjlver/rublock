// Generates a single random puzzle with a unique solution, optionally
// filtered by difficulty (minimum number of search-tree nodes the solver
// needs to visit).
//
// Usage:
//   cargo run --release --bin gen_puzzle -- --size=6 --min-nodes=50
//
// Strategy: a pool of worker threads independently fill empty grids with a
// randomised DFS, derive the row/col targets, and run
// `QueueSolverState::solve`.  The first thread to find a unique-solution
// puzzle whose solve visits at least `--min-nodes` nodes wins; the others
// stop on the next iteration.
//
// ## Why send only `(row_t, col_t)` across threads?
//
// `QueueSolverState` holds an `Rc<Cell<Stats>>`, which is `!Send`.  Rather
// than reworking the stats plumbing to be `Send + Sync`, we ship the puzzle
// targets (which are `Copy`) through an `mpsc::channel` and re-solve once on
// the main thread for display.  One extra solve is cheap; isolating the
// threading concern to this binary keeps the rest of the codebase
// single-threaded.
//
// ## Coordination
//
// - `grids` and `best_nodes` are `AtomicU64` shared via stack borrow
//   (thanks to `thread::scope`, no `Arc` is needed).  Workers update them
//   with `fetch_add` and `fetch_max`; the main thread reads them ~10× a
//   second to refresh the spinner.
// - `done: AtomicBool` is the stop flag; workers check it at the top of each
//   loop iteration.
// - `mpsc::channel` carries the winning targets to the main thread.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use indicatif::{ProgressBar, ProgressStyle};
use rand::seq::SliceRandom;
use rublock::grid::{Cell, Grid};
use rublock::queue_solver::QueueSolverState;
use rublock::solver::{Puzzle, SolveOutcome};

fn usage() -> ! {
    eprintln!("Usage: gen_puzzle [--size=N] [--min-nodes=K] [--threads=T]");
    eprintln!("  --size       grid side length, 3–11 (default: 6)");
    eprintln!("  --min-nodes  minimum search-tree nodes the solver must visit (default: 0)");
    eprintln!("  --threads    worker threads (default: available parallelism)");
    std::process::exit(1);
}

fn parse_args() -> (usize, u64, usize) {
    let mut size = 6usize;
    let mut min_nodes = 0u64;
    let mut threads = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);

    for arg in std::env::args().skip(1) {
        if let Some(val) = arg.strip_prefix("--size=") {
            size = val.parse().unwrap_or_else(|_| usage());
        } else if let Some(val) = arg.strip_prefix("--min-nodes=") {
            min_nodes = val.parse().unwrap_or_else(|_| usage());
        } else if let Some(val) = arg.strip_prefix("--threads=") {
            threads = val.parse().unwrap_or_else(|_| usage());
        } else {
            usage();
        }
    }

    if !(3..=11).contains(&size) {
        eprintln!("--size must be between 3 and 11");
        std::process::exit(1);
    }
    if threads == 0 {
        eprintln!("--threads must be at least 1");
        std::process::exit(1);
    }

    (size, min_nodes, threads)
}

fn main() {
    let (size, min_nodes, threads) = parse_args();
    match size {
        3 => run::<3>(min_nodes, threads),
        4 => run::<4>(min_nodes, threads),
        5 => run::<5>(min_nodes, threads),
        6 => run::<6>(min_nodes, threads),
        7 => run::<7>(min_nodes, threads),
        8 => run::<8>(min_nodes, threads),
        9 => run::<9>(min_nodes, threads),
        10 => run::<10>(min_nodes, threads),
        11 => run::<11>(min_nodes, threads),
        _ => unreachable!(), // validated in parse_args
    }
}

fn run<const N: usize>(min_nodes: u64, num_threads: usize) {
    let grids = AtomicU64::new(0);
    let best_nodes = AtomicU64::new(0);
    let done = AtomicBool::new(false);

    let pb = ProgressBar::new_spinner();
    // `{pos}` is indicatif's built-in counter (updated via `set_position`),
    // and `{msg}` is the free-form string we update with the best node
    // count.  Thread count is constant for this run, so we bake it straight
    // into the template rather than reaching for `{prefix}`.
    let template = format!(
        "{{spinner}} tried {{pos}} grids on {num_threads} threads, best so far: {{msg}} nodes"
    );
    pb.set_style(ProgressStyle::with_template(&template).unwrap());
    pb.set_message("0");
    pb.enable_steady_tick(Duration::from_millis(100));

    let (tx, rx) = mpsc::channel::<([u8; N], [u8; N], u64)>();

    // ── Race the workers, drive the spinner from the main thread ──────────────
    //
    // `thread::scope` lets the workers borrow the atomics directly off our
    // stack — no `Arc`, no static lifetime gymnastics.  The closure can't
    // return until every spawned thread has joined, so any borrows the
    // workers hold are guaranteed valid for the whole scope.
    let winner: Option<([u8; N], [u8; N], u64)> = thread::scope(|s| {
        for _ in 0..num_threads {
            let tx = tx.clone();
            // Borrow shared state by reference; lifetimes are tied to `s`.
            let grids = &grids;
            let best_nodes = &best_nodes;
            let done = &done;
            s.spawn(move || worker::<N>(min_nodes, grids, best_nodes, done, tx));
        }
        // Drop our copy of the sender so the channel becomes `Disconnected`
        // once every worker exits — the recv loop below uses that as a
        // fallback termination condition.
        drop(tx);

        loop {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(w) => {
                    // Stop the rest of the pool.  The winner already set
                    // `done` before sending; this is belt-and-braces.
                    done.store(true, Ordering::Relaxed);
                    return Some(w);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Refresh the spinner with the latest counters.  These
                    // loads are racy (workers may be mid-update), which is
                    // fine for a UI counter.
                    pb.set_position(grids.load(Ordering::Relaxed));
                    pb.set_message(best_nodes.load(Ordering::Relaxed).to_string());
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return None,
            }
        }
    });

    pb.finish_and_clear();

    if let Some((row_t, col_t, nodes)) = winner {
        let total_grids = grids.load(Ordering::Relaxed);
        // Re-solve on the main thread to obtain a printable state.  See the
        // module-level note on why we don't ship `QueueSolverState` across
        // threads.
        let solved = match QueueSolverState::new(Puzzle::new(row_t, col_t)).solve() {
            SolveOutcome::Unique(s) => s,
            _ => unreachable!("worker just generated this puzzle as Unique"),
        };
        report::<N>(&row_t, &col_t, &solved, nodes, total_grids);
    }
}

/// One worker iteration: random DFS-fill, derive targets, solve, race to
/// publish the result.  Exits when `done` is observed `true`.
fn worker<const N: usize>(
    min_nodes: u64,
    grids: &AtomicU64,
    best_nodes: &AtomicU64,
    done: &AtomicBool,
    tx: mpsc::Sender<([u8; N], [u8; N], u64)>,
) {
    let mut rng = rand::rng();

    while !done.load(Ordering::Relaxed) {
        let mut cells = [[Cell::Empty; N]; N];
        let Some(grid) = dfs::<N>(&mut cells, 0, &mut rng) else {
            continue;
        };

        grids.fetch_add(1, Ordering::Relaxed);

        let (row_t, col_t) = grid.compute_targets();
        let puzzle = Puzzle::new(row_t, col_t);
        let state = QueueSolverState::new(puzzle);

        if let SolveOutcome::Unique(solved) = state.solve() {
            let nodes = solved.stats().search_nodes;
            // Monotonic max — `fetch_max` handles the racing CAS for us.
            best_nodes.fetch_max(nodes, Ordering::Relaxed);
            if nodes >= min_nodes {
                // Set the stop flag *before* sending so other workers see it
                // as soon as possible; the receiver also sets it on receipt.
                done.store(true, Ordering::Relaxed);
                // The receiver may have already exited (e.g. another worker
                // raced us); ignore a closed channel.
                let _ = tx.send((row_t, col_t, nodes));
                return;
            }
        }
    }
}

fn report<const N: usize>(
    row_t: &[u8; N],
    col_t: &[u8; N],
    solved: &QueueSolverState<N>,
    nodes: u64,
    grids: u64,
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
    println!("search nodes: {nodes}  (after {grids} grids)");
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
