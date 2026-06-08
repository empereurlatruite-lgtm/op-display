// Browser side of the wargame engine (Phase 2). Loads the Rust-compiled WASM
// (engine/opengine-wasm -> web/wargame.wasm) and drives its JSON-over-memory
// ABI. All placement rules / terrain queries run in Rust (opengine-core), over
// the same baked grid that renders the terrain — so the browser and a future
// native sim agree by construction.
//
// No wasm-bindgen: a tiny hand-written protocol keeps the project tooling-free.

let ex = null; // wasm exports
const enc = new TextEncoder();
const dec = new TextDecoder();

/**
 * Instantiate the WASM module and load the terrain artifact into the engine.
 * Returns true on success; logs and returns false if the module/artifact are
 * missing (the rest of the app keeps working without unit placement).
 */
export async function initWargame(
  wasmUrl = "./wargame.wasm",
  terrainUrl = "./endless-shore-terrain.json"
) {
  try {
    const [wasmBuf, artifact] = await Promise.all([
      fetch(wasmUrl).then((r) => {
        if (!r.ok) throw new Error(`wasm ${r.status}`);
        return r.arrayBuffer();
      }),
      fetch(terrainUrl).then((r) => {
        if (!r.ok) throw new Error(`terrain ${r.status}`);
        return r.json();
      }),
    ]);
    const { instance } = await WebAssembly.instantiate(wasmBuf, {});
    ex = instance.exports;
    const r = call({ cmd: "loadArtifact", artifact });
    if (!r.ok) throw new Error(r.error || "loadArtifact failed");
    return true;
  } catch (err) {
    console.warn("wargame engine unavailable:", err.message);
    ex = null;
    return false;
  }
}

// One JSON command -> JSON response, marshalled through wasm linear memory.
// Memory views are recreated each call because the buffer can detach on grow.
function call(obj) {
  if (!ex) throw new Error("wargame not initialized");
  const bytes = enc.encode(JSON.stringify(obj));
  const p = ex.alloc(bytes.length);
  new Uint8Array(ex.memory.buffer, p, bytes.length).set(bytes);
  const rp = ex.dispatch(p, bytes.length); // frees the input buffer
  const rl = ex.last_response_len();
  const out = dec.decode(new Uint8Array(ex.memory.buffer, rp, rl).slice());
  ex.dealloc(rp, rl);
  return JSON.parse(out);
}

export const wargame = {
  ready: () => !!ex,
  /** Preview placement validity at `[nx, ny]` -> { ok, valid, error? }. */
  validate: (kind, at) => call({ cmd: "validate", kind, at }),
  /** Place a unit -> { ok, id } or { ok:false, error }. */
  place: (kind, faction, at, opts = {}) =>
    call({ cmd: "place", kind, faction, at, ...opts }),
  list: () => call({ cmd: "list" }),
  remove: (id) => call({ cmd: "remove", id }),
  clear: () => call({ cmd: "clear" }),
  /** Line of sight from -> to ([nx,ny] each) -> { ok, visible }. */
  lineOfSight: (from, to) => call({ cmd: "lineOfSight", from, to }),
};
