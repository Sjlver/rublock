# Agent Instructions

This is primarily a Rust learning project. Prefer simple, idiomatic Rust, and explain relevant Rust concepts when that helps the user learn.

The web frontend under `web/` has a secondary role as a **Svelte 5 + TypeScript** learning project (see issue #12 for the migration to Vite + Svelte + TS). The same "simple and idiomatic" preference applies there: favour Svelte 5 runes over older patterns, real TypeScript types over `any`, and small focused modules over a single large file. Rust changes still take priority over frontend polish.

- Building and testing:
  - For local runs, prefer `mise exec -- ...` so the configured Rust toolchain and `wasm-pack` are available. In other environments, such as Claude Code on the Web, use the equivalent available commands without assuming `mise` exists.
  - Main local test command: `mise exec -- cargo test`.
- Web interface:
  - The canonical WASM build and copy steps are in `.github/workflows/deploy.yml`; mirror them locally with `mise exec` when available.
  - Main local WASM build command: `mise exec -- wasm-pack build --target web --release --features wasm`.
  - The static web app imports WASM from `web/pkg/`. After building, copy `pkg/rublock_bg.wasm` and `pkg/rublock.js` into `web/pkg/` before browser smoke tests.
  - Once issue #12 lands, the frontend will be a Vite + Svelte 5 + TypeScript app: run `npm ci` and `npm run build` from `web/` (after the WASM build) and serve `web/dist/`. Update this section as part of that PR.
