import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";
import { buildHexTerrain } from "./terrain.js";
import { normToWorld } from "./hex.js";
import { createMarker, createLabel, createArrow, MarkerLayer } from "./markers.js";

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
buildHexTerrain({ textureUrl: "./endless-shore.png" })
  .then(({ group, top }) => {
    scene.add(group);
    hexTop = top;
    setStatus("Terrain loaded.");
    return placeTownLabels(); // current Able town names, dropped onto the terrain
  })
  .catch(() => setStatus("Failed to build terrain — check console."));

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

// ---- Interaction state -----------------------------------------------------
const raycaster = new THREE.Raycaster();
const pointer = new THREE.Vector2();
let tool = "marker";
let arrowStart = null; // first click point when drawing an arrow

const colorInput = document.getElementById("color");
const labelInput = document.getElementById("labelText");
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
    setStatus(tool === "arrow" ? "Arrow: click the start point." : "");
  });
});

document.getElementById("clear").addEventListener("click", () => {
  markers.clear();
  arrowStart = null;
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
