//! Propagation statistics.
//!
//! These counters help answer questions like "how often do we backtrack?" and
//! "which rule is pulling its weight?".  Most of them live behind
//! `#[cfg(debug_assertions)]`, the same switch that drives `debug_assert!`,
//! so that the rich per-rule breakdown stays out of release builds.
//!
//! The single exception is `search_nodes`: it's always present, because the
//! `gen_puzzle` binary uses it as a difficulty signal and needs release-mode
//! speed to find hard puzzles in reasonable time.  In a `--release` build:
//!
//! - `Stats` has just the one `search_nodes: u64` field.
//! - `StatsHandle` holds an `Rc<Cell<Stats>>` (one allocation per top-level
//!   solve; clones during backtracking just bump the refcount).
//! - The per-rule `incr_bits` path is still empty and inlined away, so the
//!   hot per-bit accounting in `clear_mask` costs nothing in release.
//!
//! ## Why `Rc<Cell<Stats>>`?
//!
//! The backtracking search clones solver state at every branch.  We want
//! counters that aggregate across the whole search tree, not per-branch.  So
//! the state holds a **handle**, and cloning the handle just bumps a
//! reference count — every branch ends up pointing at the same counters.
//!
//! - `Rc<T>` — multiple owners of the same heap value.  `Clone` is cheap; the
//!   value is dropped when the last handle goes out of scope.  Single-threaded
//!   (use `Arc` across threads).
//! - `Cell<T>` — lets us mutate through a `&Cell<T>` without breaking the
//!   aliasing rule, because we only ever `get`/`set` by value (no references
//!   handed out).  Requires `T: Copy` for `get`, which is fine here.
//!
//! Together, `Rc<Cell<Stats>>` means "many shared handles, each able to tick
//! the counters".

/// Which propagation rule (or pseudo-rule) cleared a bit.
///
/// The first four variants mirror the four `apply_*` rules on the solver.
/// `Backtracking` is a pseudo-rule: bits removed by the backtracking search
/// when committing to a guess or pruning its complement aren't the work of
/// any propagation rule, so we tally them separately.
///
/// In release builds this enum is unused and Rust may still compile it in, but
/// nothing reads it and the optimizer discards it.
#[derive(Copy, Clone, Debug)]
pub enum Rule {
    ArcConsistency,
    Singleton,
    HiddenSingle,
    BlackConsistency,
    Backtracking,
}

/// Tallies collected during one solve.  `search_nodes` is always present;
/// the per-rule `bits_*` fields are debug-only.
#[derive(Default, Clone, Copy, Debug)]
pub struct Stats {
    pub search_nodes: u64,
    #[cfg(debug_assertions)]
    pub bits_arc_consistency: u64,
    #[cfg(debug_assertions)]
    pub bits_singleton: u64,
    #[cfg(debug_assertions)]
    pub bits_hidden_single: u64,
    #[cfg(debug_assertions)]
    pub bits_black_consistency: u64,
    #[cfg(debug_assertions)]
    pub bits_backtracking: u64,
}

/// A shared handle to a `Stats` counter.
///
/// Clone is cheap (bumps the `Rc` refcount) and every clone points at the
/// same underlying counters, so incrementing from any branch of the
/// backtracking search updates a single shared tally.
#[derive(Clone, Default)]
pub struct StatsHandle(std::rc::Rc<std::cell::Cell<Stats>>);

impl StatsHandle {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record that a rule removed `n` bits from some cell's domain.
    #[inline]
    #[cfg(debug_assertions)]
    pub fn incr_bits(&self, rule: Rule, n: u32) {
        let mut s = self.0.get();
        let field = match rule {
            Rule::ArcConsistency => &mut s.bits_arc_consistency,
            Rule::Singleton => &mut s.bits_singleton,
            Rule::HiddenSingle => &mut s.bits_hidden_single,
            Rule::BlackConsistency => &mut s.bits_black_consistency,
            Rule::Backtracking => &mut s.bits_backtracking,
        };
        *field += n as u64;
        self.0.set(s);
    }

    #[inline]
    #[cfg(not(debug_assertions))]
    pub fn incr_bits(&self, _rule: Rule, _n: u32) {}

    /// Record one search-tree node (one entry into the recursive solve).
    #[inline]
    pub fn incr_node(&self) {
        let mut s = self.0.get();
        s.search_nodes += 1;
        self.0.set(s);
    }

    /// Return a copy of the current tallies.  Cheap: `Stats: Copy`.
    pub fn snapshot(&self) -> Stats {
        self.0.get()
    }
}

impl std::fmt::Display for Stats {
    #[cfg(debug_assertions)]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "search nodes:       {}", self.search_nodes)?;
        writeln!(f, "bits removed:")?;
        writeln!(f, "  arc-consistency:   {}", self.bits_arc_consistency)?;
        writeln!(f, "  singleton:         {}", self.bits_singleton)?;
        writeln!(f, "  hidden-single:     {}", self.bits_hidden_single)?;
        writeln!(f, "  black-consistency: {}", self.bits_black_consistency)?;
        write!(f, "  backtracking:      {}", self.bits_backtracking)
    }

    #[cfg(not(debug_assertions))]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "search nodes: {}", self.search_nodes)
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn handle_clones_share_the_same_counters() {
        let a = StatsHandle::new();
        let b = a.clone();
        a.incr_bits(Rule::Singleton, 3);
        b.incr_bits(Rule::Singleton, 4);
        // Both handles see the combined total.
        #[cfg(debug_assertions)]
        {
            assert_eq!(a.snapshot().bits_singleton, 7);
            assert_eq!(b.snapshot().bits_singleton, 7);
        }
    }

    #[test]
    fn incr_node_counts_calls() {
        let h = StatsHandle::new();
        h.incr_node();
        h.incr_node();
        h.incr_node();
        assert_eq!(h.snapshot().search_nodes, 3);
    }
}
