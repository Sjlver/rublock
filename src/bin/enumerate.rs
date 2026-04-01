use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use rublock::enumerate::{PartialGrid, count_from_partial, generate_partial_grids};
use rublock::grid::Cell;

/// Count all valid 6×6 rublock grids and report how many are valid puzzles
/// (i.e. have exactly one solution given their derived targets).
///
/// Strategy:
/// 1. BFS to produce a work queue of partial grids — enough items for good
///    parallel load distribution.  Target: 100 items per CPU core, so that
///    `rayon`'s work-stealing scheduler can handle uneven subtree sizes
///    while maintaining roughly linear progress.
/// 2. Process each work item in parallel using atomic counters to track the
///    running totals, so the progress bar can show live counts.
/// 3. Display a progress bar while work is in flight.
fn main() {
    let num_threads = rayon::current_num_threads();
    let target = num_threads * 100;

    // ── Build work queue ──────────────────────────────────────────────────────

    // Counting all 6x6 grids is a bit slow for my benchmark. Thus, start with a
    // grid that has a few cells filled already.
    let start = PartialGrid::<6>::new()
        .try_place(Cell::Black)
        .and_then(|g| g.try_place(Cell::Number(1)))
        .and_then(|g| g.try_place(Cell::Number(2)))
        .and_then(|g| g.try_place(Cell::Number(3)))
        .and_then(|g| g.try_place(Cell::Number(4)))
        .and_then(|g| g.try_place(Cell::Black))
        .and_then(|g| g.try_place(Cell::Number(1)))
        .and_then(|g| g.try_place(Cell::Number(2)))
        .expect("hard-coded initial placement must be valid");

    let work_items = generate_partial_grids(start, target);

    println!(
        "Work queue: {} items ({} threads × 100 target).",
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

    // ── Parallel enumeration ──────────────────────────────────────────────────
    //
    // Each work item is an independent subtree of the full search.  `rayon`
    // distributes them across the thread pool and steals work to keep all
    // cores busy.

    let total_grids_ref = Arc::clone(&total_grids);
    let valid_puzzles_ref = Arc::clone(&valid_puzzles);

    work_items.par_iter().for_each(|partial| {
        let (t, v) = count_from_partial(partial);
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

    println!("\nTotal valid grids:          {total}");
    println!("Valid puzzles (unique soln): {valid}");
}
