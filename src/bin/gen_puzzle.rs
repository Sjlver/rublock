// Generates a single random puzzle with a unique solution, optionally
// filtered by difficulty — the number of search-tree nodes the solver needs
// to visit must fall in `[--min-nodes, --max-nodes]` (both inclusive).
//
// Usage:
//   cargo run --release --bin gen_puzzle -- --size=6 --min-nodes=50 --max-nodes=500
//
// Strategy: a pool of worker threads independently fill empty grids with a
// randomised DFS, derive the row/col targets, and run `Solver::solve`.  The
// first thread to find a unique-solution puzzle whose solve visits a node
// count inside the window wins; the others stop on the next iteration.
//
// ## Why send only `(row_targets, col_targets)` across threads?
//
// Both solver states hold an `Rc<Cell<Stats>>` (via `StatsHandle`), which is
// `!Send`.  Rather than reworking the stats plumbing to be `Send + Sync`, we
// ship the puzzle targets (which are `Copy`) through an `mpsc::channel` and
// re-solve once on the main thread for display.  One extra solve is cheap;
// isolating the threading concern to this binary keeps the rest of the
// codebase single-threaded.
//
// ## Coordination
//
// - Shared counters live in [`SharedState`]: five atomics borrowed off the
//   main thread's stack through `thread::scope`, so no `Arc` is needed.
//   Workers update them with `fetch_add` / `fetch_min` / `fetch_max`; the
//   main thread reads them ~10× a second to refresh the spinner.
// - `done: AtomicBool` is the stop flag; workers check it at the top of each
//   loop iteration.
// - `mpsc::channel` carries the winning targets to the main thread.
//
// ## Solver dispatch
//
// Both [`run`] and [`worker`] are generic over `S: Solver<N>`: the choice
// between [`BasicSolverState`] and [`QueueSolverState`] is made exactly once
// in [`dispatch_solver`] (called from `main`) and monomorphised from there.
// Previously this decision was repeated as a two-arm match in three separate
// hot-loop bodies.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use indicatif::{ProgressBar, ProgressStyle};
use rand::seq::SliceRandom;
use rublock::basic_solver::BasicSolverState;
use rublock::enumerate::SolverChoice;
use rublock::grid::{Cell, Grid};
use rublock::queue_solver::QueueSolverState;
use rublock::solver::{Puzzle, SolveOutcome, Solver};

// ── Arguments ─────────────────────────────────────────────────────────────────

/// Parsed command-line arguments.  Kept as a single struct so the hot
/// functions (`run`, `worker`) don't have to thread five positional
/// parameters through their signatures.
#[derive(Clone, Copy)]
struct Args {
    size: usize,
    min_nodes: u64,
    max_nodes: u64,
    threads: usize,
    solver: SolverChoice,
}

fn usage() -> ! {
    eprintln!(
        "Usage: gen_puzzle [--size=N] [--min-nodes=K] [--max-nodes=K] [--threads=T] [--solver=basic|queue]"
    );
    eprintln!("  --size       grid side length, 3–11 (default: 6)");
    eprintln!(
        "  --min-nodes  minimum search-tree nodes the solver must visit (inclusive, default: 0)"
    );
    eprintln!(
        "  --max-nodes  maximum search-tree nodes the solver must visit (inclusive, default: unbounded)"
    );
    eprintln!("  --threads    worker threads (default: available parallelism)");
    eprintln!("  --solver     solver implementation to use (default: queue)");
    std::process::exit(1);
}

fn parse_args() -> Args {
    let mut args = Args {
        size: 6,
        min_nodes: 0,
        max_nodes: u64::MAX,
        threads: thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
        solver: SolverChoice::Queue,
    };

    for arg in std::env::args().skip(1) {
        if let Some(val) = arg.strip_prefix("--size=") {
            args.size = val.parse().unwrap_or_else(|_| usage());
        } else if let Some(val) = arg.strip_prefix("--min-nodes=") {
            args.min_nodes = val.parse().unwrap_or_else(|_| usage());
        } else if let Some(val) = arg.strip_prefix("--max-nodes=") {
            args.max_nodes = val.parse().unwrap_or_else(|_| usage());
        } else if let Some(val) = arg.strip_prefix("--threads=") {
            args.threads = val.parse().unwrap_or_else(|_| usage());
        } else if let Some(val) = arg.strip_prefix("--solver=") {
            args.solver = match val {
                "basic" => SolverChoice::Basic,
                "queue" => SolverChoice::Queue,
                _ => usage(),
            };
        } else {
            usage();
        }
    }

    if !(3..=11).contains(&args.size) {
        eprintln!("--size must be between 3 and 11");
        std::process::exit(1);
    }
    if args.threads == 0 {
        eprintln!("--threads must be at least 1");
        std::process::exit(1);
    }
    if args.max_nodes < args.min_nodes {
        eprintln!(
            "--max-nodes ({}) must be >= --min-nodes ({})",
            args.max_nodes, args.min_nodes
        );
        std::process::exit(1);
    }

    args
}

// ── Shared state ──────────────────────────────────────────────────────────────

/// Atomics shared between the worker pool and the UI thread.
///
/// All five live on the main thread's stack and are borrowed by the workers
/// via `thread::scope` — no `Arc`, no static lifetime.  `min_nodes_seen`
/// starts at `u64::MAX` so that `fetch_min` lowers it on first observation;
/// `max_nodes_seen` starts at `0` so `fetch_max` raises it.
struct SharedState {
    grids: AtomicU64,
    valid_puzzles: AtomicU64,
    min_nodes_seen: AtomicU64,
    max_nodes_seen: AtomicU64,
    done: AtomicBool,
}

impl SharedState {
    fn new() -> Self {
        Self {
            grids: AtomicU64::new(0),
            valid_puzzles: AtomicU64::new(0),
            min_nodes_seen: AtomicU64::new(u64::MAX),
            max_nodes_seen: AtomicU64::new(0),
            done: AtomicBool::new(false),
        }
    }

    /// Render the spinner's status message from the current counter values.
    fn status_message(&self) -> String {
        let valid_puzzles = self.valid_puzzles.load(Ordering::Relaxed);
        let min = self.min_nodes_seen.load(Ordering::Relaxed);
        let max = self.max_nodes_seen.load(Ordering::Relaxed);
        // Sentinel: no unique-solution puzzle has been observed yet.
        let range = if min == u64::MAX && max == 0 {
            "—".to_string()
        } else {
            format!("{min}..={max}")
        };
        format!("{valid_puzzles} valid puzzles, nodes seen: {range}")
    }
}

// ── Entry points ──────────────────────────────────────────────────────────────

fn main() {
    let args = parse_args();
    match args.size {
        3 => dispatch_solver::<3>(args),
        4 => dispatch_solver::<4>(args),
        5 => dispatch_solver::<5>(args),
        6 => dispatch_solver::<6>(args),
        7 => dispatch_solver::<7>(args),
        8 => dispatch_solver::<8>(args),
        9 => dispatch_solver::<9>(args),
        10 => dispatch_solver::<10>(args),
        11 => dispatch_solver::<11>(args),
        _ => unreachable!(), // validated in parse_args
    }
}

/// Monomorphise on the solver choice once, so the hot path inside `run` and
/// `worker` is already specialised to a concrete `Solver` impl.
fn dispatch_solver<const N: usize>(args: Args) {
    match args.solver {
        SolverChoice::Basic => run::<N, BasicSolverState<N>>(args),
        SolverChoice::Queue => run::<N, QueueSolverState<N>>(args),
    }
}

// ── Run loop ──────────────────────────────────────────────────────────────────

fn run<const N: usize, S: Solver<N>>(args: Args) {
    // Fast path: `--max-nodes=1` means "solvable by propagation alone".  We
    // skip the full backtracking `solve()` — propagation reaching a solved
    // state implies a unique solution — and, since every match is valid by
    // construction, the "valid puzzles" / "nodes seen" counters carry no
    // information.  Announce the mode and drop those fields from the UI.
    let fast_path = args.max_nodes == 1;
    if fast_path {
        println!(
            "max-nodes=1: propagation-only fast path (no backtracking search; \
             valid-puzzle counts omitted)."
        );
    }

    let shared = SharedState::new();
    let start = Instant::now();

    let pb = ProgressBar::new_spinner();
    // `{pos}` is indicatif's built-in counter (updated via `set_position`),
    // and `{msg}` is the free-form string we update with the valid-puzzle
    // count and observed node range.  Thread count is constant for this
    // run, so we bake it straight into the template rather than reaching
    // for `{prefix}`.  The fast path has nothing meaningful for `{msg}`.
    let template = if fast_path {
        format!("{{spinner}} tried {{pos}} grids on {} threads", args.threads)
    } else {
        format!(
            "{{spinner}} tried {{pos}} grids on {} threads, {{msg}}",
            args.threads
        )
    };
    pb.set_style(ProgressStyle::with_template(&template).unwrap());
    if !fast_path {
        pb.set_message(shared.status_message());
    }
    pb.enable_steady_tick(Duration::from_millis(100));

    let (tx, rx) = mpsc::channel::<([u8; N], [u8; N], u64)>();

    // ── Race the workers, drive the spinner from the main thread ──────────────
    //
    // `thread::scope` lets the workers borrow `shared` directly off our
    // stack — no `Arc`, no static lifetime gymnastics.  The closure can't
    // return until every spawned thread has joined, so any borrows the
    // workers hold are guaranteed valid for the whole scope.
    let winner: Option<([u8; N], [u8; N], u64)> = thread::scope(|s| {
        for _ in 0..args.threads {
            let tx = tx.clone();
            let shared = &shared;
            s.spawn(move || worker::<N, S>(args, shared, tx));
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
                    shared.done.store(true, Ordering::Relaxed);
                    return Some(w);
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // Refresh the spinner with the latest counters.  These
                    // loads are racy (workers may be mid-update), which is
                    // fine for a UI counter.
                    pb.set_position(shared.grids.load(Ordering::Relaxed));
                    if !fast_path {
                        pb.set_message(shared.status_message());
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => return None,
            }
        }
    });

    pb.finish_and_clear();

    if let Some((row_targets, col_targets, nodes)) = winner {
        let total_grids = shared.grids.load(Ordering::Relaxed);
        let total_valid = shared.valid_puzzles.load(Ordering::Relaxed);
        let elapsed = start.elapsed();
        // Re-solve on the main thread to obtain a printable state.  See the
        // module-level note on why we don't ship the solver state across
        // threads.
        let puzzle = Puzzle::new(row_targets, col_targets);
        let solved = match S::new(puzzle).solve() {
            SolveOutcome::Unique(s) => s,
            _ => unreachable!("worker just generated this puzzle as Unique"),
        };
        // Slow-path stats (node count, total valid puzzles).  `None` signals
        // the fast path, where neither value is meaningful.
        let slow_stats = (!fast_path).then_some((nodes, total_valid));
        report::<N, S>(
            &row_targets,
            &col_targets,
            &solved,
            total_grids,
            elapsed,
            slow_stats,
        );
    }
}

/// One worker iteration: random DFS-fill, derive targets, solve, race to
/// publish the result.  Exits when `shared.done` is observed `true`.
fn worker<const N: usize, S: Solver<N>>(
    args: Args,
    shared: &SharedState,
    tx: mpsc::Sender<([u8; N], [u8; N], u64)>,
) {
    let mut rng = rand::rng();

    while !shared.done.load(Ordering::Relaxed) {
        let mut cells = [[Cell::Empty; N]; N];
        let Some(grid) = dfs::<N>(&mut cells, 0, &mut rng) else {
            continue;
        };

        shared.grids.fetch_add(1, Ordering::Relaxed);

        let (row_targets, col_targets) = grid.compute_targets();
        let puzzle = Puzzle::new(row_targets, col_targets);

        if args.max_nodes == 1 {
            // Propagation-only fast path.  If propagation alone reaches a
            // solved state, the solution is trivially unique (no branching
            // was needed), so we skip the uniqueness-checking `solve()`
            // call entirely.  `valid_puzzles` and the node-range atomics
            // carry no information in this mode and are left untouched.
            //
            // `S::new` may or may not propagate internally (the queue solver
            // does, the basic solver doesn't); calling `propagate` a second
            // time is a no-op when the queue is empty, so this is safe for
            // either implementation.
            let mut st = S::new(puzzle);
            st.propagate();
            if st.is_solved() {
                shared.done.store(true, Ordering::Relaxed);
                let _ = tx.send((row_targets, col_targets, 1));
                return;
            }
            continue;
        }

        let unique_nodes = match S::new(puzzle).solve() {
            SolveOutcome::Unique(solved) => Some(solved.stats().search_nodes),
            _ => None,
        };

        if let Some(nodes) = unique_nodes {
            shared.valid_puzzles.fetch_add(1, Ordering::Relaxed);
            // Track the full observed range of node counts, regardless of
            // whether this puzzle qualifies — the spinner uses these to
            // show how far we are from the requested window.
            shared.min_nodes_seen.fetch_min(nodes, Ordering::Relaxed);
            shared.max_nodes_seen.fetch_max(nodes, Ordering::Relaxed);
            if (args.min_nodes..=args.max_nodes).contains(&nodes) {
                // Set the stop flag *before* sending so other workers see it
                // as soon as possible; the receiver also sets it on receipt.
                shared.done.store(true, Ordering::Relaxed);
                // The receiver may have already exited (e.g. another worker
                // raced us); ignore a closed channel.
                let _ = tx.send((row_targets, col_targets, nodes));
                return;
            }
        }
    }
}

/// Print the winning puzzle, its solved state, and run statistics.
///
/// `slow_stats = Some((nodes, valid_puzzles))` on the full solve path;
/// `None` on the `--max-nodes=1` fast path, where node counts and the
/// "valid puzzles" running total carry no information.
fn report<const N: usize, S: Solver<N>>(
    row_targets: &[u8; N],
    col_targets: &[u8; N],
    solved: &S,
    grids: u64,
    elapsed: Duration,
    slow_stats: Option<(u64, u64)>,
) {
    // Targets line: row targets followed by column targets, ready to pipe
    // into `cargo run -- <targets>`.
    let mut nums: Vec<String> = Vec::with_capacity(2 * N);
    nums.extend(row_targets.iter().map(|n| n.to_string()));
    nums.extend(col_targets.iter().map(|n| n.to_string()));
    println!("{}", nums.join(" "));
    println!();
    print!("{solved}");
    println!();
    if let Some((nodes, valid_puzzles)) = slow_stats {
        println!("search nodes: {nodes}");
        println!(
            "Searched {valid_puzzles} valid puzzles in {grids} grids in {:.1} seconds.",
            elapsed.as_secs_f64()
        );
    } else {
        println!(
            "Searched {grids} grids in {:.1} seconds.",
            elapsed.as_secs_f64()
        );
    }
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
