# Rublock roadmap

Rublock is a small puzzle solving project, to learn Rust. See README.md for the puzzle rules and the goals of the project.

## A second, work queue based solver

Add a second solver in a separate file (`queue_solver.rs`), keeping the original `solver.rs` intact so the two can be compared.

The new solver is structured around a **work queue** and a set of **constraint systems**. The shared state is the same domain representation as the original solver: each cell holds a `CellDomain` bitmask with digit bits (1..=N-2), two row-black bits (BLACK1\_ROW, BLACK2\_ROW), and two col-black bits (BLACK1\_COL, BLACK2\_COL).

### The work queue

The queue holds `(row, col, bit)` triples, one per removed domain bit. It is fed by `clear_mask`, which enqueues a triple whenever it actually clears a bit (i.e. the domain shrank). Because `clear_mask` only fires on genuine shrinkage and domains only ever shrink, each `(row, col, bit)` triple is enqueued **at most once** — no deduplication is needed.

Each constraint system exposes an `update(row, col, bit)` method that is called when a bit is dequeued. Handlers may call `clear_mask`, which enqueues further triples. Propagation runs until the queue is empty.

### Initialization

When a `QueueSolverState` is constructed:

1. **Enumerate live tuples.** For each row and each column, compute the full set of tuples that are consistent with the row/col target. A tuple has the form `[BLACK1, d₁, d₂, …, dₖ, BLACK2]` where the digits sum to the inside target (non-wrapping) or outside target (wrapping). Only tuples that physically fit in the row are kept. This gives the initial `live_row[r]` and `live_col[c]` sets.

2. **Compute initial support.** For each cell `(r, c)`, compute the union of all pattern bits from every live tuple in row `r` that covers column `c`. This union is the set of row-direction bits that are actually supported at `(r, c)`. Do the same in the column direction. Any domain bit not present in either union is already dead.

3. **Seed the queue.** Call `clear_mask` for every unsupported bit found in step 2. This produces the initial queue entries and bootstraps propagation.

### Constraint systems

Each constraint system holds auxiliary counters that are derived from, and kept consistent with, the domain state. They are updated incrementally via `update`.

#### 1. Singleton constraints

Equivalent to `apply_singleton_rule`.

A cell's **row domain** is `domain & (ALL_DIGITS | ROW_BLACKS)` and its **col domain** is `domain & (ALL_DIGITS | COL_BLACKS)`. Each cell carries two counters:

- `row_count[r][c]` — number of bits set in the cell's row domain.
- `col_count[r][c]` — number of bits set in the cell's col domain.

Digit bits appear in both domains and decrement both counters. Row-black bits decrement only `row_count`; col-black bits decrement only `col_count`.

**`update(r, c, bit)`:** Decrement the appropriate counter(s). If `row_count[r][c]` reaches 1, pass the remaining row domain directly to `set_cell(r, c, domain & (ALL_DIGITS | ROW_BLACKS))` — it already has exactly one bit, so no extraction is needed. Same for `col_count`.

#### 2. Hidden singles constraints

Equivalent to `apply_hidden_single_rule`.

Two counter tables:

- `count_row[r][bit]` — number of cells in row `r` whose domain contains `bit`.
- `count_col[c][bit]` — number of cells in column `c` whose domain contains `bit`.

At construction, initialise from the full domains.

**`update(r, c, bit)`:**
- If `bit` is a digit or row-black bit: decrement `count_row[r][bit]`. If it reaches 1, scan row `r` to find the surviving cell `c'` that still has `bit`, then call `set_cell(r, c', bit)`.
- If `bit` is a digit or col-black bit: decrement `count_col[c][bit]`. If it reaches 1, scan column `c` to find the surviving cell `r'`, then call `set_cell(r', c, bit)`.

Digit bits trigger both branches.

#### 3. Black consistency constraints

Equivalent to `apply_black_consistency_rule`.

Each cell carries:

- `row_black_count[r][c]` — number of row-black bits (BLACK1\_ROW, BLACK2\_ROW) still set in the domain; starts at 2.
- `col_black_count[r][c]` — same for col-black bits; starts at 2.

**`update(r, c, bit)`:**
- If `bit` is a row-black bit: decrement `row_black_count[r][c]`. If it reaches 0, call `clear_mask` on both BLACK1\_COL and BLACK2\_COL for cell `(r, c)`.
- If `bit` is a col-black bit: decrement `col_black_count[r][c]`. If it reaches 0, call `clear_mask` on both row-black bits for cell `(r, c)`.

The resulting `update` calls from `clear_mask` are harmless: by the time they are processed, the target bits are already gone.

#### 4. General arc consistency constraints

Equivalent to `apply_general_arc_consistency`. This is the most complex constraint system.

##### Tuples

A **tuple** represents one valid placement of the cage pattern within a row (or column). It records:

- `start` — the column (or row) index of the first cell in the span.
- `pattern` — an array of bitmasks (one per cell in the span), of the form `[BLACK1_bit, digit_mask, …, digit_mask, BLACK2_bit]` for an inside tuple, or `[BLACK2_bit, digit_mask, …, digit_mask, BLACK1_bit]` for an outside (wrapping) tuple. The `digit_mask` is the union of all digit bits belonging to the digit subset for this tuple.

The column for position `p` is `(start + p) % N`. Whether a tuple wraps is implicit: it wraps iff `start + pattern.len() > N`. There is no need to store a `wrap` field.

A tuple is **live** if, for every position `p` in its span, `domain[row][(start + p) % N] & pattern[p] != 0`. This is a **conservative over-approximation**: it checks per-cell compatibility (condition A) but not that every pattern bit is covered by at least one cell (condition B). For example, the target-7 tuple `[BLACK1, {3,4}, {3,4}, BLACK2]` would remain live even if both intermediate cells have only `{4}` in their domain — digit 3 is uncoverable but condition A still holds. The existing solver's `is_pattern_supported` catches both conditions; the queue solver catches only condition A.

This means some dead tuples linger in the live set longer, producing a weaker propagation. The solver remains correct (no valid bits are ever removed), just less aggressive at pruning. The support-count mechanism described below still removes bits that reach zero support, which closes some — but not all — of the gap.

When `update(row, col, bit)` is called, any live tuple covering `col` with `bit` present in its pattern at the position corresponding to `col` must be rechecked: if `domain[row][col] & pattern[pos] == 0`, the tuple has died.

##### Support counts

To propagate the effect of a dead tuple back onto the domains, each cell position maintains a support count per domain bit:

- `support_row[r][c][bit]` — number of live tuples in row `r` whose pattern at column `c` includes `bit`.
- `support_col[r][c][bit]` — same for column direction.

These are initialised at construction from the initial live tuple sets.

When a tuple dies, decrement `support_row[r][c'][bit']` (or `support_col`) for every `(c', bit')` in its span. If any counter reaches 0, call `clear_mask(r, c', bit')`, which enqueues the removal.

##### Variant A — scan (simple)

Live tuples for a row are stored in a plain `Vec<Tuple>` (`live_row[r]`). Column tuples go in `live_col[c]`.

**`update(r, c, bit)`:**
1. Iterate `live_row[r]`. For each tuple T:
   - Skip if T does not cover column `c`.
   - Skip if `bit` is not in T's pattern at the position corresponding to `c`.
   - Recheck liveness: if `domain[r][c] & T.pattern[pos] == 0`, T is dead.
     - Swap-remove T from `live_row[r]`.
     - For every position `(c', bit')` in T's span, decrement `support_row[r][c'][bit']`; if 0, call `clear_mask`.
2. Repeat for `live_col[c]` in the column direction.

**Properties:**
- Data structures are simple `Vec`s — easy to implement, easy to clone for backtracking.
- Each call scans all live tuples for a row. For N=6 there are at most ~96 tuples per row/col in the worst case, so this is fast in practice.
- Swap-remove makes deletion O(1) but does not preserve order (order does not matter here).

##### Variant B — bidirectional links (complex)

Tuples are stored in a pool and referenced by `TupleId`. Two reverse indices are maintained:

- `cell_to_tuples_row[r][c][bit]` — list of `TupleId`s of live row-tuples in row `r` whose pattern at column `c` includes `bit`.
- `cell_to_tuples_col[r][c][bit]` — same for column tuples.

**`update(r, c, bit)`:**
1. Look up `cell_to_tuples_row[r][c][bit]`. For each `TupleId` in that list:
   - If the tuple has already been removed, skip.
   - Recheck liveness at position `c`: if `domain[r][c] & T.pattern[pos] == 0`, T is dead.
     - Mark T as removed.
     - For every `(c', bit')` in T's span, remove T from `cell_to_tuples_row[r][c'][bit']`, decrement `support_row[r][c'][bit']`; if 0, call `clear_mask`.
2. Repeat for the column direction.

**Properties:**
- Only checks tuples that specifically include the removed bit — avoids scanning unrelated tuples.
- Much more bookkeeping: the reverse index must be updated on every tuple removal, and must be correctly cloned for backtracking.
- For N=6 the scan in Variant A is cheap enough that the extra complexity of Variant B is unlikely to be worthwhile.

**Decision:** Start with Variant A. The live-tuple list per row/col is at most a few dozen entries, making the linear scan negligible. Variant B can be revisited if profiling shows the scan is a bottleneck.

### Backtracking

Backtracking works identically to the existing solver: clone the entire `QueueSolverState` (including all counters, live-tuple vecs, support-count arrays, and the queue itself) before committing to a branch. If a branch fails, discard it and continue with the saved snapshot.

The queue is empty at the point of branching (propagation runs to fixpoint first), so the clone captures a clean, fully-propagated state.
