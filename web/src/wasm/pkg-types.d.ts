// Hand-written declaration for the wasm-pack output. The generated
// `pkg/rublock.d.ts` lives outside the TS project (it is dropped in by the
// build), so we declare the surface we actually use.
declare module './pkg/rublock.js' {
  export default function init(
    moduleOrPath?: string | URL | Request | Response | BufferSource | WebAssembly.Module
  ): Promise<unknown>;
  export function generate_puzzle(size: number): string;
  export function solve_puzzle(rowTargets: Uint8Array, colTargets: Uint8Array): string;
}
