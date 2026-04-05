/// A pair of dirty-line bitsets — one bit per row index, one per column index.
///
/// Rules receive a `ChangeSet` indicating which lines were dirty last pass, and
/// return one indicating which lines they touched this pass. The propagation
/// loop composes the results with `|` and passes the union to the next pass,
/// skipping unchanged rows and columns entirely.
///
/// `u64` is always large enough: all realistic puzzle sizes are well under 64.
#[derive(Copy, Clone, Default)]
pub struct ChangeSet {
    pub rows: u64,
    pub cols: u64,
}

impl ChangeSet {
    /// Mark every row and column as dirty — use this at the start of propagation.
    pub fn all(n: usize) -> Self {
        let mask = (1u64 << n) - 1;
        Self {
            rows: mask,
            cols: mask,
        }
    }

    pub fn set_row(&mut self, r: usize) {
        self.rows |= 1 << r;
    }
    pub fn set_col(&mut self, c: usize) {
        self.cols |= 1 << c;
    }

    pub fn any(self) -> bool {
        self.rows != 0 || self.cols != 0
    }

    /// Iterate indices of set row bits.
    pub fn iter_rows(self) -> SetBits {
        SetBits(self.rows)
    }
    /// Iterate indices of set column bits.
    pub fn iter_cols(self) -> SetBits {
        SetBits(self.cols)
    }
}

impl std::ops::BitOr for ChangeSet {
    type Output = Self;
    fn bitor(self, rhs: Self) -> Self {
        Self {
            rows: self.rows | rhs.rows,
            cols: self.cols | rhs.cols,
        }
    }
}

impl std::ops::BitOrAssign for ChangeSet {
    fn bitor_assign(&mut self, rhs: Self) {
        self.rows |= rhs.rows;
        self.cols |= rhs.cols;
    }
}

/// Iterator over indices of set bits, using the trailing-zeros trick.
pub struct SetBits(pub u64);

impl Iterator for SetBits {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        if self.0 == 0 {
            return None;
        }
        let i = self.0.trailing_zeros() as usize;
        self.0 &= self.0 - 1; // clear lowest set bit
        Some(i)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changeset_set_and_iterate_rows() {
        let mut cs = ChangeSet::default();
        cs.set_row(0);
        cs.set_row(3);
        cs.set_row(5);
        let rows: Vec<usize> = cs.iter_rows().collect();
        assert_eq!(rows, vec![0, 3, 5]);
    }

    #[test]
    fn changeset_set_and_iterate_cols() {
        let mut cs = ChangeSet::default();
        cs.set_col(1);
        cs.set_col(4);
        let cols: Vec<usize> = cs.iter_cols().collect();
        assert_eq!(cols, vec![1, 4]);
    }

    #[test]
    fn changeset_bitor_unions_both_fields() {
        let mut a = ChangeSet::default();
        a.set_row(1);
        a.set_col(2);
        let mut b = ChangeSet::default();
        b.set_row(3);
        b.set_col(4);
        let c = a | b;
        assert_eq!(c.iter_rows().collect::<Vec<_>>(), vec![1, 3]);
        assert_eq!(c.iter_cols().collect::<Vec<_>>(), vec![2, 4]);
    }

    #[test]
    fn changeset_any_is_false_when_empty() {
        let empty = ChangeSet::default();
        assert!(!empty.any());
    }

    #[test]
    fn changeset_any_is_true_when_nonempty() {
        let mut cs = ChangeSet::default();
        cs.set_row(0);
        assert!(cs.any());
    }

    #[test]
    fn changeset_all_sets_every_bit_up_to_n() {
        let cs = ChangeSet::all(6);
        let rows: Vec<usize> = cs.iter_rows().collect();
        let cols: Vec<usize> = cs.iter_cols().collect();
        assert_eq!(rows, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(cols, vec![0, 1, 2, 3, 4, 5]);
    }
}
