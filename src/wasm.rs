use rand::seq::SliceRandom;
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::black_solver::BlackSolverState;
use crate::classify::{self, ClassifyVariant};
use crate::grid::{Cell, Grid};
use crate::recorder::{Explain, Rule, Step};
use crate::solver::{Puzzle, SolveOutcome, Solver};

// ── Response shapes (mirror web/src/state/types.ts) ──────────────────────────

#[derive(Serialize)]
struct PuzzleResp<'a> {
    row_targets: &'a [u8],
    col_targets: &'a [u8],
}

#[derive(Serialize)]
struct SolvedResp<'a> {
    row_targets: &'a [u8],
    col_targets: &'a [u8],
    cells: Vec<&'a [Cell]>,
}

#[derive(Serialize)]
#[serde(rename_all = "lowercase")]
enum ClassifyVariantOut {
    Unsolvable,
    Unique,
    Multiple,
}

impl From<ClassifyVariant> for ClassifyVariantOut {
    fn from(v: ClassifyVariant) -> Self {
        match v {
            ClassifyVariant::Unsolvable => ClassifyVariantOut::Unsolvable,
            ClassifyVariant::Unique => ClassifyVariantOut::Unique,
            ClassifyVariant::Multiple => ClassifyVariantOut::Multiple,
        }
    }
}

#[derive(Serialize)]
struct ClassifyResp<'a> {
    variant: ClassifyVariantOut,
    search_nodes: u64,
    propagation_waves: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    cells: Option<Vec<&'a [Cell]>>,
}

#[derive(Serialize)]
struct ExplainResp<'a> {
    row_targets: &'a [u8],
    col_targets: &'a [u8],
    cells: Vec<&'a [Cell]>,
    steps: Vec<StepOut>,
}

#[derive(Serialize)]
struct StepOut {
    events: Vec<EventOut>,
}

#[derive(Serialize)]
struct EventOut {
    row: usize,
    col: usize,
    before: u64,
    after: u64,
    rule: RuleOut,
}

/// Mirrors `Rule` but with `Serialize` derived. Kept here so `recorder.rs`
/// stays free of serde.
#[derive(Serialize)]
enum RuleOut {
    TargetTuples,
    ArcConsistency,
    Singleton,
    HiddenSingle,
    BlackConsistency,
    Backtracking,
}

impl From<Rule> for RuleOut {
    fn from(r: Rule) -> Self {
        match r {
            Rule::TargetTuples => RuleOut::TargetTuples,
            Rule::ArcConsistency => RuleOut::ArcConsistency,
            Rule::Singleton => RuleOut::Singleton,
            Rule::HiddenSingle => RuleOut::HiddenSingle,
            Rule::BlackConsistency => RuleOut::BlackConsistency,
            Rule::Backtracking => RuleOut::Backtracking,
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

/// Install a panic hook that turns Rust panics into JavaScript exceptions
/// with a useful message. Without it, a panic surfaces as an opaque
/// `RuntimeError: unreachable executed` and the wasm instance is poisoned.
/// The hook is a defense-in-depth measure — every external input is already
/// validated at the boundary, but if a solver bug triggers a panic we'd
/// rather show the user a real error than a frozen UI.
#[wasm_bindgen(start)]
pub fn _wasm_start() {
    console_error_panic_hook::set_once();
}

fn js_err(msg: &str) -> JsValue {
    JsValue::from_str(msg)
}

fn to_js<T: Serialize>(v: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(v).map_err(|e| js_err(&e.to_string()))
}

fn cells_out<const N: usize>(grid: &Grid<N>) -> Vec<&[Cell]> {
    grid.cells.iter().map(|row| &row[..]).collect()
}

fn steps_out(steps: &[Step]) -> Vec<StepOut> {
    steps
        .iter()
        .map(|s| StepOut {
            events: s
                .events
                .iter()
                .map(|e| EventOut {
                    row: e.row,
                    col: e.col,
                    before: e.before,
                    after: e.after,
                    rule: e.rule.into(),
                })
                .collect(),
        })
        .collect()
}

fn try_puzzle<const N: usize>(
    row_targets: Vec<u8>,
    col_targets: Vec<u8>,
) -> Result<Puzzle<N>, JsValue> {
    let row_targets: [u8; N] = row_targets
        .try_into()
        .map_err(|_| js_err("row_targets length does not match puzzle size"))?;
    let col_targets: [u8; N] = col_targets
        .try_into()
        .map_err(|_| js_err("col_targets length does not match puzzle size"))?;
    Puzzle::<N>::try_new(row_targets, col_targets).map_err(|e| js_err(&e))
}

// ── Exports ─────────────────────────────────────────────────────────────────

#[wasm_bindgen]
pub fn generate_puzzle(size: u32) -> Result<JsValue, JsValue> {
    match size {
        5 => generate_puzzle_n::<5>(),
        6 => generate_puzzle_n::<6>(),
        7 => generate_puzzle_n::<7>(),
        8 => generate_puzzle_n::<8>(),
        _ => Err(js_err("size must be 5–8")),
    }
}

/// Generate a puzzle whose classification (search-node and propagation-wave
/// counts) falls inside the given inclusive windows. The Play tab's
/// difficulty dropdown maps each label (Easy … Extremely hard) to a window
/// and calls this; `u32::MAX` means "no upper bound".
///
/// The fast path in `generate_puzzle` only ever emits propagation-only
/// puzzles (it stops at the first random grid where `propagate()` finishes),
/// so the harder buckets need the full classify-and-filter loop here.
#[wasm_bindgen]
pub fn generate_puzzle_with_constraints(
    size: u32,
    min_nodes: u32,
    max_nodes: u32,
    min_waves: u32,
    max_waves: u32,
) -> Result<JsValue, JsValue> {
    match size {
        5 => generate_constrained_n::<5>(min_nodes, max_nodes, min_waves, max_waves),
        6 => generate_constrained_n::<6>(min_nodes, max_nodes, min_waves, max_waves),
        7 => generate_constrained_n::<7>(min_nodes, max_nodes, min_waves, max_waves),
        8 => generate_constrained_n::<8>(min_nodes, max_nodes, min_waves, max_waves),
        _ => Err(js_err("size must be 5–8")),
    }
}

#[wasm_bindgen]
pub fn explain_puzzle(row_targets: Vec<u8>, col_targets: Vec<u8>) -> Result<JsValue, JsValue> {
    if row_targets.len() != col_targets.len() {
        return Err(js_err(
            "row_targets and col_targets must have the same length",
        ));
    }
    match row_targets.len() {
        5 => explain_puzzle_n::<5>(row_targets, col_targets),
        6 => explain_puzzle_n::<6>(row_targets, col_targets),
        7 => explain_puzzle_n::<7>(row_targets, col_targets),
        8 => explain_puzzle_n::<8>(row_targets, col_targets),
        _ => Err(js_err("size must be 5–8")),
    }
}

#[wasm_bindgen]
pub fn solve_puzzle(row_targets: Vec<u8>, col_targets: Vec<u8>) -> Result<JsValue, JsValue> {
    if row_targets.len() != col_targets.len() {
        return Err(js_err(
            "row_targets and col_targets must have the same length",
        ));
    }
    match row_targets.len() {
        5 => solve_puzzle_n::<5>(row_targets, col_targets),
        6 => solve_puzzle_n::<6>(row_targets, col_targets),
        7 => solve_puzzle_n::<7>(row_targets, col_targets),
        8 => solve_puzzle_n::<8>(row_targets, col_targets),
        _ => Err(js_err("size must be 5–8")),
    }
}

#[wasm_bindgen]
pub fn classify_puzzle(row_targets: Vec<u8>, col_targets: Vec<u8>) -> Result<JsValue, JsValue> {
    if row_targets.len() != col_targets.len() {
        return Err(js_err(
            "row_targets and col_targets must have the same length",
        ));
    }
    match row_targets.len() {
        5 => classify_puzzle_n::<5>(row_targets, col_targets),
        6 => classify_puzzle_n::<6>(row_targets, col_targets),
        7 => classify_puzzle_n::<7>(row_targets, col_targets),
        8 => classify_puzzle_n::<8>(row_targets, col_targets),
        _ => Err(js_err("size must be 5–8")),
    }
}

fn generate_constrained_n<const N: usize>(
    min_nodes: u32,
    max_nodes: u32,
    min_waves: u32,
    max_waves: u32,
) -> Result<JsValue, JsValue> {
    let min_nodes = min_nodes as u64;
    let max_nodes = max_nodes as u64;
    let min_waves = min_waves as u64;
    let max_waves = max_waves as u64;
    let mut rng = rand::rng();
    loop {
        let mut cells = [[Cell::Empty; N]; N];
        let Some(grid) = dfs::<N>(&mut cells, 0, &mut rng) else {
            continue;
        };
        let (row_targets, col_targets) = grid.compute_targets();
        let puzzle = Puzzle::new(row_targets, col_targets);
        let result = classify::classify::<N>(puzzle);
        if !matches!(result.variant, ClassifyVariant::Unique) {
            continue;
        }
        if result.search_nodes < min_nodes
            || result.search_nodes > max_nodes
            || result.propagation_waves < min_waves
            || result.propagation_waves > max_waves
        {
            continue;
        }
        return to_js(&PuzzleResp {
            row_targets: &row_targets,
            col_targets: &col_targets,
        });
    }
}

fn generate_puzzle_n<const N: usize>() -> Result<JsValue, JsValue> {
    let mut rng = rand::rng();
    loop {
        let mut cells = [[Cell::Empty; N]; N];
        let Some(grid) = dfs::<N>(&mut cells, 0, &mut rng) else {
            continue;
        };
        let (row_targets, col_targets) = grid.compute_targets();
        let puzzle = Puzzle::new(row_targets, col_targets);
        let mut st = BlackSolverState::<N>::new(puzzle);
        st.propagate();
        if st.is_solved() {
            return to_js(&PuzzleResp {
                row_targets: &row_targets,
                col_targets: &col_targets,
            });
        }
    }
}

fn solve_puzzle_n<const N: usize>(
    row_targets: Vec<u8>,
    col_targets: Vec<u8>,
) -> Result<JsValue, JsValue> {
    let puzzle = try_puzzle::<N>(row_targets, col_targets)?;
    let state = BlackSolverState::<N>::new(puzzle.clone());
    match state.solve() {
        SolveOutcome::Unsolvable => Err(js_err("puzzle is unsolvable")),
        SolveOutcome::Multiple(_) => Err(js_err("puzzle has multiple solutions")),
        SolveOutcome::Unique(solved) => {
            let grid = solved
                .solved_cells()
                .ok_or_else(|| js_err("solver returned an incomplete state"))?;
            to_js(&SolvedResp {
                row_targets: &puzzle.row_targets,
                col_targets: &puzzle.col_targets,
                cells: cells_out(&grid),
            })
        }
    }
}

fn classify_puzzle_n<const N: usize>(
    row_targets: Vec<u8>,
    col_targets: Vec<u8>,
) -> Result<JsValue, JsValue> {
    let puzzle = try_puzzle::<N>(row_targets, col_targets)?;
    let result = classify::classify::<N>(puzzle.clone());
    to_js(&ClassifyResp {
        variant: result.variant.into(),
        search_nodes: result.search_nodes,
        propagation_waves: result.propagation_waves,
        cells: result.cells.as_ref().map(cells_out),
    })
}

fn explain_puzzle_n<const N: usize>(
    row_targets: Vec<u8>,
    col_targets: Vec<u8>,
) -> Result<JsValue, JsValue> {
    let puzzle = try_puzzle::<N>(row_targets, col_targets)?;
    let state = BlackSolverState::<N, Explain>::with_recorder(puzzle.clone());
    match state.solve() {
        SolveOutcome::Unsolvable => Err(js_err("puzzle is unsolvable")),
        SolveOutcome::Multiple(_) => Err(js_err("puzzle has multiple solutions")),
        SolveOutcome::Unique(solved) => {
            let grid = solved
                .solved_cells()
                .ok_or_else(|| js_err("solver returned an incomplete state"))?;
            let steps = state.recorder().steps();
            to_js(&ExplainResp {
                row_targets: &puzzle.row_targets,
                col_targets: &puzzle.col_targets,
                cells: cells_out(&grid),
                steps: steps_out(&steps),
            })
        }
    }
}

fn dfs<const N: usize>(
    cells: &mut [[Cell; N]; N],
    pos: usize,
    rng: &mut impl rand::Rng,
) -> Option<Grid<N>> {
    if pos == N * N {
        return Some(Grid { cells: *cells });
    }

    let row = pos / N;
    let col = pos % N;

    let row_blacks = (0..col).filter(|&c| cells[row][c] == Cell::Black).count();
    let col_blacks = (0..row).filter(|&r| cells[r][col] == Cell::Black).count();
    let row_digit_mask: u64 = (0..col)
        .filter_map(|c| {
            if let Cell::Number(n) = cells[row][c] {
                Some(1u64 << n)
            } else {
                None
            }
        })
        .fold(0, |a, b| a | b);
    let col_digit_mask: u64 = (0..row)
        .filter_map(|r| {
            if let Cell::Number(n) = cells[r][col] {
                Some(1u64 << n)
            } else {
                None
            }
        })
        .fold(0, |a, b| a | b);

    let digits = (N - 2) as u8;
    let mut candidates: Vec<Cell> = std::iter::once(Cell::Black)
        .chain((1..=digits).map(Cell::Number))
        .filter(|&c| match c {
            Cell::Black => row_blacks < 2 && col_blacks < 2,
            Cell::Number(d) => {
                let bit = 1u64 << d;
                row_digit_mask & bit == 0 && col_digit_mask & bit == 0
            }
            Cell::Empty => unreachable!(),
        })
        .collect();

    candidates.shuffle(rng);

    for candidate in candidates {
        cells[row][col] = candidate;
        if let Some(grid) = dfs(cells, pos + 1, rng) {
            return Some(grid);
        }
    }

    cells[row][col] = Cell::Empty;
    None
}
