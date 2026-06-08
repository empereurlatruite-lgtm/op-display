import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { buildHexTerrain } from "./terrain.js";
import { normToWorld, HEX_WIDTH, HEX_DEPTH } from "./hex.js";
import { createMarker, createLabel, createArrow, MarkerLayer } from "./markers.js";
import { initWargame, wargame } from "./wargame.js";
import { createUnitMesh, UnitLayer } from "./units.js";

const UNIT_TOOLS = new Set(["squad", "boat", "tank"]);

// ---- Scene / camera / renderer ---------------------------------------------
const canvas = document.getElementById("scene");
const renderer = new THREE.WebGLRenderer({ canvas, antialias: true });
renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.PCFSoftShadowMap;

const scene = new THREE.Scene();
scene.background = new THREE.Color(0x11161c);

const camera = new THREE.PerspectiveCamera(50, 1, 0.1, 1000);
camera.position.set(0, 13, 15);

const controls = new OrbitControls(camera, canvas);
controls.enableDamping = true;
controls.target.set(0, 0, 0);

// ---- Lights ----------------------------------------------------------------
scene.add(new THREE.AmbientLight(0xffffff, 0.55));

const sun = new THREE.DirectionalLight(0xffffff, 1.1);
sun.position.set(6, 12, 4);
sun.castShadow = true;
sun.shadow.mapSize.set(1024, 1024);
sun.shadow.camera.near = 1;
sun.shadow.camera.far = 40;
sun.shadow.camera.left = -12;
sun.shadow.camera.right = 12;
sun.shadow.camera.top = 12;
sun.shadow.camera.bottom = -12;
scene.add(sun);

// ---- The hex: 3D-elevated Endless Shore terrain (relief from the map) --------
let hexTop = null; // raycast target; set once the terrain is built
const loaderEl = document.getElementById("loader");
function hideLoader() {
  loaderEl?.classList.add("hidden");
}
// Safety net: never leave the loader stuck if something hangs.
const loaderTimeout = setTimeout(hideLoader, 8000);

buildHexTerrain()
  .then(({ group, top }) => {
    scene.add(group);
    hexTop = top;
    setStatus("Map loaded.");
    // Boot the Rust wargame engine (terrain queries + placement rules) so the
    // Squad/Boat/Tank tools work. Failure is non-fatal — annotations still work.
    initWargame().then((ok) => {
      if (ok) setStatus("Engine ready — place markers or units.");
    });
    return placeTownLabels(); // current Able town names, dropped onto the terrain
  })
  .then(() => {
    // Reveal only after a couple of frames so the shaded terrain (its patched
    // shader compiles on first render) is on screen before the fade-in.
    clearTimeout(loaderTimeout);
    requestAnimationFrame(() => requestAnimationFrame(hideLoader));
  })
  .catch(() => {
    setStatus("Failed to build terrain — check console.");
    clearTimeout(loaderTimeout);
    hideLoader();
  });

// Live current-war town labels from the Able shard (saved snapshot).
async function placeTownLabels() {
  let data;
  try {
    const res = await fetch("./endless-shore-labels.json");
    if (!res.ok) return;
    data = await res.json();
  } catch {
    return;
  }
  const down = new THREE.Vector3(0, -1, 0);
  const ray = new THREE.Raycaster();
  for (const l of data.labels ?? []) {
    const p = normToWorld(l.x, l.y, 6); // start above terrain
    ray.set(new THREE.Vector3(p.x, 30, p.z), down);
    const hit = ray.intersectObject(hexTop, false)[0];
    const at = hit ? hit.point : normToWorld(l.x, l.y, 0);
    scene.add(
      createLabel(at, l.text, "#f7eed6", { onSurface: true, worldHeight: 0.42, depthTest: true })
    );
  }
  setMeta(
    `${data.displayName} · ${data.shard} · War #${data.warNumber} · ${data.labels.length} towns`
  );
}

const markers = new MarkerLayer(scene);
const units = new UnitLayer(scene);

// ---- Interaction state -----------------------------------------------------
const raycaster = new THREE.Raycaster();
const pointer = new THREE.Vector2();
let tool = "marker";
let arrowStart = null; // first click point when drawing an arrow
let losStart = null; // first click for a line-of-sight query

const colorInput = document.getElementById("color");
const labelInput = document.getElementById("labelText");
const factionInput = document.getElementById("faction");
const statusEl = document.getElementById("status");
const metaEl = document.getElementById("meta");

function setStatus(msg) {
  statusEl.textContent = msg || "";
}

function setMeta(msg) {
  if (metaEl) metaEl.textContent = msg || "";
}

// Tool buttons.
document.querySelectorAll(".tools button").forEach((btn) => {
  btn.addEventListener("click", () => {
    document.querySelectorAll(".tools button").forEach((b) => b.classList.remove("active"));
    btn.classList.add("active");
    tool = btn.dataset.tool;
    arrowStart = null;
    losStart = null;
    if (tool === "arrow") setStatus("Arrow: click the start point.");
    else if (tool === "los") setStatus("Line of sight: click the observer.");
    else if (UNIT_TOOLS.has(tool)) setStatus(`Click the hex to deploy a ${tool}.`);
    else setStatus("");
  });
});

document.getElementById("clear").addEventListener("click", () => {
  markers.clear();
  units.clear();
  if (wargame.ready()) wargame.clear();
  arrowStart = null;
  losStart = null;
  setStatus("Cleared.");
});

// Distinguish a click from an orbit-drag so dragging the camera doesn't place.
let downPos = null;
canvas.addEventListener("pointerdown", (e) => {
  downPos = { x: e.clientX, y: e.clientY };
});

canvas.addEventListener("pointerup", (e) => {
  if (!downPos) return;
  const moved = Math.hypot(e.clientX - downPos.x, e.clientY - downPos.y);
  downPos = null;
  if (moved > 5) return; // it was a drag (orbit), not a placement click

  const hit = pickHex(e);
  if (!hit) return;
  placeAt(hit.point);
});

function pickHex(event) {
  const rect = canvas.getBoundingClientRect();
  pointer.x = ((event.clientX - rect.left) / rect.width) * 2 - 1;
  pointer.y = -((event.clientY - rect.top) / rect.height) * 2 + 1;
  raycaster.setFromCamera(pointer, camera);
  const hits = raycaster.intersectObject(hexTop, false);
  return hits[0] || null;
}

function placeAt(point) {
  const color = colorInput.value;

  if (UNIT_TOOLS.has(tool)) {
    placeUnit(tool, point);
    return;
  }
  if (tool === "los") {
    queryLineOfSight(point);
    return;
  }

  if (tool === "marker") {
    markers.add(createMarker(point, color));
    setStatus("Marker placed.");
  } else if (tool === "label") {
    const text = labelInput.value.trim() || "Label";
    markers.add(createLabel(point, text, color));
    setStatus(`Label "${text}" placed.`);
  } else if (tool === "arrow") {
    if (!arrowStart) {
      arrowStart = point.clone();
      setStatus("Arrow: click the end point.");
    } else {
      markers.add(createArrow(arrowStart, point, color));
      arrowStart = null;
      setStatus("Arrow placed.");
    }
  }
}

// World hit-point -> warapi-normalized [nx, ny] (inverse of normToWorld).
function worldToNorm(point) {
  return [point.x / HEX_WIDTH + 0.5, point.z / HEX_DEPTH + 0.5];
}

// Deploy a unit: the Rust engine validates terrain rules + occupancy; on success
// we render a token on the surface, on failure we report why (e.g. tank on water).
function placeUnit(kind, point) {
  if (!wargame.ready()) {
    setStatus("Engine not loaded — units unavailable.");
    return;
  }
  const faction = factionInput?.value || "neutral";
  const at = worldToNorm(point);
  const res = wargame.place(kind, faction, at);
  if (!res.ok) {
    setStatus(`Can't deploy ${kind} here: ${res.error}.`);
    return;
  }
  const unit = { id: res.id, kind, faction, pos: at, heading: 0 };
  const mesh = createUnitMesh(unit);
  mesh.position.copy(point);
  units.add(mesh);
  setStatus(`${faction} ${kind} deployed.`);
}

// Two-click line-of-sight query: the engine marches the height grid; we draw the
// sight line green (clear) or red (blocked by terrain).
function queryLineOfSight(point) {
  if (!wargame.ready()) {
    setStatus("Engine not loaded — line of sight unavailable.");
    return;
  }
  if (!losStart) {
    losStart = point.clone();
    setStatus("Line of sight: click the target.");
    return;
  }
  const from = worldToNorm(losStart);
  const to = worldToNorm(point);
  const res = wargame.lineOfSight(from, to);
  const visible = !!res.visible;
  markers.add(createSightLine(losStart, point, visible));
  setStatus(visible ? "Line of sight: CLEAR." : "Line of sight: BLOCKED by terrain.");
  losStart = null;
}

// A thin cylinder beats THREE.Line here — WebGL ignores line width, so a real
// tube is what actually reads as a sight line. Green = clear, red = blocked.
function createSightLine(a, b, visible) {
  const lift = new THREE.Vector3(0, 0.14, 0);
  const pa = a.clone().add(lift);
  const pb = b.clone().add(lift);
  const dir = new THREE.Vector3().subVectors(pb, pa);
  const len = dir.length() || 0.001;
  const geom = new THREE.CylinderGeometry(0.04, 0.04, len, 8);
  const mat = new THREE.MeshStandardMaterial({
    color: visible ? 0x49d17a : 0xe2453b,
    emissive: visible ? 0x176b39 : 0x7a1816,
    roughness: 0.5,
  });
  const mesh = new THREE.Mesh(geom, mat);
  mesh.position.copy(pa).lerp(pb, 0.5);
  mesh.quaternion.setFromUnitVectors(new THREE.Vector3(0, 1, 0), dir.normalize());
  return mesh;
}

// ---- Resize + render loop --------------------------------------------------
function resize() {
  const w = window.innerWidth;
  const h = window.innerHeight;
  renderer.setSize(w, h, false);
  camera.aspect = w / h;
  camera.updateProjectionMatrix();
}
window.addEventListener("resize", resize);
resize();

function animate() {
  requestAnimationFrame(animate);
  controls.update();
  renderer.render(scene, camera);
}
animate();
