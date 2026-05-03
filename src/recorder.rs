//! Propagation event recorders.
//!
//! A solver implementation calls into a [`Recorder`] whenever its propagation
//! does something interesting: a bit is removed from a cell's domain, the
//! search tree branches, or a logical "step" of propagation begins.  The
//! solver doesn't care what the recorder does with these events — different
//! recorders serve different purposes:
//!
//! - [`SearchNodes`] is the cheap default.  It only counts search nodes, which
//!   `gen_puzzle` uses as a difficulty signal in release-mode batches.
//! - [`FullStats`] keeps a per-rule bit-removal breakdown alongside the node
//!   count.  Used by `main.rs` to show what happened during a solve and by
//!   the test suite to assert that each rule pulled its weight.
//! - [`Explain`] (added in `recorder::explain`) records every bit removal
//!   into time-ordered "steps" so the web UI's Explain tab can replay a
//!   propagation pass.
//!
//! Replaces the old `stats.rs` module.  The previous design wrapped the
//! per-rule fields in `#[cfg(debug_assertions)]` so release builds didn't pay
//! for them.  The cost is now confined to the recorder type the caller picks:
//! `SearchNodes` only carries a single `Cell<u64>`, so the per-rule fields
//! aren't even built into the binary unless someone chose `FullStats`.
//!
//! ## Why `Rc<Cell<…>>` (and `Rc<RefCell<…>>` for `Explain`)?
//!
//! The backtracking search clones solver state at every branch.  We want
//! counters that aggregate across the whole search tree, not per-branch.  So
//! the state holds a *handle*, and cloning the handle just bumps the `Rc`
//! refcount — every branch ends up pointing at the same underlying counters.
//! Single-threaded use only (use `Arc` across threads).

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::solver::CellDomain;

// ── Rule ──────────────────────────────────────────────────────────────────────

/// Which propagation rule (or pseudo-rule) cleared a bit.
///
/// `TargetTuples` covers the bulk reduction `seed_queue` does up front in
/// the black/queue solvers: bits with no live tuple support before any
/// propagation has run.  It's distinct from `ArcConsistency`, which fires
/// later during propagation when a tuple's last support disappears.
///
/// `Backtracking` is a pseudo-rule: bits removed by the search loop when
/// committing to a guess or pruning its complement aren't the work of any
/// propagation rule, so we tally them separately.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Rule {
    TargetTuples,
    ArcConsistency,
    Singleton,
    HiddenSingle,
    BlackConsistency,
    Backtracking,
}

// ── Recorder trait ────────────────────────────────────────────────────────────

/// What a solver tells its recorder while it propagates.
///
/// Every method has an empty default body so each recorder implementation
/// only overrides the events it cares about.  The `Clone + Default` bounds
/// match how the recorder is held inside a solver state: cloned across
/// backtracking branches (cheap, refcount bump), and built fresh in each
/// solver's `new()` via `Default`.
pub trait Recorder: Clone + Default {
    /// A bit was cleared from a cell's domain.
    ///
    /// `before` and `after` are the cell's domain immediately before and
    /// after the change; `before & !after` gives the bits that were
    /// removed.  `rule` attributes the removal to a specific propagation
    /// rule (see [`Rule`]).
    #[allow(unused_variables)]
    fn on_bits_removed(
        &self,
        row: usize,
        col: usize,
        before: CellDomain,
        after: CellDomain,
        rule: Rule,
    ) {
    }

    /// Backtracking search created a new search-tree node (initial entry,
    /// branching guess, or its complement).
    fn on_search_node(&self) {}

    /// A new logical propagation step is about to begin.
    ///
    /// Step-aware recorders flush the in-progress step (if any) and start a
    /// fresh one.  Step boundaries are placed by the solvers themselves —
    /// see each solver's `propagate()` for the exact placement.
    fn on_step_start(&self) {}

    /// Number of search-tree nodes seen so far.  Default `0` for recorders
    /// that don't track this.
    fn search_nodes(&self) -> u64 {
        0
    }
}

// ── SearchNodes ───────────────────────────────────────────────────────────────

/// Minimal recorder: only counts search-tree nodes.
///
/// This is the default `Recorder` for every solver state.  It's the cheapest
/// "real" recorder we ship — one `Cell<u64>` increment per branching node and
/// nothing else.  Used by anything that just wants to solve (the main CLI
/// path uses `FullStats` instead, but `gen_puzzle`, `compare`, the wasm
/// `solve_puzzle` entry point, and the bench harness all use this).
#[derive(Clone, Default)]
pub struct SearchNodes(Rc<Cell<u64>>);

impl SearchNodes {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Recorder for SearchNodes {
    #[inline]
    fn on_search_node(&self) {
        self.0.set(self.0.get() + 1);
    }

    fn search_nodes(&self) -> u64 {
        self.0.get()
    }
}

// ── FullStats ─────────────────────────────────────────────────────────────────

/// Per-rule bit-removal breakdown plus a search-node count.
///
/// Used by `main.rs` (which prints these via `Display`) and the per-solver
/// `stats_track_*` tests (which assert that each rule contributed
/// something).
#[derive(Default, Clone, Copy, Debug)]
pub struct Stats {
    pub search_nodes: u64,
    pub bits_target_tuples: u64,
    pub bits_arc_consistency: u64,
    pub bits_singleton: u64,
    pub bits_hidden_single: u64,
    pub bits_black_consistency: u64,
    pub bits_backtracking: u64,
}

#[derive(Clone, Default)]
pub struct FullStats(Rc<Cell<Stats>>);

impl FullStats {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return a copy of the current tallies.  Cheap: `Stats: Copy`.
    pub fn snapshot(&self) -> Stats {
        self.0.get()
    }
}

impl Recorder for FullStats {
    #[inline]
    fn on_bits_removed(
        &self,
        _row: usize,
        _col: usize,
        before: CellDomain,
        after: CellDomain,
        rule: Rule,
    ) {
        let removed = (before & !after).count_ones() as u64;
        if removed == 0 {
            return;
        }
        let mut s = self.0.get();
        let field = match rule {
            Rule::TargetTuples => &mut s.bits_target_tuples,
            Rule::ArcConsistency => &mut s.bits_arc_consistency,
            Rule::Singleton => &mut s.bits_singleton,
            Rule::HiddenSingle => &mut s.bits_hidden_single,
            Rule::BlackConsistency => &mut s.bits_black_consistency,
            Rule::Backtracking => &mut s.bits_backtracking,
        };
        *field += removed;
        self.0.set(s);
    }

    #[inline]
    fn on_search_node(&self) {
        let mut s = self.0.get();
        s.search_nodes += 1;
        self.0.set(s);
    }

    fn search_nodes(&self) -> u64 {
        self.0.get().search_nodes
    }
}

impl std::fmt::Display for Stats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "search nodes:       {}", self.search_nodes)?;
        writeln!(f, "bits removed:")?;
        writeln!(f, "  target tuples:     {}", self.bits_target_tuples)?;
        writeln!(f, "  arc-consistency:   {}", self.bits_arc_consistency)?;
        writeln!(f, "  singleton:         {}", self.bits_singleton)?;
        writeln!(f, "  hidden-single:     {}", self.bits_hidden_single)?;
        writeln!(f, "  black-consistency: {}", self.bits_black_consistency)?;
        write!(f, "  backtracking:      {}", self.bits_backtracking)
    }
}

// ── Explain ───────────────────────────────────────────────────────────────────

/// A single domain-bit removal recorded by [`Explain`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Event {
    pub row: usize,
    pub col: usize,
    pub before: CellDomain,
    pub after: CellDomain,
    pub rule: Rule,
}

/// A group of domain-bit removals that belong to the same logical step.
///
/// One step corresponds to one BFS propagation wave (black/queue solvers), one
/// fixpoint iteration (basic solver), or one branching assignment.  Empty
/// steps — where no bits were actually removed — are discarded before
/// the step list is exposed.
#[derive(Clone, Debug)]
pub struct Step {
    pub events: Vec<Event>,
}

#[derive(Debug, Default)]
struct ExplainLog {
    steps: Vec<Step>,
    current: Vec<Event>,
}

/// Step-by-step propagation recorder.
///
/// Each call to [`Recorder::on_step_start`] closes the in-progress step (if
/// it has any events) and begins a new one.  Empty steps are silently
/// discarded, so the final fixpoint iteration of the basic solver — which
/// makes no changes — never appears in the step list.
///
/// Use [`Explain::steps`] after solving to retrieve the recorded log.
#[derive(Clone, Default)]
pub struct Explain(Rc<RefCell<ExplainLog>>);

impl Explain {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return all recorded steps plus any events still pending in the
    /// current in-progress step.  Does not mutate the recorder.
    pub fn steps(&self) -> Vec<Step> {
        let log = self.0.borrow();
        let mut result = log.steps.clone();
        if !log.current.is_empty() {
            result.push(Step {
                events: log.current.clone(),
            });
        }
        result
    }
}

impl Recorder for Explain {
    fn on_bits_removed(
        &self,
        row: usize,
        col: usize,
        before: CellDomain,
        after: CellDomain,
        rule: Rule,
    ) {
        if (before & !after) == 0 {
            return;
        }
        self.0.borrow_mut().current.push(Event {
            row,
            col,
            before,
            after,
            rule,
        });
    }

    fn on_step_start(&self) {
        let mut log = self.0.borrow_mut();
        if !log.current.is_empty() {
            let events = std::mem::take(&mut log.current);
            log.steps.push(Step { events });
        }
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn search_nodes_clones_share_the_same_counter() {
        let a = SearchNodes::new();
        let b = a.clone();
        a.on_search_node();
        b.on_search_node();
        b.on_search_node();
        assert_eq!(a.search_nodes(), 3);
        assert_eq!(b.search_nodes(), 3);
    }

    #[test]
    fn full_stats_attributes_bits_to_rule() {
        let s = FullStats::new();
        s.on_bits_removed(0, 0, 0b1110, 0b0010, Rule::Singleton); // 2 bits
        s.on_bits_removed(0, 0, 0b1100, 0b1000, Rule::TargetTuples); // 1 bit
        s.on_bits_removed(0, 0, 0b1111, 0b1111, Rule::ArcConsistency); // no-op
        let snap = s.snapshot();
        assert_eq!(snap.bits_singleton, 2);
        assert_eq!(snap.bits_target_tuples, 1);
        assert_eq!(snap.bits_arc_consistency, 0);
    }

    #[test]
    fn full_stats_clones_share_counters() {
        let a = FullStats::new();
        let b = a.clone();
        a.on_search_node();
        b.on_search_node();
        b.on_bits_removed(0, 0, 0b110, 0b010, Rule::HiddenSingle);
        assert_eq!(a.snapshot().search_nodes, 2);
        assert_eq!(a.snapshot().bits_hidden_single, 1);
        assert_eq!(b.snapshot().search_nodes, 2);
    }

    #[test]
    fn explain_groups_events_into_steps() {
        let rec = Explain::new();

        // Step 0: two events, then step boundary.
        rec.on_bits_removed(0, 0, 0b1110, 0b0110, Rule::TargetTuples);
        rec.on_bits_removed(0, 1, 0b1110, 0b1100, Rule::ArcConsistency);
        rec.on_step_start(); // closes step 0, opens step 1

        // Step 1: one event, then step boundary with nothing new.
        rec.on_bits_removed(1, 0, 0b1111, 0b0111, Rule::Singleton);
        rec.on_step_start(); // closes step 1, opens empty step 2
        rec.on_step_start(); // empty step 2 discarded, opens step 3

        // Step 3 has no events — pending current is empty.
        let steps = rec.steps();
        assert_eq!(steps.len(), 2, "empty steps are not recorded");
        assert_eq!(steps[0].events.len(), 2);
        assert_eq!(steps[0].events[0].rule, Rule::TargetTuples);
        assert_eq!(steps[0].events[1].rule, Rule::ArcConsistency);
        assert_eq!(steps[1].events.len(), 1);
        assert_eq!(steps[1].events[0].rule, Rule::Singleton);
    }

    #[test]
    fn explain_pending_events_included_in_steps() {
        let rec = Explain::new();
        rec.on_bits_removed(0, 0, 0b110, 0b010, Rule::BlackConsistency);
        // No on_step_start called — pending events should appear via steps().
        let steps = rec.steps();
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].events[0].rule, Rule::BlackConsistency);
    }

    #[test]
    fn explain_clones_share_the_same_log() {
        let a = Explain::new();
        let b = a.clone();
        a.on_bits_removed(0, 0, 0b110, 0b010, Rule::Singleton);
        b.on_step_start(); // closes the step with a's event
        b.on_bits_removed(0, 1, 0b111, 0b011, Rule::HiddenSingle);
        // Both handles point at the same log.
        let steps = a.steps();
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].events[0].rule, Rule::Singleton);
        assert_eq!(steps[1].events[0].rule, Rule::HiddenSingle);
    }
}
