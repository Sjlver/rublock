## Plan: Solver Improvements

### Work Queue / Rule Skipping

This optimizes the `propagate` fixpoint loop to avoid re-running rules on unchanged rows/columns.

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

1. **Define `ChangeSet` and `SetBits`** as above (a small standalone module `changeset.rs`).

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
