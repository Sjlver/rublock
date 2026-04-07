# Implementation plan — work-queue solver

This document is a step-by-step implementation plan for `queue_solver.rs`. It assumes familiarity with the existing `solver.rs` and the design rationale in `ROADMAP.md`.

---

## 1. Shared infrastructure

No changes to `solver.rs`, `grid.rs`, `changeset.rs`, or the `Tables` / `Puzzle` types. `queue_solver.rs` will import `Puzzle` and `Tables` from `solver.rs` (or those types can be extracted to a shared module later).

---

## 2. `LiveTuple`

```rust
struct LiveTuple {
    start:   usize,    // index of the first cell in the span
    pattern: Vec<u64>, // one bitmask per position; pattern.len() = cage_size + 2
}
```

Helper methods (all take `N: usize` as a parameter, or live on `QueueSolverState`):

| Method | Body |
|---|---|
| `col_at(p, N)` | `(self.start + p) % N` |
| `wraps(N)` | `self.start + self.pattern.len() > N` |
| `covers(c, N)` | exists `p` such that `col_at(p, N) == c` |
| `pos_of(c, N)` | the `p` such that `col_at(p, N) == c`; returns `Option<usize>` |

**Note on `col_at`:** Since `start + p` can exceed `N` by at most `N-1` (the span wraps at most once), a conditional subtract — `if self.start + p >= N { self.start + p - N } else { self.start + p }` — is equivalent and avoids a modulo. With `N` as a const generic, LLVM can optimise `% N` at compile time (multiply-by-reciprocal for non-power-of-two `N`), so the two forms likely produce comparable code. Use `% N` for clarity; check with `cargo asm` or Compiler Explorer if performance matters.

---

## 3. `QueueSolverState` — struct fields

```rust
pub struct QueueSolverState<const N: usize> {
    pub puzzle: Puzzle<N>,
    domains:    [[CellDomain; N]; N],
    tables:     Arc<Tables>,
    queue:      VecDeque<(usize, usize, u64)>,  // (row, col, bit)

    // ── Singleton constraint ──────────────────────────────────────────────────
    // How many value-choices does this cell have in the row / col view?
    // Row view = domain & (ALL_DIGITS | ROW_BLACKS)
    // Col view = domain & (ALL_DIGITS | COL_BLACKS)
    // Digit bits count toward both.
    row_domain_size: [[u8; N]; N],
    col_domain_size: [[u8; N]; N],

    // ── Hidden singles constraint ─────────────────────────────────────────────
    // How many candidate cells does this bit have in this row / col?
    // row_candidates[r][p] = cells in row r whose domain has bit (1 << p).
    // col_candidates[c][p] = cells in col c whose domain has bit (1 << p).
    // Indexed by bit position p = bit.trailing_zeros().
    // Max bit position = N+2; each Vec has length N+3.
    row_candidates: [Vec<u8>; N],
    col_candidates: [Vec<u8>; N],

    // ── Black consistency constraint ──────────────────────────────────────────
    // How many row-black / col-black bits remain in this cell's domain?
    row_blacks_left: [[u8; N]; N],  // starts at 2
    col_blacks_left: [[u8; N]; N],  // starts at 2

    // ── General arc consistency constraint ────────────────────────────────────
    live_row: [Vec<LiveTuple>; N],  // live row-direction tuples, per row
    live_col: [Vec<LiveTuple>; N],  // live col-direction tuples, per col

    // support_row[r][c][p] = number of live row-tuples in row r
    //   whose pattern at column c includes bit (1 << p).
    // support_col[r][c][p] = same for col-direction tuples.
    // Each inner Vec has length N+3.
    support_row: [[Vec<u32>; N]; N],
    support_col: [[Vec<u32>; N]; N],
}
```

All fields must be `Clone`. `Vec` fields clone element-by-element; fixed arrays clone via `Copy` or derive. `Arc<Tables>` clones cheaply (reference count).

---

## 4. `new()` — construction and initialization

**Counter invariant:** Every counter reflects the full domain state as it was *before* any queued bit-removal has been processed. When `clear_mask` removes a bit from a domain and enqueues it, the counters are *not* decremented yet — that happens in `update` when the bit is dequeued. This means counters must always be initialized from the domain state that existed *before* any `clear_mask` calls, so that each subsequent `update` call brings them into sync correctly.

Construction proceeds in five phases:

### Phase 1 — enumerate live tuples

For each row `r`:
- For each `(len, digit_mask)` in `tables.valid_tuples_for_target(row_targets[r])` (inside tuples):
  - Pattern = `[BLACK1_ROW, digit_mask × len, BLACK2_ROW]`, length = `len + 2`.
  - For each `start` in `0..N` such that `start + len + 2 <= N`: push `LiveTuple { start, pattern }` into `live_row[r]`.
- For each `(len, digit_mask)` in `tables.valid_tuples_for_target(outside_target)` (outside/wrapping tuples):
  - Pattern = `[BLACK2_ROW, digit_mask × len, BLACK1_ROW]`, length = `len + 2`.
  - For each `start` in `0..N` such that `start + len + 2 > N`: push into `live_row[r]`.

Repeat symmetrically for columns, using `COL_BLACKS` bits.

### Phase 2 — initialise support counts

For each row `r`, for each live tuple T in `live_row[r]`:
- For each position `p` in `0..T.pattern.len()`:
  - Let `c = (T.start + p) % N` and `bits = T.pattern[p]`.
  - For each set bit `b` in `bits`: increment `support_row[r][c][b.trailing_zeros()]`.

Repeat for columns.

### Phase 3 — initialise all counters from the full domain

The domains are still full (no bits have been cleared yet). Initialize all counters now, **before** any `clear_mask` call, so that the counter invariant above holds throughout propagation.

For each cell `(r, c)`:
- `row_domain_size[r][c]` = `(full_domain & (ALL_DIGITS | ROW_BLACKS)).count_ones()`
- `col_domain_size[r][c]` = `(full_domain & (ALL_DIGITS | COL_BLACKS)).count_ones()`
- `row_blacks_left[r][c]` = `(full_domain & ROW_BLACKS).count_ones()`  → always 2 initially
- `col_blacks_left[r][c]` = `(full_domain & COL_BLACKS).count_ones()`  → always 2 initially

For each row `r` and each bit position `p` (0..N+3):
- `row_candidates[r][p]` = number of cells in row `r` with bit `(1 << p)` set in the full domain.

For each col `c` and each bit position `p`:
- `col_candidates[c][p]` = number of cells in col `c` with bit `(1 << p)` set in the full domain.

### Phase 4 — seed the queue with unsupported bits

Now scan for bits that are already dead — present in the full domain but unsupported by any live tuple:

- A row-direction bit (digit or row-black) at `(r, c)` is dead if `support_row[r][c][p] == 0`.
- A col-direction bit (digit or col-black) at `(r, c)` is dead if `support_col[r][c][p] == 0`.

For each such dead bit, call `clear_mask(r, c, bit)`. This removes the bit from `domains[r][c]` and enqueues `(r, c, bit)`. The counters are **not** updated here; that happens during Phase 5.

### Phase 5 — initial propagation

Call `propagate()` to drain the queue seeded in Phase 4. Each dequeued `(r, c, bit)` calls `update`, which decrements the relevant counters and may enqueue further removals.

---

## 5. `clear_mask` and `set_cell`

### `clear_mask(r, c, mask)`

```
let before = domains[r][c];
domains[r][c] &= !mask;
let removed = before & !domains[r][c];
for each set bit b in removed:
    queue.push_back((r, c, b));
```

Counters are **not** touched here. The enqueued triples will decrement counters via `update` during `propagate`.

### `set_cell(r, c, bit)`

Identical logic to the existing `SolverState::set_cell`. It calls `clear_mask` internally, so newly removed bits are automatically enqueued. Copy the implementation directly, replacing the `ChangeSet`-based return with direct `clear_mask` calls.

---

## 6. `update(r, c, bit)` — central dispatch

```rust
fn update(&mut self, r: usize, c: usize, bit: u64) {
    self.update_singleton(r, c, bit);
    self.update_hidden_singles(r, c, bit);
    self.update_black_consistency(r, c, bit);
    self.update_arc(r, c, bit);
}
```

Handlers may call `clear_mask`, which enqueues further triples. The queue is not drained inside `update`; that happens in `propagate`.

---

## 7. `update_singleton`

```
if bit & (ALL_DIGITS | ROW_BLACKS) != 0:
    row_domain_size[r][c] -= 1
    if row_domain_size[r][c] == 1:
        let row_domain = domains[r][c] & (ALL_DIGITS | ROW_BLACKS)
        set_cell(r, c, row_domain)   // exactly one bit remains; pass it directly

if bit & (ALL_DIGITS | COL_BLACKS) != 0:
    col_domain_size[r][c] -= 1
    if col_domain_size[r][c] == 1:
        let col_domain = domains[r][c] & (ALL_DIGITS | COL_BLACKS)
        set_cell(r, c, col_domain)
```

If the cell is already determined, `set_cell` on a single-bit domain is a no-op (all `clear_mask` calls will be no-ops).

---

## 8. `update_hidden_singles`

```
let p = bit.trailing_zeros() as usize;

if bit & (ALL_DIGITS | ROW_BLACKS) != 0:
    row_candidates[r][p] -= 1
    if row_candidates[r][p] == 1:
        let c2 = scan row r for the cell still holding bit
        set_cell(r, c2, bit)

if bit & (ALL_DIGITS | COL_BLACKS) != 0:
    col_candidates[c][p] -= 1
    if col_candidates[c][p] == 1:
        let r2 = scan col c for the cell still holding bit
        set_cell(r2, c, bit)
```

Digit bits trigger both branches. The scan for the surviving cell is O(N) — acceptable for N=6.

---

## 9. `update_black_consistency`

```
if bit & ROW_BLACKS != 0:
    row_blacks_left[r][c] -= 1
    if row_blacks_left[r][c] == 0:
        clear_mask(r, c, COL_BLACKS)

if bit & COL_BLACKS != 0:
    col_blacks_left[r][c] -= 1
    if col_blacks_left[r][c] == 0:
        clear_mask(r, c, ROW_BLACKS)
```

---

## 10. `update_arc`

```
fn update_arc(&mut self, r: usize, c: usize, bit: u64) {
    // Row direction
    let mut i = 0;
    while i < live_row[r].len() {
        let T = &live_row[r][i];
        let Some(pos) = pos_of(T, c, N) else { i += 1; continue; };
        if T.pattern[pos] & bit == 0   { i += 1; continue; }  // bit not in pattern here
        if domains[r][c] & T.pattern[pos] != 0 { i += 1; continue; }  // T still alive
        // T is dead: remove and update support counts
        let dead = live_row[r].swap_remove(i);
        for p in 0..dead.pattern.len() {
            let c2 = (dead.start + p) % N;
            let mut bits = dead.pattern[p];
            while bits != 0 {
                let b = bits & bits.wrapping_neg();
                bits &= bits - 1;
                let idx = b.trailing_zeros() as usize;
                support_row[r][c2][idx] -= 1;
                if support_row[r][c2][idx] == 0 {
                    clear_mask(r, c2, b);
                }
            }
        }
        // Do NOT increment i: swap_remove moved the last element to position i.
    }

    // Column direction: symmetric, using live_col[c] and support_col.
}
```

---

## 11. `propagate`

```rust
pub fn propagate(&mut self) {
    while let Some((r, c, bit)) = self.queue.pop_front() {
        self.update(r, c, bit);
    }
}
```

---

## 12. `is_contradiction`, `is_solved`, `pick_branching_cell`

Copy directly from `SolverState`. These only inspect `domains`.

---

## 13. `count_solutions`

Copy directly from `SolverState`. The cloning approach for backtracking works unchanged: all new fields (`Vec`s of live tuples, counter arrays, the queue) are `Clone`. The queue is empty at branch points (propagation runs to fixpoint first), so the clone captures a clean state.

---

## 14. Order of work

1. Define `LiveTuple` and its helpers (`covers`, `pos_of`).
2. Define `QueueSolverState` with all fields (stub methods).
3. Implement `clear_mask` and `set_cell`.
4. Implement `new()` in phases: tuples → support counts → counters from full domain → seed queue → propagate.
5. Implement `update_singleton`.
6. Implement `update_hidden_singles`.
7. Implement `update_black_consistency`.
8. Implement `update_arc`.
9. Implement `update` (dispatch) and `propagate`.
10. Implement `is_contradiction`, `is_solved`, `pick_branching_cell`, `count_solutions`.
11. Wire up a binary that runs both solvers on the same puzzle and asserts agreement.

Steps 5–8 can be developed and tested independently: after each one, run a known puzzle through `propagate` and compare domains against the original solver's output.

---

## 15. Open design questions (resolved)

| Question | Decision |
|---|---|
| Work queue deduplication | Not needed: each bit is enqueued at most once, since domains only shrink and `clear_mask` only enqueues on genuine shrinkage. |
| Queue entry granularity | Individual bits, not masks. |
| Singleton — how to find the remaining bit | When counter reaches 1, `domains[r][c] & row_view` is already a single-bit value; pass it directly to `set_cell`. |
| Digit bits and hidden-singles counters | Decrement both `row_candidates` and `col_candidates`. |
| Tuple `wrap` field | Omitted; wrap is implicit from `start + pattern.len() > N`. |
| Tuple `len` field | Omitted; implicit as `pattern.len() - 2`. |
| Live-tuple tracking variant | Variant A (scan). |
| Liveness check strength | Condition A only (per-cell). The live set is an over-approximation; see ROADMAP.md. |
| Counter initialization timing | From the **full** domain, before any `clear_mask` call — see counter invariant in §4. |
| Backtracking | Clone the entire `QueueSolverState`. Queue is empty at branch points. |
