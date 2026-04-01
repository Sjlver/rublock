## Plan: Solver Improvements

### Phase 1 — Generalized Arc Consistency for BLACK bits

This replaces `apply_black_range_rules` with a stronger pruning rule that checks whether each BLACK1/BLACK2 bit has actual *tuple-level* support, not just distance-range support.

**Concept recap.** For BLACK1 at position `p` in a row with target `t`, two independent conditions must hold — failure of either lets you remove the BLACK1 bit:

1. **Forward (inside) support:** There must exist some length `l` and a valid tuple in `valid_tuples[t][l]` such that:
   - Cell at `p + l + 1` has BLACK2 in its domain
   - Each of the `l` cells `p+1 .. p+l` has digit-domain intersecting the tuple
2. **Backward (outside, wrapping) support:** There must exist some length `l` and a valid tuple in `valid_tuples[max_sum - t][l]` such that:
   - Cell at `(p - l - 1 + N) % N` has BLACK2 in its domain
   - Each of the `l` cells going backward from `p` (wrapping at edge 0 → N-1) has digit-domain intersecting the tuple

The same logic applies symmetrically to BLACK2 (checking backward for inside, forward-wrapping for outside).

**Steps:**

1. **Write a helper** `has_tuple_support(line: &[CellDomain; N], positions: &[usize], target: usize, anchor_pos: usize, anchor_bit: u64) -> bool` that checks whether the given cell positions' digit-domains are compatible with *any* tuple in `valid_tuples[target][positions.len()]`, and whether `line[anchor_pos]` has `anchor_bit` set.

2. **Write `apply_black_arc_consistency`** that iterates over all cells in each row and column, and for each BLACK1/BLACK2 bit present, checks both forward and backward support using the helper. If either check finds no support, clear the bit. This replaces `apply_black_range_rules` in the `propagate` loop.

3. **Remove `d_min` and `d_max`** from `Tables` — they're no longer needed. Remove the code that computes them in `Tables::build`.

4. **Update tests.** The test `black_range_rule_uses_actual_domain_state` calls `apply_black_range_rules` directly — rename/rewrite it for the new method. The integration tests (`newspaper_puzzle_1/2`) should still pass since the new rule is strictly stronger. Also `apply_inside_outside_rule` and `apply_inside_outside_cage_rule` are still valid rules but the new GAC rule may make some of their work redundant — keep them for now, they still provide value when blacks aren't yet pinned.

5. **Consider making `apply_inside_outside_rule` redundant.** The new GAC rule implicitly checks that the inside and outside digit constraints are satisfiable. The `cant_be_inside` table and `apply_inside_outside_rule` might still catch things the GAC rule doesn't (it prunes *digit* bits, while GAC prunes *black* bits). Keep both for now; you can benchmark later whether removing `apply_inside_outside_rule` changes anything.

**Key implementation detail — wrapping for the outside region.** From BLACK1 at position `p`, the backward-wrapping positions for the outside region are:

```
(p-1) % N, (p-2) % N, ..., (p-l) % N
```

and the anchor BLACK2 is expected at `(p - l - 1 + N) % N`. This naturally handles the fact that the "outside" region wraps around the row boundary.

---

### Phase 2 — Nogood Learning in Backtracking

This is simpler than it sounds, and independent of Phase 1. The change is localized to `count_solutions`.

**Current behavior** in `count_solutions`:

```
for each candidate bit:
    clone state → branch
    set_cell(row, col, bit) on branch
    recursively solve branch
```

If a branch returns 0 solutions, the value is simply skipped and the next candidate is tried.

**Improved behavior:**

```
for each candidate bit (iterate over remaining bits dynamically):
    clone state → branch
    set_cell(row, col, bit) on branch
    sub = recursively solve branch
    if sub == 0:
        remove bit from state.domains[row][col]   // nogood learned
        recurse  -- this will call propagate. It will abandon the current cell; maybe a different cell is the most constrained now.
```

**Steps:**

1. **Restructure the branching loop** in `count_solutions`. Instead of iterating over a fixed `bits` mask, dynamically recompute `branching_bits` after each nogood is learned. This handles the case where re-propagation after removing a failed value causes further domain reductions.

2. **Handle edge cases:**
   - After re-propagation, the current cell might be singleton or empty... simply recursing should handle all this.

3. **Test.** The existing `count_solutions` tests should still pass. You could add a test where nogood learning demonstrably reduces work (e.g., a puzzle where the first guess fails and propagation of the nogood immediately resolves more cells).

---

### Phase 3 — Work Queue / Rule Skipping

This optimizes the `propagate` fixpoint loop to avoid re-running rules on unchanged rows/columns. Implement this last because it benefits from having the final rule set from Phase 1.

**Approach: per-row/per-column changed flags** (as suggested in the roadmap — simpler than a full work queue).

**Steps:**

1. **Add change-tracking to `SolverState`:**
   ```rust
   row_changed: [bool; N],
   col_changed: [bool; N],
   ```
   Initialize all to `true` (everything needs processing initially).

2. **Modify `clear_mask`** (and `set_cell`, which calls it) to set `row_changed[row] = true` and `col_changed[col] = true` whenever a domain actually shrinks. Since `clear_mask` is a static method taking `&mut domains`, you'll need to either:
   - Make it a method on `&mut self` so it can update the flags, or
   - Return the changed info and let the caller update flags, or
   - Pass the flags arrays as additional parameters.

   The cleanest option is probably making `clear_mask` return a `bool` (which it already does) and having a thin wrapper or having the callers set the flags. Actually, since almost all callers are rule methods on `&mut self`, making `clear_mask` a method that also sets the flags is cleanest.

3. **Modify each rule** to skip rows/columns whose flags are `false`. For example, `apply_black_arc_consistency` would skip row `r` if `!row_changed[r]`. But be careful: a rule operating on row `r` might need to be re-run if a *column* that intersects row `r` changed (because a cell in that column had its domain reduced). The safe approach:
   - Each rule operates on a line (row or column).
   - A line needs re-processing if *any* cell in it changed — which means the line's flag OR the flag of any crossing line that had a change.
   - Simplification: at the start of each propagation pass, compute `need_row[r] = row_changed[r] || any(col_changed[c] for c in 0..N)` and similarly for columns. This is conservative but cheap.
   - Actually, the simpler version the roadmap suggests: just use per-row and per-column flags directly. A row rule runs if `row_changed[r]` is true. A column rule runs if `col_changed[c]` is true. At the start of each pass, snapshot and clear the flags. Rules that make changes set new flags. This undershoots slightly (a row rule might miss that a cell in its row changed due to a column rule) but that'll be caught on the next pass when the row gets flagged.

   Wait, that's actually fine because `clear_mask` sets *both* `row_changed` and `col_changed`. So if a column rule changes cell (r,c), it sets `row_changed[r] = true`, which means row `r`'s rules will run on the next pass. The system is self-correcting.

4. **Add cheap precondition checks** where applicable. For example, `apply_black_consistency_rule` can skip a row if all cells already have at most one row-black bit configuration (both row-blacks fully assigned). This is a simple check at the start of the row loop.

5. **Snapshot-and-clear pattern** for the propagation loop:

   ```rust
   pub fn propagate(&mut self) {
       loop {
           let snap_row = self.row_changed;
           let snap_col = self.col_changed;
           self.row_changed = [false; N];
           self.col_changed = [false; N];

           // Run rules, which internally check snap_row/snap_col
           // and set self.row_changed/col_changed for new changes
           self.apply_rule_1(&snap_row, &snap_col);
           self.apply_rule_2(&snap_row, &snap_col);
           // ...

           if !self.row_changed.iter().any(|&x| x)
              && !self.col_changed.iter().any(|&x| x) {
               break;
           }
       }
   }
   ```

   Actually, this gets a bit awkward with the signatures. An alternative: keep the flags on `self` and have rules read/write them directly. At the top of `propagate`, snapshot and clear. Rules check the snapshot to decide what to process, and write directly to `self.row_changed` / `self.col_changed`.

6. **Test.** The existing integration and unit tests should all still pass (the optimization shouldn't change results). You could add a test that verifies the flags are set correctly, but the main validation is that the newspaper puzzles still solve correctly.

---

### Recommended Implementation Order

| Order | Feature | Rationale |
|-------|---------|-----------|
| 1st | **GAC for BLACK bits** (Phase 1) | Core algorithmic improvement; strongest impact on pruning |
| 2nd | **Nogood learning** (Phase 2) | Small, self-contained change to `count_solutions`; independent of Phase 1 |
| 3rd | **Work queue / rule skipping** (Phase 3) | Performance optimization; best done after the rule set is finalized |

Each phase is independently testable and committable. Phase 1 is the most involved (~100-150 lines of new logic replacing ~40 lines), Phase 2 is ~20-30 lines of changes, and Phase 3 is moderate (~50-80 lines of plumbing).
