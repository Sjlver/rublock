# Agent Instructions

This is primarily a Rust learning project. Prefer simple, idiomatic Rust, and explain relevant Rust concepts when that helps the user learn.

The web frontend under `web/` has a secondary role as a **Svelte 5 + TypeScript** learning project (see issue #12 for the migration to Vite + Svelte + TS). The same "simple and idiomatic" preference applies there: favour Svelte 5 runes over older patterns, real TypeScript types over `any`, and small focused modules over a single large file. Rust changes still take priority over frontend polish.

- Building and testing:
  - For local runs, prefer `mise exec -- ...` so the configured Rust toolchain and `wasm-pack` are available. In other environments, such as Claude Code on the Web, use the equivalent available commands without assuming `mise` exists.
  - Main local test command: `mise exec -- cargo test`.
- Web interface:
  - The frontend is a **Vite + Svelte 5 (runes) + TypeScript** app under `web/`. The shell is `web/index.html`; everything else lives under `web/src/`.
  - Build pipeline: build the wasm crate, stage `pkg/rublock_bg.wasm` and `pkg/rublock.js` into `web/src/wasm/pkg/`, then run `npm run build` in `web/`. The canonical sequence is in `.github/workflows/deploy.yml`.
  - Local end-to-end build + preview: `mise run web` (does wasm build + stage + `npm install` + `npm run build` + `npm run preview`).
  - Local dev server with HMR: `mise run web-dev`.
  - Format everything before committing: `mise run fmt` runs `cargo fmt` and the web prettier config in one shot.
  - Direct npm scripts (run from `web/`): `dev`, `build`, `preview`, `check` (svelte-check), `format`.
