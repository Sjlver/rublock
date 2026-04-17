# Performance Roadmap

Ideas for improving `QueueSolverState` performance, roughly ordered by
expected impact. All measurements should be done with `cargo bench` against
the standard enumeration workload.

---

## Idea 1: Fixed-size array for `LiveTuple::pattern`

**Priority: High**

`LiveTuple::pattern` is a `Vec<u64>`, so every tuple owns a separate heap
allocation. A live state contains O(N × tuples_per_row) tuples. Every clone
— one per backtracking step — must walk and clone all of them.

**Fix:** Replace with a fixed-size `[u64; N]` plus a `len: u8` field. Patterns
are at most N elements long (one element per column/row), so this is always
safe. The `start` field can also shrink to `u8`.

```rust
struct LiveTuple<const N: usize> {
    start: u8,
    len:   u8,
    pattern: [u64; N],
}
```

All iteration over `pattern` changes from `0..self.pattern.len()` to
`0..self.len as usize`. With this change, `live_tuples_row` and
`live_tuples_col` become `Vec<LiveTuple<N>>` where each element is
plain-old-data — no pointer chasing — and `clone()` reduces to a `memcpy`
over those Vecs.

---

## Idea 2: Fixed-size arrays for `row_candidates` / `col_candidates`

**Priority: High**

`row_candidates: [Vec<u8>; N]` and `col_candidates: [Vec<u8>; N]` are
currently 2N tiny heap allocations (one per row/column) each holding N+3
bytes. That is 12 allocations for N=6, multiplied by every backtracking clone.

The same problem was already solved for `tuple_support_row` by splitting it
into separate digit and black arrays. Do the same here:

```rust
row_candidates_digit: [[u8; N]; N],  // [row][trailing_zeros of digit bit]
row_candidates_black: [[u8; 2]; N],  // [row][0 = BLACK1, 1 = BLACK2]
col_candidates_digit: [[u8; N]; N],
col_candidates_black: [[u8; 2]; N],
```

Add a pair of helper methods analogous to `tuple_support_row` / `tuple_support_col`:

```rust
fn row_candidates_for(&mut self, r: usize, bit: u64) -> &mut u8 { ... }
fn col_candidates_for(&mut self, c: usize, bit: u64) -> &mut u8 { ... }
```

This keeps all call sites readable and mirrors the existing convention.

---

## Idea 3: Shrink `CellDomain` from `u64` to `u16`

**Priority: Medium**

The maximum practical grid size is N=11 (anything larger would require
hexadecimal digits in the puzzle display). The bits needed for N=11 are:
digits 1–9 (bits 1–9), BLACK1\_ROW (bit 10), BLACK2\_ROW (bit 11),
BLACK1\_COL (bit 12), BLACK2\_COL (bit 13) — 13 bits total. A `u16` holds
16, so it fits for all supported grid sizes.

**What shrinks:**

| Field | N=6 before | N=6 after | N=11 before | N=11 after |
|---|---|---|---|---|
| `domains: [[CellDomain; N]; N]` | 288 B | 72 B | 968 B | 242 B |
| `LiveTuple::pattern: [CellDomain; N]` (after Idea 1) | 48 B/tuple | 12 B/tuple | 88 B/tuple | 22 B/tuple |

The four `tuple_support_*` arrays are already `u16` — no change there.

**Struct size effect for N=6:** `domains` shrinks from 288 to 72 bytes.
The support arrays dominate the struct at ~1152 bytes and are unchanged, so
the overall struct shrinks by roughly 13%. The cache benefit is modest at N=6
but grows at larger N where `domains` is proportionally larger.

**Queue entries:** `(usize, usize, u64)` → `(usize, usize, u16)`. On 64-bit
targets, alignment padding keeps the entry at 24 bytes regardless — no savings
without also shrinking the index types.

**Bitwise op cost:** Negligible. x86-64 `POPCNT`, `TZCNT`, and `BLSI` all
operate on 16-bit registers without penalty.

**Implementation effort:** Very low. Change the type alias:

```rust
type CellDomain = u16;
```

and update the const definitions (replace `1u64` literals with `1u16` in
`ALL_DIGITS`, `BLACK1_ROW`, etc.). The rest compiles unchanged.

**Estimated speedup:** ~10–15% for N=6; potentially more for N=9–11 where
`domains` is a larger fraction of the struct. Stacks with Idea 1: smaller
tuple patterns + smaller domains means more of the working set fits in L1.

**Related: shrinking index types.** Row and column indices never exceed N=11,
so `u8` would be sufficient. The only place this would affect *memory* is the
queue entry `(usize, usize, u64)` → `(u8, u8, u16)`, saving 20 bytes per
entry (24 → 4 bytes). Everywhere else — function parameters, local variables,
loop counters — indices live in registers and their type has no memory impact.
The problem is that `usize` is Rust's canonical index type: array indexing
(`self.domains[r][c]`), `for i in 0..N`, and the entire standard library all
work in `usize`. Storing `u8` indices means writing `r as usize` at every
index site, which Clippy discourages and which adds noise without benefit. Not
worth it; keep `usize` for indices throughout.

---

## Idea 4: Replace `VecDeque` with `Vec` (stack discipline)

**Priority: Low / benchmark first**

The propagation queue is a `VecDeque<(usize, usize, u64)>` used FIFO. Queue
order doesn't affect correctness — propagation reaches the same fixpoint
regardless of order — so LIFO (a plain `Vec` with `push`/`pop`) is equally
correct.

A `Vec` avoids the two-pointer wrap-around bookkeeping of `VecDeque` and
keeps the hot end at a single memory location, which is friendlier to the
CPU cache.

**Plan:** Replace `queue: VecDeque<...>` with `queue: Vec<...>`, `push_back`
→ `push`, `pop_front` → `pop`. Commit this change in isolation, then run
`cargo bench` to see whether it makes any difference in practice before
keeping or reverting it.

---

## Idea 5: Early exit in `pick_branching_cell`

**Priority: Trivial**

The current implementation always scans all N² = 36 cells, even after
finding a cell with exactly 2 choices — the minimum possible for any
undecided cell (`branching_bits` returns 0 for fully-determined cells).

```rust
if freedom == 2 {
    return Some((r, c));
}
```

This is a one-line change with zero risk. At backtracking nodes near the
leaves of the search tree, most cells are already determined, so the first
2-choice cell is found near the beginning of the scan.

---

## Idea 6: Experiment — disable individual propagation rules

**Priority: Experimental**

Each propagation rule reduces the search tree but adds per-node overhead.
The history in `benchmarks.md` already shows one surprising result: adding
the general arc-consistency rule made the solver *slower* overall (1.231 s
vs 0.532 s), even though it prunes more.

The question is whether the same is true for the other rules in
`QueueSolverState`.

Suggested experiments, from cheapest to most invasive:

1. **Disable `update_black_consistency`**: remove the call and its associated
   `row_blacks_left` / `col_blacks_left` state. How much does the search tree
   grow?

2. **Disable `update_hidden_singles`**: remove the call and `row_candidates` /
   `col_candidates`. Hidden singles are O(N) per removed bit; are they worth it?

3. **Disable `update_arc`** entirely: the arc-consistency machinery is the
   most expensive rule (O(live_tuples) per removed bit, plus all the support
   count bookkeeping). Try running with just `update_singleton` +
   `update_hidden_singles` + `update_black_consistency`.

4. **Pure backtracking baseline**: no propagation at all — only branch and
   check for contradiction. This gives a lower bound on overhead and an
   upper bound on search-tree benefit.

The cleanest implementation is to gate each rule behind a const generic bool
parameter (e.g., `QueueSolverState<N, ARC_CONSISTENCY>`). With a `false`
const, the entire rule — including state fields and bookkeeping — can be
compiled away with `if ARC_CONSISTENCY { ... }` blocks, making comparisons
apples-to-apples.

---

## Idea 7: Reverse-index tuples by cell (re-assessed, lower priority)

**Priority: Low**

`update_arc` scans all of `live_tuples_row[r]` to find tuples covering
column `c`. A reverse index — `cells_to_row_tuples[r][c]: Vec<usize>` —
would restrict the scan to only the tuples that actually cover cell `(r, c)`.

However, the total number of live tuples per row is small. A typical target
has only 1–2 valid digit-set patterns (e.g., target 5 → {1,4} or {2,3}).
Each pattern yields at most N starting positions, and each tuple covers at
most N cells. So the live tuple count is at most ~4 × N = 24 for N=6.
The current scan is therefore already short in practice.

A reverse index adds non-trivial bookkeeping: when `swap_remove(i)` moves
the last tuple to slot `i`, the reverse index entries for every cell that
last tuple covers must be updated from `len` to `i`. Implement this only if
profiling shows `update_arc`'s linear scan is a genuine bottleneck.

---

## Non-idea: liveness check in `update_arc`

The `.any()` loop in `update_arc`:

```rust
(0..t.pattern.len()).any(|i| {
    let c2 = (t.start + i) % N;
    self.domains[r][c2] & bit != 0
})
```

could be re-considered. It makes the liveness check a bit stricter (although it
is still an over-approximation). However, the cost of the scan might not be
worth it.
