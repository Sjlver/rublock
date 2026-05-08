# Rublock — Specs

This folder contains concise descriptions of the major subsystems. The goal is
to give a quick orientation without having to read the code. Each file covers
one area at the level of *what* it does and *why* it is structured that way;
implementation details live in the source.

## The Project

**Rublock** is a solver and puzzle generator for **Doplo**, a logic puzzle
published by [Küng Rätsel](https://doplo.ch).

### Doplo rules

The puzzle is played on an N×N grid (default 6×6). Each row and column has a
**target number** attached.

1. Each row and each column must contain exactly **two black squares**.
2. The remaining N−2 squares in each row/column must contain the digits
   **1 … N−2**, each exactly once.
3. The **sum of the digits between the two black squares** must equal that
   row/column's target.

### Codebase map

| Path | What it is |
|---|---|
| `src/` | Rust library + CLI binary |
| `src/bin/` | Additional binaries: `gen_puzzle`, `enumerate`, `compare` |
| `web/` | Vite + Svelte 5 + TypeScript web app |
| `pkg/` | WASM build output (generated; not hand-edited) |
| `benches/` | Criterion benchmarks |
| `doc/spec/` | This folder |

## Files in this folder

- [`black_solver.md`](black_solver.md) — the default constraint-propagation solver
- [`web_interface.md`](web_interface.md) — the Svelte 5 web app
- [`binaries.md`](binaries.md) — CLI binaries (`rublock`, `gen_puzzle`, `enumerate`, `compare`)

## Features under development

*(Add entries here when starting a non-trivial feature. Remove or archive them
once the feature ships.)*

<!-- Example entry:
### Undo/redo in the web UI
**Status:** in progress
**Goal:** let players undo and redo cell entries in the Play tab.
**Approach:** maintain a stack of `CellOperation` values in `playState`; expose
`undo()` / `redo()` helpers; wire up keyboard shortcuts (Ctrl+Z / Ctrl+Y).
-->
