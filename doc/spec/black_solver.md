# Black Solver (`src/black_solver.rs`)

The default solver. It is selected by the `--solver=black` flag (the default)
and is what the web app calls through the WASM binding.

## What it solves

Given an N×N Doplo puzzle (row and column targets), find all assignments of
cells to {black, 1, 2, …, N−2} that satisfy every constraint, or report that
none exist.

## Domain representation

Every cell holds a **bitmask** (`CellDomain = u64`) of the values it might
still take:

| Bit | Meaning |
|---|---|
| 0 | black |
| 1 … N−2 | digit values |

`FULL_DOMAIN` has all bits set. A cell with `domain == 0` is a contradiction;
a cell with `domain.count_ones() == 1` is fully determined.

## `LiveTuple`

A **live tuple** is a candidate assignment for an entire row (or column). Each
tuple fixes:

- the **start position** of the left black cell,
- the **digit-set bitmask** (which digits appear between the two black cells),
- implicitly the right black cell and the outer digits (those outside the cage).

A tuple is *alive* as long as every cell in the row is still consistent with
the tuple's pattern. When the last tuple supporting a particular (cell, value)
pair dies, that value is removed from the cell's domain.

Initial live-tuple sets are built from the precomputed `Tables` in
`src/solver.rs`, which enumerate every valid digit subset for every target.

## Propagation rules

All three rules feed a FIFO queue of pending bit-removals. `propagate()` drains
the queue to a fixpoint.

### 1. Singleton

When a cell's domain shrinks to a single value, that value is *set*:
- If it is a digit, remove it from every other cell in the same row and column.
- If it is black, check whether the row/column now has exactly two black cells;
  if so, remove black from all remaining cells in that line.

### 2. Hidden single

Track `row_candidates[row][bit]`: how many cells in the row still allow this
bit. When a digit's count reaches 1, the sole remaining cell must take that
digit. When black's count reaches 2, those two cells must be the black pair.

### 3. Arc consistency (live-tuple pruning)

When a bit is removed from a cell, re-check every live tuple in the same row
(and column) that covers that cell. If a tuple's pattern is no longer
consistent with the current domains, remove the tuple and decrement support
counts. When a support count for `(cell, bit)` reaches zero, remove `bit` from
that cell's domain.

## Backtracking

When propagation reaches a fixpoint without solving the puzzle:

1. **Pick a cell**: the cell with the fewest remaining domain bits (MRV
   heuristic), returning immediately if a cell with 2 bits is found.
2. **Pick a bit**: the bit with the smallest live-tuple support count (fail-first).
3. Clone the state, commit to the chosen value (`take_branch`), propagate.
4. If that branch contradicts, exclude the value (`reject_branch`) and propagate
   the other direction.

The backtracking loop lives in `src/backtrack.rs` and is shared by all solver
implementations via the `Solver` trait.

## Recorder

`BlackSolverState<N, R>` is generic over a `Recorder` type parameter (default:
`SearchNodes`). The recorder observes bit-removals and step boundaries without
affecting solver logic. Available recorders:

- `SearchNodes` — cheapest; just counts backtracking nodes.
- `FullStats` — counts bits removed per rule category.
- `Explain` — records every propagation event for step-by-step explanation in
  the web UI's walkthrough tab.

## Entry points

```rust
// Default recorder (SearchNodes)
let state = BlackSolverState::<6>::new(puzzle);

// Custom recorder
let state = BlackSolverState::<6, FullStats>::with_recorder(puzzle);

let outcome: SolveOutcome<_> = state.solve();
```
