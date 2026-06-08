# op-display engine

A small Rust workspace that owns op-display's terrain computation and (Phase 2)
the wargame logic. It's the shared core behind two front-ends: a **native CLI
baker now**, and the **same core compiled to WASM later** for an in-browser
squad/boat/tank sim.

## Crates

- **`opengine-core`** — the shared library. Pure logic + types, no filesystem /
  image decoding / CLI, so it compiles unchanged to WASM for the browser.
  - `hex` — hex constants + `norm_to_world` (mirror of `web/hex.js`).
  - `terrain` — heightmap sampling, box blur, color/material classification, and
    `bake()` (the Phase-1 core; a port of the old `web/terrain.js` math).
  - `artifact` — serde structs for the baked grid (`web/<slug>-terrain.json`).
  - `wargame` — the squad/boat/tank model: `units`, `query` (`TerrainQuery` +
    `BakedGrid`), `placement` (rules + occupancy), and `session` (the JSON
    command engine the browser drives). LOS and pathfinding are still `todo!()`.
- **`op-engine`** — the native CLI baker. PNG I/O + file writing only; all math
  delegates to `opengine-core`.
- **`opengine-wasm`** — the browser ABI (Phase 2). A plain `cdylib` (no
  wasm-bindgen) exposing a tiny `alloc`/`dealloc`/`dispatch` protocol over linear
  memory; it just wraps `opengine_core::wargame::Session`. Driven by
  `web/wargame.js`.

## Build & test

```bash
cd engine
cargo build
cargo test            # core math + wargame stubs
```

## Bake the terrain artifact

From `engine/`:

```bash
cargo run -p op-engine -- \
  --height ../web/endless-shore-height.png \
  --texture ../web/endless-shore.png \
  --out ../web/endless-shore-terrain.json \
  --seg-x 320 --seg-y 277 --relief 0.8
```

Output lands at `web/endless-shore-terrain.json` and **is committed** — the web
app loads it at runtime, so serving/deploying stays buildless (GitHub/Cloudflare
Pages). Re-bake only when the source PNGs or terrain params change (terrain is
war-invariant, so this is rare).

`--relief` is stored in the artifact and applied in JS at displacement time, so
you can tune relief in the browser without re-baking. Pass `--from-color` to
infer relief from the texture when no heightmap is available.

### Pipeline

`tools/fetch_hex_terrain.py` still *fetches/crops* the source PNGs from the
community map tiles; `op-engine` *consumes* them into the grid. Porting the fetch
into an `op-engine fetch` subcommand is a possible follow-up.

### Artifact size note

`height` is stored as plain JSON numbers (rounded to 4dp) for a small,
diff-friendly file. If the committed size ever becomes a problem, the same
`web/baked.js` seam can switch to a base64-`Uint16Array` payload — no CDN
dependency either way.

## Wargame WASM (Phase 2)

The squad/boat/tank planner runs the Rust `wargame::Session` in the browser via
WebAssembly. The build artifact `web/wargame.wasm` is committed (like the terrain
grid) so the served site stays buildless.

```bash
rustup target add wasm32-unknown-unknown   # one-time
engine/build-wasm.sh                        # build + install web/wargame.wasm
```

Browser side: `web/wargame.js` instantiates the module and speaks a small
JSON-over-linear-memory protocol (`alloc` → write command bytes → `dispatch` →
read response → `dealloc`); `web/units.js` renders the tokens. The engine
validates every placement against the baked terrain grid — placements must be
inside the hexagon, boats need water, tanks can't climb cliffs, units can't
overlap — so the browser and a future native sim agree by construction.

**Done & tested:** placement (with hex containment + occupancy), terrain queries,
the command session, and **line of sight** (`TerrainQuery::line_of_sight` marches
the sight ray against the height grid; the browser's 👁 tool draws the line green
for clear / red for blocked).

**Next (still `todo!()`):** `placement::find_path` — A* over walkable cells for
movement routes. Additive: implement it, add a `session::Command` variant, and a
tool in `web/wargame.js`; no changes to Phase 1.
