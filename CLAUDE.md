# Agent Instructions

This is primarily a Rust learning project. Prefer simple, idiomatic Rust, and explain relevant Rust concepts when that helps the user learn.

The web frontend under `web/` has a secondary role as a **Svelte 5 + TypeScript** learning project. The same "simple and idiomatic" preference applies there: favor Svelte 5 runes over older patterns, real TypeScript types over `any`, and small focused modules over a single large file. Rust changes still take priority over frontend polish.

See `doc/spec/` for concise descriptions of the major subsystems.

## Toolchain

- **Rust**: managed by [mise](https://mise.jdx.dev/). The `mise.toml` pins `rust = "latest"` and installs `wasm-pack`. Run Rust commands via `mise exec -- ...` locally so the correct toolchain and `wasm-pack` are on `$PATH`.
- **JavaScript/Node**: also managed by mise (`node = "lts"`). Use `npm` (not `pnpm` or `yarn`) for the `web/` workspace.

## Building and testing

| Goal | Command |
|---|---|
| Rust tests | `mise exec -- cargo test` |
| WASM build | `mise exec -- wasm-pack build --target web --release --features wasm` |
| Full web build + preview | `mise run web` |
| Web dev server (HMR) | `mise run web-dev` |
| Playwright browser tests | `mise run test-web` |
| All tests | `mise run test` |
| Format everything | `mise run fmt` |

Direct `npm` scripts (run from `web/`): `dev`, `build`, `build:test`, `preview`, `check` (svelte-check), `format`, `test`.

## Playwright tests

**Claude Code on the Web cannot run Playwright tests** (no browser available). Do not attempt `mise run test-web` or `npm run test` in that environment — the command will hang or fail.

Instead, open a pull request. The `browser-tests.yml` CI workflow runs the full Playwright suite on every PR and uploads the report as an artifact. Claude will be notified of CI results and can act on any failures.

## Web build pipeline

1. `wasm-pack build --target web --release --features wasm` → produces `pkg/`
2. Copy `pkg/rublock_bg.wasm` and `pkg/rublock.js` into `web/src/wasm/pkg/`
3. `npm run build` in `web/`

The canonical sequence is in `.github/workflows/deploy.yml`. `mise run web` does all three steps.

## Code style

- Format before committing: `mise run fmt` runs `cargo fmt` and the web prettier config in one shot.
- No comments unless the *why* is non-obvious. Don't describe *what* the code does.
