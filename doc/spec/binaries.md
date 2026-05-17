# Binaries

All binaries are part of the `rublock` crate. Build with
`cargo build --release [--bin <name>]` or run directly with `cargo run`.

---

## `rublock` (`src/main.rs`)

The primary CLI. Solves a single puzzle given its targets on the command line.

```
cargo run -- [--solver=basic|queue|black] <row targets...> <col targets...>
```

Targets are 2N numbers: N row targets followed by N column targets (N = 3–11).
Prints the solved grid (or "no solution"), the outcome (unique/multiple), and
solver statistics.

**Default solver:** `black`.

---

## `gen_puzzle` (`src/bin/gen_puzzle.rs`)

Generates a random puzzle with a unique solution, optionally filtered to a
difficulty window defined by the solver's backtracking node count and/or
productive propagation-wave count.

```
cargo run --release --bin gen_puzzle -- \
  [--size=N] [--min-nodes=K] [--max-nodes=K] \
  [--min-waves=W] [--max-waves=W] \
  [--threads=T] [--solver=basic|queue|black]
```

**Strategy:** a pool of worker threads independently fill random grids via DFS,
derive the targets, and run `solve()`. The first thread to find a puzzle whose
node count *and* wave count both fall in their windows wins; the others stop.
A spinner shows progress.

**Special case `--max-nodes=1`:** skips `solve()` entirely — only puzzles
solvable by propagation alone (no backtracking) are accepted. Much faster.
`--min-waves` / `--max-waves` still apply in this mode, and are how you target
the propagation-only difficulty buckets surfaced by the web UI ("Easy" /
"Medium" / "Challenging") — see `web/src/state/classification.ts` for the
per-size wave thresholds.

Output includes the target string (ready to paste into `cargo run --`) and a
shareable URL.

---

## `enumerate` (`src/bin/enumerate.rs`)

Counts the total number of valid Doplo puzzles (unique-solution) of a given
size. Uses Rayon for parallelism.

```
cargo run --release --bin enumerate -- [--size=N] [--solver=basic|queue|black]
```

Feasible up to N=6; larger sizes become prohibitively expensive.

---

## `compare` (`src/bin/compare.rs`)

Differential test: loops forever generating random puzzles and solving each one
with all three solver backends (`basic`, `queue`, `black`). Exits non-zero if
any two backends disagree on the outcome, the solved cells, or whether
backtracking was required.

```
cargo run --bin compare -- [--size=N]
```

Useful when developing a new solver or changing propagation logic.

---

## Solver implementations

All binaries share the same three backends, selectable via `--solver`:

| Name | Description |
|---|---|
| `black` | Default. Arc-consistency (live-tuple) + hidden singles + singleton. See [`black_solver.md`](black_solver.md). |
| `queue` | Earlier arc-consistency approach using a different tuple representation. |
| `basic` | Simple propagation only (singleton + hidden singles); no arc consistency. Slower on hard puzzles. |
