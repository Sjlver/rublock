use rand::seq::SliceRandom;
use wasm_bindgen::prelude::*;

use crate::grid::{Cell, Grid};
use crate::queue_solver::QueueSolverState;
use crate::solver::Puzzle;

#[wasm_bindgen]
pub fn generate_puzzle(size: u32) -> String {
    match size {
        5 => generate_puzzle_n::<5>(),
        6 => generate_puzzle_n::<6>(),
        7 => generate_puzzle_n::<7>(),
        8 => generate_puzzle_n::<8>(),
        _ => r#"{"error":"size must be 5–8"}"#.to_string(),
    }
}

fn generate_puzzle_n<const N: usize>() -> String {
    let mut rng = rand::rng();
    loop {
        let mut cells = [[Cell::Empty; N]; N];
        let Some(grid) = dfs::<N>(&mut cells, 0, &mut rng) else {
            continue;
        };
        let (row_targets, col_targets) = grid.compute_targets();
        let puzzle = Puzzle::new(row_targets, col_targets);
        let mut st = QueueSolverState::<N>::new(puzzle);
        st.propagate();
        if st.is_solved() {
            return to_json::<N>(&row_targets, &col_targets);
        }
    }
}

fn to_json<const N: usize>(row_targets: &[u8; N], col_targets: &[u8; N]) -> String {
    let rows: Vec<String> = row_targets.iter().map(|n| n.to_string()).collect();
    let cols: Vec<String> = col_targets.iter().map(|n| n.to_string()).collect();
    format!(
        r#"{{"size":{},"row_targets":[{}],"col_targets":[{}]}}"#,
        N,
        rows.join(","),
        cols.join(",")
    )
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
