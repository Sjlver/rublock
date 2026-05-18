// Difficulty calibration: collect a sample of puzzles whose backtracking
// node count lies in `[--min-nodes, --max-nodes]`, then print a histogram
// and percentile summary of their productive propagation-wave counts.
//
// Usage:
//   cargo run --release --bin calibrate_difficulty -- \
//     --size=6 --max-nodes=1 --count=1000
//
// Threading and the `--max-nodes=1` fast path mirror `gen_puzzle`: workers
// race to fill random grids and solve them (or just propagate, when no
// backtracking is permitted), and ship matching samples back through an
// `mpsc::channel`.  Unlike `gen_puzzle`, the main thread keeps the channel
// open and accumulates samples until `--count` of them have arrived.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use indicatif::{ProgressBar, ProgressStyle};
use rublock::basic_solver::BasicSolverState;
use rublock::black_solver::BlackSolverState;
use rublock::enumerate::SolverChoice;
use rublock::grid::random_grid;
use rublock::queue_solver::QueueSolverState;
use rublock::recorder::Recorder;
use rublock::solver::{Puzzle, SolveOutcome, Solver};

// ── Arguments ─────────────────────────────────────────────────────────────────

#[derive(Clone, Copy)]
struct Args {
    size: usize,
    min_nodes: u64,
    max_nodes: u64,
    count: u64,
    threads: usize,
    solver: SolverChoice,
}

fn usage() -> ! {
    eprintln!(
        "Usage: calibrate_difficulty [--size=N] [--min-nodes=K] [--max-nodes=K] \
         [--count=C] [--threads=T] [--solver=basic|queue|black]"
    );
    eprintln!("  --size       grid side length, 3–11 (default: 6)");
    eprintln!(
        "  --min-nodes  minimum search-tree nodes the solver must visit (inclusive, default: 0)"
    );
    eprintln!(
        "  --max-nodes  maximum search-tree nodes the solver must visit (inclusive, default: unbounded)"
    );
    eprintln!("  --count      number of matching puzzles to sample (default: 1000)");
    eprintln!("  --threads    worker threads (default: available parallelism)");
    eprintln!("  --solver     solver implementation to use (default: black)");
    std::process::exit(1);
}

fn parse_args() -> Args {
    let mut args = Args {
        size: 6,
        min_nodes: 0,
        max_nodes: u64::MAX,
        count: 1000,
        threads: thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
        solver: SolverChoice::Black,
    };

    for arg in std::env::args().skip(1) {
        if let Some(val) = arg.strip_prefix("--size=") {
            args.size = val.parse().unwrap_or_else(|_| usage());
        } else if let Some(val) = arg.strip_prefix("--min-nodes=") {
            args.min_nodes = val.parse().unwrap_or_else(|_| usage());
        } else if let Some(val) = arg.strip_prefix("--max-nodes=") {
            args.max_nodes = val.parse().unwrap_or_else(|_| usage());
        } else if let Some(val) = arg.strip_prefix("--count=") {
            args.count = val.parse().unwrap_or_else(|_| usage());
        } else if let Some(val) = arg.strip_prefix("--threads=") {
            args.threads = val.parse().unwrap_or_else(|_| usage());
        } else if let Some(val) = arg.strip_prefix("--solver=") {
            args.solver = match val {
                "basic" => SolverChoice::Basic,
                "queue" => SolverChoice::Queue,
                "black" => SolverChoice::Black,
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
    if args.count == 0 {
        eprintln!("--count must be at least 1");
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

struct SharedState {
    grids: AtomicU64,
    done: AtomicBool,
}

impl SharedState {
    fn new() -> Self {
        Self {
            grids: AtomicU64::new(0),
            done: AtomicBool::new(false),
        }
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

fn dispatch_solver<const N: usize>(args: Args) {
    match args.solver {
        SolverChoice::Basic => run::<N, BasicSolverState<N>>(args),
        SolverChoice::Queue => run::<N, QueueSolverState<N>>(args),
        SolverChoice::Black => run::<N, BlackSolverState<N>>(args),
    }
}

// ── Run loop ──────────────────────────────────────────────────────────────────

fn run<const N: usize, S: Solver<N>>(args: Args) {
    let fast_path = args.max_nodes == 1;
    if fast_path {
        println!(
            "max-nodes=1: propagation-only fast path (no backtracking search; \
             every propagation-solvable puzzle counts as a sample)."
        );
    }
    println!(
        "Sampling {} puzzles of size {} with nodes in [{}, {}] on {} threads.",
        args.count,
        args.size,
        args.min_nodes,
        if args.max_nodes == u64::MAX {
            "∞".to_string()
        } else {
            args.max_nodes.to_string()
        },
        args.threads,
    );

    let shared = SharedState::new();
    let start = Instant::now();

    let pb = ProgressBar::new(args.count);
    pb.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>6}/{len} samples \
             {msg} ({eta} remaining)",
        )
        .unwrap()
        .progress_chars("=> "),
    );
    pb.set_message("(0 grids tried)".to_string());
    pb.enable_steady_tick(Duration::from_millis(100));

    let (tx, rx) = mpsc::channel::<u64>();
    let mut samples: Vec<u64> = Vec::with_capacity(args.count as usize);

    thread::scope(|s| {
        for _ in 0..args.threads {
            let tx = tx.clone();
            let shared = &shared;
            s.spawn(move || worker::<N, S>(args, shared, tx));
        }
        // Drop the main thread's sender so the channel closes once every
        // worker has exited — protects us if a window matches no puzzles
        // and the user never hits Ctrl-C: `Disconnected` will still break
        // the loop (though in practice workers will keep trying forever).
        drop(tx);

        loop {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(waves) => {
                    samples.push(waves);
                    pb.set_position(samples.len() as u64);
                    pb.set_message(format!(
                        "({} grids tried)",
                        shared.grids.load(Ordering::Relaxed)
                    ));
                    if samples.len() as u64 >= args.count {
                        shared.done.store(true, Ordering::Relaxed);
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    pb.set_message(format!(
                        "({} grids tried)",
                        shared.grids.load(Ordering::Relaxed)
                    ));
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => break,
            }
        }
    });

    pb.finish_and_clear();

    let total_grids = shared.grids.load(Ordering::Relaxed);
    let elapsed = start.elapsed();
    report(&samples, total_grids, elapsed);
}

fn worker<const N: usize, S: Solver<N>>(args: Args, shared: &SharedState, tx: mpsc::Sender<u64>) {
    let mut rng = rand::rng();

    while !shared.done.load(Ordering::Relaxed) {
        let grid = random_grid::<N>(&mut rng);
        shared.grids.fetch_add(1, Ordering::Relaxed);

        let (row_targets, col_targets) = grid.compute_targets();
        let puzzle = Puzzle::new(row_targets, col_targets);

        let waves = if args.max_nodes == 1 {
            // Propagation-only fast path: a propagation-solvable puzzle is
            // trivially uniquely solvable (no branching was needed).  Skip
            // the uniqueness-checking `solve()` and read the wave count
            // straight off the recorder.
            let mut st = S::new(puzzle);
            st.propagate();
            if !st.is_solved() {
                continue;
            }
            st.recorder().propagation_waves()
        } else {
            match S::new(puzzle).solve() {
                SolveOutcome::Unique(solved) => {
                    let nodes = solved.recorder().search_nodes();
                    if !(args.min_nodes..=args.max_nodes).contains(&nodes) {
                        continue;
                    }
                    solved.recorder().propagation_waves()
                }
                _ => continue,
            }
        };

        if tx.send(waves).is_err() {
            // Main thread is done; stop quietly.
            return;
        }
    }
}

// ── Reporting ─────────────────────────────────────────────────────────────────

fn report(samples: &[u64], total_grids: u64, elapsed: Duration) {
    println!();
    if samples.is_empty() {
        println!(
            "No samples collected after {} grids in {:.1}s.",
            total_grids,
            elapsed.as_secs_f64()
        );
        return;
    }

    let mut sorted: Vec<u64> = samples.to_vec();
    sorted.sort_unstable();
    let n = sorted.len();

    let min = sorted[0];
    let max = sorted[n - 1];
    let sum: u128 = sorted.iter().map(|&x| x as u128).sum();
    let mean = sum as f64 / n as f64;

    println!(
        "Collected {} samples from {} grids in {:.1}s ({:.0} grids/s).",
        n,
        total_grids,
        elapsed.as_secs_f64(),
        total_grids as f64 / elapsed.as_secs_f64().max(f64::EPSILON),
    );
    println!("min = {min}, max = {max}, mean = {mean:.2}");
    println!();

    // ── Histogram ─────────────────────────────────────────────────────────────
    // One row per wave count between min and hist_max (inclusive), so empty
    // buckets show up as visible gaps.  Bar widths scale to the densest row.
    let hist_max = max.min(1000);
    let mut counts: Vec<u64> = vec![0; (hist_max + 1) as usize];
    for &w in &sorted {
        counts[w.min(1000) as usize] += 1;
    }
    let max_count = counts.iter().copied().max().unwrap_or(0);
    let bar_width: u64 = 50;

    println!("Wave count histogram:");
    println!("  waves |  count | share");
    for w in min..=hist_max {
        let c = counts[w as usize];
        let share = c as f64 / n as f64;
        let bar_len = if max_count == 0 {
            0
        } else {
            (c * bar_width / max_count) as usize
        };
        let bar = "#".repeat(bar_len);
        let waves = if w == 1000 {
            "1k+".to_string()
        } else {
            w.to_string()
        };
        println!(
            "  {:>5} | {:>6} | {:>5.1}% {}",
            waves,
            c,
            share * 100.0,
            bar
        );
    }
    println!();

    // ── Percentiles ───────────────────────────────────────────────────────────
    // Linear interpolation between adjacent ranks (NumPy's default).  Wave
    // counts are integers, but reporting fractional percentiles makes it
    // obvious when the chosen rank falls on a bucket boundary.
    let pcts = [
        1.0,
        5.0,
        10.0,
        25.0,
        100.0 / 3.0,
        50.0,
        200.0 / 3.0,
        75.0,
        90.0,
        95.0,
        99.0,
    ];
    println!("Percentiles:");
    for &p in &pcts {
        let value = percentile(&sorted, p);
        println!("  p{:>5.2} = {:>6.2}", p, value);
    }
}

fn percentile(sorted: &[u64], p: f64) -> f64 {
    let n = sorted.len();
    debug_assert!(n > 0);
    if n == 1 {
        return sorted[0] as f64;
    }
    let pos = (p / 100.0) * (n - 1) as f64;
    let lo = pos.floor() as usize;
    let hi = pos.ceil() as usize;
    if lo == hi {
        sorted[lo] as f64
    } else {
        let frac = pos - lo as f64;
        sorted[lo] as f64 * (1.0 - frac) + sorted[hi] as f64 * frac
    }
}
