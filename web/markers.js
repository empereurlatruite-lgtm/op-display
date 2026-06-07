import * as THREE from "three";
import { onTop } from "./hex.js";

/**
 * A pin marker: a cone standing on the hex face.
 */
export function createMarker(point, color) {
  const geometry = new THREE.ConeGeometry(0.35, 1.1, 16);
  const material = new THREE.MeshStandardMaterial({
    color,
    roughness: 0.4,
    metalness: 0.2,
  });
  const cone = new THREE.Mesh(geometry, material);
  cone.castShadow = true;

  const base = onTop(point);
  cone.position.set(base.x, base.y + 0.55, base.z); // half-height above the surface
  return cone;
}

/**
 * A camera-facing text label rendered to a canvas texture sprite.
 */
export function createLabel(point, text, color = "#ffffff", opts = {}) {
  const { onSurface = false, worldHeight: wh = 1.2, lift = onSurface ? 0.25 : 1.6 } = opts;
  const fontSize = 64;
  const padding = onSurface ? 6 : 16;

  const measure = document.createElement("canvas").getContext("2d");
  measure.font = `bold ${fontSize}px sans-serif`;
  const textWidth = Math.ceil(measure.measureText(text).width);

  const canvas = document.createElement("canvas");
  canvas.width = textWidth + padding * 2;
  canvas.height = fontSize + padding * 2;

  const ctx = canvas.getContext("2d");
  if (onSurface) {
    // Clean map-style label: text with a dark outline, no box (low clutter).
    ctx.font = `600 ${fontSize}px sans-serif`;
    ctx.textBaseline = "middle";
    ctx.textAlign = "center";
    ctx.lineJoin = "round";
    ctx.lineWidth = 8;
    ctx.strokeStyle = "rgba(0,0,0,0.9)";
    ctx.strokeText(text, canvas.width / 2, canvas.height / 2);
    ctx.fillStyle = color;
    ctx.fillText(text, canvas.width / 2, canvas.height / 2);
  } else {
    ctx.fillStyle = "rgba(17, 22, 28, 0.85)";
    roundRect(ctx, 0, 0, canvas.width, canvas.height, 14);
    ctx.fill();
    ctx.strokeStyle = color;
    ctx.lineWidth = 3;
    roundRect(ctx, 1.5, 1.5, canvas.width - 3, canvas.height - 3, 13);
    ctx.stroke();
    ctx.font = `bold ${fontSize}px sans-serif`;
    ctx.fillStyle = color;
    ctx.textBaseline = "middle";
    ctx.textAlign = "center";
    ctx.fillText(text, canvas.width / 2, canvas.height / 2);
  }

  const texture = new THREE.CanvasTexture(canvas);
  texture.anisotropy = 4;

  const sprite = new THREE.Sprite(
    new THREE.SpriteMaterial({ map: texture, transparent: true, depthTest: opts.depthTest ?? false })
  );

  // Scale to a sensible world size, preserving the canvas aspect ratio.
  sprite.scale.set(wh * (canvas.width / canvas.height), wh, 1);

  // onSurface: keep the point's own Y (sits on terrain); else clamp to hex top.
  const base = onSurface ? point : onTop(point);
  sprite.position.set(base.x, base.y + lift, base.z);
  sprite.userData.texture = texture; // kept for disposal
  return sprite;
}

/**
 * An arrow between two points on the hex face.
 */
export function createArrow(from, to, color) {
  const a = onTop(from, 0.05);
  const b = onTop(to, 0.05);
  const dir = new THREE.Vector3().subVectors(b, a);
  const length = dir.length();
  if (length < 1e-4) return null; // ignore zero-length arrows
  dir.normalize();

  const headLength = Math.min(1.0, length * 0.35);
  const headWidth = headLength * 0.6;
  return new THREE.ArrowHelper(dir, a, length, color, headLength, headWidth);
}

/**
 * Tracks placed annotations so they can all be removed and disposed at once.
 */
export class MarkerLayer {
  constructor(scene) {
    this.scene = scene;
    this.items = [];
  }

  add(object) {
    if (!object) return;
    this.scene.add(object);
    this.items.push(object);
  }

  clear() {
    for (const obj of this.items) {
      this.scene.remove(obj);
      disposeObject(obj);
    }
    this.items.length = 0;
  }
}

function disposeObject(obj) {
  obj.traverse?.((child) => {
    child.geometry?.dispose?.();
    const mat = child.material;
    if (Array.isArray(mat)) mat.forEach((m) => m.dispose?.());
    else mat?.dispose?.();
  });
  obj.userData?.texture?.dispose?.();
  obj.material?.map?.dispose?.();
  obj.material?.dispose?.();
}

function roundRect(ctx, x, y, w, h, r) {
  ctx.beginPath();
  ctx.moveTo(x + r, y);
  ctx.arcTo(x + w, y, x + w, y + h, r);
  ctx.arcTo(x + w, y + h, x, y + h, r);
  ctx.arcTo(x, y + h, x, y, r);
  ctx.arcTo(x, y, x + w, y, r);
  ctx.closePath();
}
