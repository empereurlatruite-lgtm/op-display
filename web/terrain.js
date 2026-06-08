import * as THREE from "three";
import { createHex } from "./hex.js";

/**
 * Build the Endless Shore tile: a flat hex prism textured with the **official
 * Foxhole region map** (foxhole.wiki.gg) — clean water / land / roads. No
 * heightmap, no displacement, no procedural shading. The official map is the
 * source of truth, so what you see is the real map.
 *
 * Returns { group, top } where `top` is the flat textured hex (raycast target).
 */
export async function buildHexTerrain({ mapUrl = "./endless-shore-map.png" } = {}) {
  let texture = null;
  try {
    const img = await loadImage(mapUrl);
    texture = new THREE.Texture(img);
    texture.colorSpace = THREE.SRGBColorSpace;
    texture.anisotropy = 16;
    texture.needsUpdate = true;
  } catch (e) {
    console.warn("map texture failed to load:", e.message);
  }

  const { group, top } = createHex({ texture });
  top.castShadow = true;
  top.receiveShadow = true;
  return { group, top };
}

export function loadImage(url) {
  return new Promise((resolve, reject) => {
    const i = new Image();
    i.crossOrigin = "anonymous";
    i.onload = () => resolve(i);
    i.onerror = reject;
    i.src = url;
  });
}
