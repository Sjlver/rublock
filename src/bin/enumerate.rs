use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use indicatif::{ProgressBar, ProgressStyle};
use rand::seq::SliceRandom;
use rayon::prelude::*;
use rublock::enumerate::{PartialGrid, SolverChoice, count_from_partial, generate_partial_grids};

fn usage() -> ! {
    eprintln!("Usage: enumerate [--size=N] [--solver=basic|queue]");
    eprintln!("  --size    grid side length, 3–11 (default: 6)");
    eprintln!("  --solver  basic or queue (default: queue)");
    std::process::exit(1);
}

fn parse_args() -> (usize, SolverChoice) {
    let mut size = 6usize;
    let mut solver = SolverChoice::Queue;

    for arg in std::env::args().skip(1) {
        if let Some(val) = arg.strip_prefix("--size=") {
            size = val.parse().unwrap_or_else(|_| usage());
        } else if let Some(val) = arg.strip_prefix("--solver=") {
            solver = match val {
                "basic" => SolverChoice::Basic,
                "queue" => SolverChoice::Queue,
                _ => usage(),
            };
        } else {
            usage();
        }
    }

    if !(3..=11).contains(&size) {
        eprintln!("--size must be between 3 and 11");
        std::process::exit(1);
    }

    (size, solver)
}

fn run<const N: usize>(solver: SolverChoice) {
    let started_at = Instant::now();
    let num_threads = rayon::current_num_threads();
    let target = num_threads * 1000;

    // ── Build work queue ──────────────────────────────────────────────────────

    let start = PartialGrid::<N>::new();

    let mut work_items = generate_partial_grids(start, target);
    work_items.shuffle(&mut rand::rng());

    println!("Enumerating grids of size {} using {} solver", N, solver);
    println!(
        "Work queue: {} items ({} threads × 1000 target).",
        work_items.len(),
        num_threads,
    );

    // ── Shared atomic counters ────────────────────────────────────────────────
    //
    // Workers increment these atomically so the progress bar can read live
    // totals without synchronisation overhead on the hot path.  Relaxed
    // ordering is sufficient: we only need the final value to be correct, not
    // any ordering guarantee relative to other memory operations.

    let total_grids = Arc::new(AtomicU64::new(0));
    let valid_puzzles = Arc::new(AtomicU64::new(0));

    // ── Set up progress bar ───────────────────────────────────────────────────

    let pb = ProgressBar::new(work_items.len() as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] [{bar:40.cyan/blue}] {pos:>6}/{len} items  \
             grids={msg}  ({eta} remaining)",
        )
        .unwrap()
        .progress_chars("=> "),
    );
    pb.set_message(format!(
        "{} ({} valid)",
        total_grids.load(Ordering::Relaxed),
        valid_puzzles.load(Ordering::Relaxed),
    ));

    // ── Parallel enumeration ──────────────────────────────────────────────────
    //
    // Each work item is an independent subtree of the full search.  `rayon`
    // distributes them across the thread pool and steals work to keep all
    // cores busy.

    let total_grids_ref = Arc::clone(&total_grids);
    let valid_puzzles_ref = Arc::clone(&valid_puzzles);

    work_items.par_iter().for_each(|partial| {
        let (t, v) = count_from_partial(partial, solver);
        total_grids_ref.fetch_add(t, Ordering::Relaxed);
        valid_puzzles_ref.fetch_add(v, Ordering::Relaxed);
        // Update message with live counts before incrementing the bar.
        pb.set_message(format!(
            "{} ({} valid)",
            total_grids_ref.load(Ordering::Relaxed),
            valid_puzzles_ref.load(Ordering::Relaxed),
        ));
        pb.inc(1);
    });

    pb.finish_with_message("done");

    // ── Report ────────────────────────────────────────────────────────────────

    let total = total_grids.load(Ordering::Relaxed);
    let valid = valid_puzzles.load(Ordering::Relaxed);

    println!("\nTotal valid grids:            {total:10}");
    println!("Valid puzzles (unique soln):  {valid:10}");

    let elapsed_s = started_at.elapsed().as_secs_f64();
    let grids_per_s = (total as f64) / elapsed_s.max(f64::EPSILON);
    println!("Time: {elapsed_s:.3} seconds ({grids_per_s:.1} grids per second)");
}

fn main() {
    let (size, solver) = parse_args();
    match size {
        3 => run::<3>(solver),
        4 => run::<4>(solver),
        5 => run::<5>(solver),
        6 => run::<6>(solver),
        7 => run::<7>(solver),
        8 => run::<8>(solver),
        9 => run::<9>(solver),
        10 => run::<10>(solver),
        11 => run::<11>(solver),
        _ => unreachable!(), // validated in parse_args
    }
}
