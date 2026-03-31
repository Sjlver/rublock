# Rublock roadmap

Rublock is a small puzzle solving project, to learn Rust. See README.md for the puzzle rules.

## Solver generalization for arbitrary N (done)

The solver in src/solver.rs is now fully generic over `const N: usize`.  All
rules, masks, and loops use N instead of the old hardcoded 6.  The bit-position
constants (BLACK1_ROW, ALL_DIGITS, etc.) are associated constants computed from N
at monomorphisation time.  `Tables` stores `max_sum` so that the outside-cage
target (`max_sum - t`) is derived correctly for any N.

Tests cover N=2 (degenerate all-black grid) and N=4 (two-digit puzzles) in
addition to the original N=6 newspaper puzzles.

Both `grid.rs` and `enumerate.rs` are now also fully generic over N.  `PartialGrid<N>`,
`Grid<N>`, and all helper functions use `N` throughout.  `bin/enumerate.rs` enumerates
5×5 grids (`N=5`) and shows live grid/puzzle counts in the progress bar via
`Arc<AtomicU64>` shared atomic counters.

## Solver improvements

I'd like to replace apply_black_range_rules with a more general version, something like generalized arc consistency. It goes like this:

- For a cell with a BLACK1 bit, check that bit's support as follows:
  - There must exist a valid tuple with sum=target and length=l, such that:
    - All the following l cells' domains intersect with the tuple's digits
    - The cell at distance l+1 has a BLACK2 bit
  - If there is no such tuple, we can remove the BLACK1 bit
- The same thing goes backwards:
  - There must exist a valid tuple with sum=10-target and length=l, such that
    - All the preceding l cells' domains intersect with the tuple's digits
    - The cell at distance l+1 backwards (wraps around the edge) has a BLACK2 bit set
  - If there is no such tuple, we can again remove the BLACK1 bit
- The same can be done equivalently for each cell with BLACK2 bit, to check that bit's support.

This is strictly more general than apply_black_range_rules. It would make d_min and d_max unnecessary, we can directly use the valid_tuples.

## Solver work queue or rule skipping

Currently, the solver runs the same rules over and over until a fixpoint is reached. This is a lot of unnecessary work. For example, the rules that clear BLACK bits run even if all the black cells are already determined.

I'd like to use something like a work queue instead (like in the AC-3 algorithm). If a cell's domain shrink, enqueue rules for that row and that column. Or generalize the `changed` boolean flag to a per-row and per-column flag. Rules that work on unchanged columns could be skipped. I kinda like this idea; it seems simpler than a work queue, and it plausibly reduces the amount of work that is done.

Maybe some rules could also have a cheap pre-conditions check at the start. For example, rules that clear BLACK bits can skip rows where the BLACK cells are already assigned.

## Nogood learning

I'd also like to change the backtracking loop to do some nogood learning. If it does an assignment, and then the recursive call does not find any solution, then we know the assignment is invalid. We can remove it from the domain and recurse, rather than trying the other values first. The recursive call will do a `propagate` and might make good use of the reduced domain.
