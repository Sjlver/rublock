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

**Approach: a `ChangeSet` bitset struct** that bundles row and column dirty flags together, so rules can return it and the fixpoint loop can compose them with `|`.

**`ChangeSet` struct.** Wraps two `u64` bitfields — one bit per row index, one per column index. Since N is a const generic and all realistic puzzle sizes are well under 64, `u64` is always sufficient.

```rust
#[derive(Copy, Clone, Default)]
struct ChangeSet {
    rows: u64,
    cols: u64,
}

impl ChangeSet {
    /// Mark every row and column as dirty (use at propagation start).
    fn all(n: usize) -> Self {
        let mask = (1u64 << n) - 1;
        Self { rows: mask, cols: mask }
    }
    fn set_row(&mut self, r: usize) { self.rows |= 1 << r; }
    fn set_col(&mut self, c: usize) { self.cols |= 1 << c; }
    fn has_row(self, r: usize) -> bool { self.rows & (1 << r) != 0 }
    fn has_col(self, c: usize) -> bool { self.cols & (1 << c) != 0 }
    fn any(self) -> bool { self.rows != 0 || self.cols != 0 }

    /// Iterate indices of set bits (set-bit walk: O(k) for k set bits).
    fn iter_rows(self) -> SetBits { SetBits(self.rows) }
    fn iter_cols(self) -> SetBits { SetBits(self.cols) }
}

// Rust operator overloading via std::ops traits — ChangeSet | ChangeSet
// unions both bitfields, enabling `r1 | r2 | r3 | ...` in the loop.
impl std::ops::BitOr for ChangeSet {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self { rows: self.rows | rhs.rows, cols: self.cols | rhs.cols }
    }
}
impl std::ops::BitOrAssign for ChangeSet {
    fn bitor_assign(&mut self, rhs: Self) { self.rows |= rhs.rows; self.cols |= rhs.cols; }
}

// `!changed` → bool, so `if !changed { break; }` works naturally.
impl std::ops::Not for ChangeSet {
    type Output = bool;
    fn not(self) -> bool { !self.any() }
}

/// Iterator over indices of set bits using the trailing_zeros trick.
struct SetBits(u64);
impl Iterator for SetBits {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        if self.0 == 0 { return None; }
        let i = self.0.trailing_zeros() as usize;
        self.0 &= self.0 - 1;   // clear lowest set bit
        Some(i)
    }
}
```

**Steps:**

1. **Define `ChangeSet` and `SetBits`** as above (a small standalone module or at the top of `solver.rs`).

2. **Change each `apply_xxx_rule` signature** to accept a `ChangeSet` (the previous pass's dirty set) and return a `ChangeSet` (what it changed). Rules no longer mutate any shared flag state:

   ```rust
   fn apply_black_arc_consistency(&mut self, prev: ChangeSet) -> ChangeSet {
       let mut changed = ChangeSet::default();
       for r in prev.iter_rows() {
           // process row r; if a domain shrinks at cell (r, c):
           changed.set_row(r);
           changed.set_col(c);
       }
       for c in prev.iter_cols() {
           // process column c; if a domain shrinks at cell (r, c):
           changed.set_row(r);
           changed.set_col(c);
       }
       changed
   }
   ```

   The `prev` snapshot tells the rule which lines were dirty *before* this pass. Changes made by earlier rules in the same pass are already reflected in `self.domains` (since they run sequentially and mutate `&mut self`), but a rule only re-processes lines flagged in `prev` — changes from earlier rules in this pass will be picked up next pass via the returned `ChangeSet`. This is correct fixpoint behavior.

3. **Rewrite the `propagate` loop** to compose rule results with `|`:

   ```rust
   pub fn propagate(&mut self) {
       let mut changed = ChangeSet::all(N);
       loop {
           changed = self.apply_black_arc_consistency(changed)
               | self.apply_inside_outside_rule(changed)
               | self.apply_black_consistency_rule(changed)
               | self.apply_singleton_rule(changed)
               | self.apply_hidden_single_rule(changed)
               | self.apply_inside_outside_cage_rule(changed);
           if !changed { break; }
       }
   }
   ```

   No snapshot-and-clear bookkeeping needed on `self` — the returned `ChangeSet` *is* the snapshot for the next pass.

4. **Update `clear_mask`** to return a `ChangeSet` instead of `bool`. It already knows the row and column of the cell being modified, so it can construct the right `ChangeSet` directly when a domain actually shrinks, and return an empty one otherwise. Call sites then use `changed |= clear_mask(...)`, which composes cleanly with the rest of the rule accumulation pattern.

5. **Add cheap precondition checks** where applicable. For example, `apply_black_consistency_rule` can skip a row if all cells already have at most one row-black bit configuration. This is a simple check at the start of the row loop inside `iter_rows`.

6. **Test.** The existing integration and unit tests should all still pass (the optimization shouldn't change results). You could add a unit test for `ChangeSet` / `SetBits` (set, iterate, `|`, `!`) and verify the newspaper puzzles still solve correctly as an integration smoke-test.

---

### Recommended Implementation Order

| Order | Feature | Rationale |
|-------|---------|-----------|
| 1st | **GAC for BLACK bits** (Phase 1) | Core algorithmic improvement; strongest impact on pruning |
| 2nd | **Nogood learning** (Phase 2) | Small, self-contained change to `count_solutions`; independent of Phase 1 |
| 3rd | **Work queue / rule skipping** (Phase 3) | Performance optimization; best done after the rule set is finalized |

Each phase is independently testable and committable. Phase 1 is the most involved (~100-150 lines of new logic replacing ~40 lines), Phase 2 is ~20-30 lines of changes, and Phase 3 is moderate (~50-80 lines of plumbing).
