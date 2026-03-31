use indicatif::{ProgressBar, ProgressStyle};
use rayon::prelude::*;
use rublock::enumerate::{count_from_partial, generate_partial_grids};

/// Count all valid 6×6 rublock grids and report how many are valid puzzles
/// (i.e. have exactly one solution given their derived targets).
///
/// Strategy:
/// 1. BFS to produce a work queue of partial grids — enough items for good
///    parallel load distribution.  Target: 100 items per CPU core, so that
///    `rayon`'s work-stealing scheduler can handle uneven subtree sizes
///    while maintaining roughly linear progress.
/// 2. Process each work item in parallel, accumulating counts.
/// 3. Display a progress bar while work is in flight.
fn main() {
    let num_threads = rayon::current_num_threads();
    let target = num_threads * 100;

    // ── Build work queue ──────────────────────────────────────────────────────

    let work_items = generate_partial_grids(target);

    println!(
        "Work queue: {} items ({} threads × 100 target).",
        work_items.len(),
        num_threads,
    );

    // ── Set up progress bar ───────────────────────────────────────────────────

    let pb = ProgressBar::new(work_items.len() as u64);
    pb.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] [{bar:50.cyan/blue}] {pos:>6}/{len} items  ({eta} remaining)",
        )
        .unwrap()
        .progress_chars("=> "),
    );

    // ── Parallel enumeration ──────────────────────────────────────────────────
    //
    // Each work item is an independent subtree of the full search.  `rayon`
    // distributes them across the thread pool and steals work to keep all
    // cores busy.  The two `u64` counts are folded with addition, which is
    // associative and commutative — safe to reduce in any order.

    let (total, valid) = work_items
        .par_iter()
        .map(|partial| {
            let counts = count_from_partial(partial);
            pb.inc(1);
            counts
        })
        .reduce(|| (0, 0), |(a0, b0), (a1, b1)| (a0 + a1, b0 + b1));

    pb.finish_with_message("done");

    // ── Report ────────────────────────────────────────────────────────────────

    println!("\nTotal valid grids:          {total}");
    println!("Valid puzzles (unique soln): {valid}");
}
