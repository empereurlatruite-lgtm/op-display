# op-display — single-hex op planning board

A tiny three.js tool for the 2REI regiment: render **one hexagon** in 3D that you
can orbit and zoom, and drop **markers, labels, and arrows** onto its surface to
brief a single operation ("op").

Static HTML + CDN three.js — **no build step, no npm**, same convention as
`../propaganda/` and `../publisheur/web/`.

**Repo:** <https://github.com/empereurlatruite-lgtm/op-display>

## Run it

ES module scripts need to be served over http(s) (not opened as a `file://` path),
so start a tiny local server:

```bash
cd web
python3 -m http.server 3003
```

Then open <http://localhost:3003/>.

> Port **3003** is in the 3000–3999 "Web UIs/Dashboards" range tracked by
> `~/FEADNGRAY-Workspace/Gray-PortsDirectory` (8000 is taken by `DeclarAI-llm-hardware`).

(Or deploy `web/` to Cloudflare/GitHub Pages like the other tools.)

## Use it

- **Drag** to orbit, **scroll** to zoom.
- Pick a tool in the panel, then **click the hex** to place:
  - **📍 Marker** — a colored pin on the surface.
  - **🏷️ Label** — a camera-facing text tag (set the text in the panel first).
  - **➳ Arrow** — click a start point, then an end point.
- **Color** applies to the next item placed.
- **Clear all** removes every annotation.

## Files

| File | Role |
|------|------|
| `web/index.html` | Viewport, control panel, three.js import map (CDN). |
| `web/app.js`     | Scene, camera, lights, OrbitControls, click→raycast placement. |
| `web/hex.js`     | Hexagon geometry (6-segment cylinder = hex prism) + top-face helper. |
| `web/markers.js` | Marker / label-sprite / arrow factories + a disposable layer. |
| `web/baked.js`   | Loads the Rust-baked terrain grid into a THREE geometry (see Engine). |
| `web/wargame.js` | Loads the wargame WASM + drives its placement engine (see Engine). |
| `web/units.js`   | Squad / boat / tank token meshes + a disposable layer. |
| `web/style.css`  | Fullscreen canvas + side panel styling. |

## Engine (Rust)

A small Rust workspace under [`engine/`](engine/) owns the terrain computation
and the wargame logic:

- **`opengine-core`** — hex/terrain math + the squad/boat/tank model.
- **`op-engine`** — native CLI baker: heightmap → compact terrain grid
  (`web/endless-shore-terrain.json`, committed) the web app loads at runtime.
- **`opengine-wasm`** — the same core compiled to WebAssembly for the in-browser
  unit planner (`web/wargame.wasm`, committed).

Both artifacts are committed, so serving/deploying stays buildless.

```bash
cd engine
cargo test                                   # core math + wargame
# re-bake the terrain grid (only when source PNGs / params change):
cargo run -p op-engine -- \
  --height ../web/endless-shore-height.png \
  --texture ../web/endless-shore.png \
  --out ../web/endless-shore-terrain.json \
  --seg-x 320 --seg-y 277 --relief 0.8
./build-wasm.sh                              # rebuild web/wargame.wasm
```

**Units:** pick Squad / Boat / Tank and a faction, then click the hex — the Rust
engine validates each placement against the terrain (inside the hex, boats need
water, tanks can't climb cliffs, units can't overlap) and the browser renders a
token. The **👁 Line of sight** tool takes two clicks and draws the sight line
green (clear) or red (blocked by terrain), computed by the engine over the
height grid.

Load `?mode=js` to force the original in-browser procedural terrain instead of
the baked grid — handy for side-by-side comparison. See
[`engine/README.md`](engine/README.md) for details.

## Endless Shore wiring

This board is set up for the **Endless Shore** hex (the op location) with **3D
elevation**:

- **Terrain surface** — `web/endless-shore.png` is the **real in-game Endless Shore
  map** (cropped to the hex bounding-box aspect, `HEX_W/HEX_H`), textured onto the
  tile 1:1 and masked to the hex outline. (The community-tiles texture was replaced
  with the actual game map because it rendered neighbour snow/biomes incorrectly.)
- **Real elevation** — `web/endless-shore-height.png` is the actual Foxhole
  heightmap for Endless Shore (white = high cliffs/mesas, dark = low/water).
  `web/terrain.js` samples its luminance and displaces a subdivided hex surface, so
  the mesas, cliffs, and water channels are true terrain. Tune `relief` in
  `buildHexTerrain()` (currently ~1.15). If the height PNG is missing it falls back
  to inferring relief from the map colors.

### Where the terrain data comes from

Both PNGs are cropped from the community world map:

- **Tiles:** [`pickles976/Foxhole-Map-3D-Tiles`](https://github.com/pickles976/Foxhole-Map-3D-Tiles)
  — a 16384×16384 world texture + heightmap (heightmap by **Derp / NoUDerp**),
  sliced into `{x}_{y}_{size}.png` tiles. Renderer: [`Foxhole-Map-3D`](https://github.com/pickles976/Foxhole-Map-3D).
- **Crop math** mirrors that repo's `config.js` / `utils.js` `OffsetToPosition`.
  Endless Shore is hex `[1.5, 0]` (Deadlands `[0,0]` is map center).
- **Terrain is war-invariant** — a hex's land/cliffs are identical every war, so
  this never goes stale (only ownership/bases change, and those pins were removed).

### Regenerate / other hexes

```bash
python3 tools/fetch_hex_terrain.py ENDLESS     # -> web/endless-shore{,-height}.png
```

Add entries to `REGIONS` in that script (keyed like `regions.js`) for other hexes.
Lower `SIZE` for more detail (more tile downloads).

### Notes

- The texture's cyan water/padding is recolored to Foxhole blue in the fetch script.
- Coordinate convention: `x,y ∈ [0,1]`, `(0,0)` = top-left (NW) of the hex box —
  see `normToWorld()` in `web/hex.js`.

### Live town labels (current war, Able shard)

The town names on the terrain are the **current** ones from the live War API on the
**Able (Live-1)** shard — `web/endless-shore-labels.json` (War #135 at time of
writing). Charlie/Live-3 (what `charlie_tracker` polls) has been 503 since 2026-05-25;
**Able is up**. Refresh the labels with:

```bash
curl -s -H "User-Agent: 2REI-opboard" \
  "https://war-service-live.foxholeservices.com/api/worldconquest/maps/EndlessShoreHex/static" \
| python3 -c 'import sys,json; s=json.load(sys.stdin); \
print(json.dumps({"hex":"EndlessShoreHex","displayName":"Endless Shore","shard":"Able (Live-1)","regionId":s.get("regionId"),"labels":[{"text":t["text"],"x":round(t["x"],5),"y":round(t["y"],5)} for t in s.get("mapTextItems",[])]}, indent=2))' \
> web/endless-shore-labels.json
```

The label x/y are normalized `[0,1]` over the same hex box as the heightmap, so they
drop straight onto the terrain (raycast in `app.js` → `placeTownLabels`). Terrain is
war-invariant; only these labels (and control) change between wars.

> Mountain height is `relief` in `buildHexTerrain()` (currently `0.8` — modest, since
> the Derp heightmap is real/verified against live Able town positions). The old
> base-marker overlay (`web/data.js`, `web/endless-shore.json`) is kept but unused.
