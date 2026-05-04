use rand::seq::SliceRandom;
use serde::Serialize;
use wasm_bindgen::prelude::*;

use crate::black_solver::BlackSolverState;
use crate::grid::{Cell, Grid};
use crate::recorder::{Explain, Rule, Step};
use crate::solver::{Puzzle, SolveOutcome, Solver};

// ── Response shapes (mirror web/src/state/types.ts) ──────────────────────────

#[derive(Serialize)]
struct PuzzleResp<'a> {
    size: usize,
    row_targets: &'a [u8],
    col_targets: &'a [u8],
}

#[derive(Serialize)]
struct SolvedResp<'a> {
    size: usize,
    row_targets: &'a [u8],
    col_targets: &'a [u8],
    cells: Vec<Vec<CellOut>>,
}

#[derive(Serialize)]
struct ExplainResp<'a> {
    size: usize,
    row_targets: &'a [u8],
    col_targets: &'a [u8],
    cells: Vec<Vec<CellOut>>,
    steps: Vec<StepOut>,
}

/// `number | "black"` — matches the TS `CellValue` union.
#[derive(Serialize)]
#[serde(untagged)]
enum CellOut {
    Number(u8),
    Black(&'static str),
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

fn js_err(msg: &str) -> JsValue {
    JsValue::from_str(msg)
}

fn to_js<T: Serialize>(v: &T) -> Result<JsValue, JsValue> {
    serde_wasm_bindgen::to_value(v).map_err(|e| js_err(&e.to_string()))
}

fn cells_out<const N: usize>(cells: &[[i8; N]; N]) -> Vec<Vec<CellOut>> {
    cells
        .iter()
        .map(|row| {
            row.iter()
                .map(|&v| {
                    if v < 0 {
                        CellOut::Black("black")
                    } else {
                        CellOut::Number(v as u8)
                    }
                })
                .collect()
        })
        .collect()
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
                size: N,
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
    let row_targets: [u8; N] = row_targets
        .try_into()
        .map_err(|_| js_err("row_targets length does not match puzzle size"))?;
    let col_targets: [u8; N] = col_targets
        .try_into()
        .map_err(|_| js_err("col_targets length does not match puzzle size"))?;

    let puzzle = Puzzle::<N>::new(row_targets, col_targets);
    let state = BlackSolverState::<N>::new(puzzle.clone());
    match state.solve() {
        SolveOutcome::Unsolvable => Err(js_err("puzzle is unsolvable")),
        SolveOutcome::Multiple(_) => Err(js_err("puzzle has multiple solutions")),
        SolveOutcome::Unique(solved) => {
            let cells = solved
                .solved_cells()
                .ok_or_else(|| js_err("solver returned an incomplete state"))?;
            to_js(&SolvedResp {
                size: N,
                row_targets: &puzzle.row_targets,
                col_targets: &puzzle.col_targets,
                cells: cells_out(&cells),
            })
        }
    }
}

fn explain_puzzle_n<const N: usize>(
    row_targets: Vec<u8>,
    col_targets: Vec<u8>,
) -> Result<JsValue, JsValue> {
    let row_targets: [u8; N] = row_targets
        .try_into()
        .map_err(|_| js_err("row_targets length does not match puzzle size"))?;
    let col_targets: [u8; N] = col_targets
        .try_into()
        .map_err(|_| js_err("col_targets length does not match puzzle size"))?;

    let puzzle = Puzzle::<N>::new(row_targets, col_targets);
    let state = BlackSolverState::<N, Explain>::with_recorder(puzzle.clone());
    match state.solve() {
        SolveOutcome::Unsolvable => Err(js_err("puzzle is unsolvable")),
        SolveOutcome::Multiple(_) => Err(js_err("puzzle has multiple solutions")),
        SolveOutcome::Unique(solved) => {
            let cells = solved
                .solved_cells()
                .ok_or_else(|| js_err("solver returned an incomplete state"))?;
            let steps = state.recorder().steps();
            to_js(&ExplainResp {
                size: N,
                row_targets: &puzzle.row_targets,
                col_targets: &puzzle.col_targets,
                cells: cells_out(&cells),
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
