// Hand-written declaration for the wasm-pack output. The generated
// `pkg/rublock.d.ts` lives outside the TS project (it is dropped in by the
// build), so we declare the surface we actually use.
//
// Each function returns native JS values directly (built by `serde-wasm-bindgen`
// on the Rust side) and throws a `string` error on the failure path.
declare module './pkg/rublock.js' {
  export default function init(
    moduleOrPath?: string | URL | Request | Response | BufferSource | WebAssembly.Module
  ): Promise<unknown>;
  export function generate_puzzle(size: number): unknown;
  export function solve_puzzle(rowTargets: Uint8Array, colTargets: Uint8Array): unknown;
  export function explain_puzzle(rowTargets: Uint8Array, colTargets: Uint8Array): unknown;
}
