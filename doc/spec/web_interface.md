# Web Interface (`web/`)

A static single-page app that loads the Rust solver as a WASM module.
Built with **Vite + Svelte 5 (runes) + TypeScript**.

## Tabs

| Tab | Purpose |
|---|---|
| **Play** | Solve a puzzle interactively. Supports digit entry, pencil notes, undo/redo, and a "solve" shortcut. Progress is persisted to `localStorage`. |
| **Create** | Specify custom row/column targets and hand the puzzle off to Play. |
| **Print** | Generate a printable sheet of puzzles at a chosen difficulty. |
| **Walkthrough** | Step through the solver's reasoning for the current puzzle, one propagation wave at a time. |
| **How to play** | Static rules explanation with an animated example. |

## Source layout

```
web/src/
  main.ts                # entry point: mounts App, installs error overlay, init locale
  components/
    App.svelte           # root: tab router, WASM init, persistence bootstrap
    BottomNav.svelte     # bottom navigation bar
    PuzzleGrid.svelte    # the N×N grid with target labels
    InputBar.svelte      # digit/black/notes input palette
    PageHeader.svelte    # title bar and share button
    tabs/                # one Svelte component per tab
  state/
    puzzle.svelte.ts     # playState: cell values, notes, selected cell, undo stack
    create.svelte.ts     # createState: targets for the Create tab
    url.svelte.ts        # tab routing via URL hash; share-link generation
    storage.svelte.ts    # localStorage persistence (reads/writes playState)
    toast.svelte.ts      # ephemeral notification state
    types.ts             # shared TypeScript types (PuzzleData, CellValue, …)
  wasm/                  # WASM glue
  i18n/                  # locale strings (English + German)
  analytics.ts           # event tracking (excluded from test builds)
  share.ts               # share-sheet / clipboard helper
  error-overlay.ts       # dev-time error overlay
```

## State management

All reactive state uses **Svelte 5 runes** (`$state`, `$derived`, `$effect`).
Module-level `$state` objects act as singletons (e.g. `playState` in
`puzzle.svelte.ts`). Components read and write these objects directly — there
is no Svelte store or context API in use.

## WASM integration

`web/src/wasm/api.ts` wraps the generated `rublock.js` bindings. Two main
functions are exposed to the app:

- `generatePuzzle(size, minNodes, maxNodes)` — runs the Rust generator.
- `solvePuzzle(rowTargets, colTargets)` — runs the black solver and returns
  the solved grid plus an `Explain` recorder snapshot for the walkthrough.

WASM is initialised once at app startup (`initWasm()`); all subsequent calls
are synchronous.

## Persistence

`storage.svelte.ts` saves `playState` (cell values, notes, puzzle identity, and
active puzzle size) to `localStorage` under a versioned key. It is installed as
a `$effect` in `App.svelte` and writes only after the boot sequence has
populated state (controlled by the `persistReady` flag).

## Build pipeline

See [`CLAUDE.md`](../../CLAUDE.md) for commands. In brief:

1. Build the WASM crate with `wasm-pack`.
2. Copy `pkg/rublock_bg.wasm` and `pkg/rublock.js` into `web/src/wasm/pkg/`.
3. Run `npm run build` (or `npm run build:test` to exclude analytics).

## Testing

End-to-end browser tests live in `web/src/` alongside the source (Playwright).
They cover the golden paths for each tab. **These tests cannot be run in Claude
Code on the Web** — push a PR and CI will run them automatically
(`browser-tests.yml`).
