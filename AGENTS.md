# Agent Instructions

This is a Rust learning project. Prefer simple, idiomatic Rust, and explain relevant Rust concepts when that helps the user learn.

- Building and testing:
  - For local runs, prefer `mise exec -- ...` so the configured Rust toolchain and `wasm-pack` are available. In other environments, such as Claude Code on the Web, use the equivalent available commands without assuming `mise` exists.
  - Main local test command: `mise exec -- cargo test`.
- Web interface:
  - The canonical WASM build and copy steps are in `.github/workflows/deploy.yml`; mirror them locally with `mise exec` when available.
  - Main local WASM build command: `mise exec -- wasm-pack build --target web --release --features wasm`.
  - The static web app imports WASM from `web/pkg/`. After building, copy `pkg/rublock_bg.wasm` and `pkg/rublock.js` into `web/pkg/` before browser smoke tests.
