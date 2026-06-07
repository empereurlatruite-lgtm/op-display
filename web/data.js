import * as THREE from "three";
import { normToWorld } from "./hex.js";

// Foxhole faction colors (Colonial green, Warden blue, contested/neutral grey).
const TEAM_COLOR = {
  COLONIALS: 0x5b8a3a,
  WARDENS: 0x3a6ea5,
  NONE: 0x9aa4ad,
};

/**
 * Fetch the exported Endless Shore snapshot and build a group of markers,
 * one per real map item, placed at its true normalized position.
 * Returns { group, meta } or null if the data file is missing.
 */
export async function loadHexData(url = "./endless-shore.json") {
  let data;
  try {
    const res = await fetch(url);
    if (!res.ok) return null;
    data = await res.json();
  } catch {
    return null;
  }

  const group = new THREE.Group();
  group.name = "data-layer";

  // Reuse one geometry; just swap material color per team.
  const pin = new THREE.CylinderGeometry(0.12, 0.12, 0.5, 12);
  const cap = new THREE.SphereGeometry(0.2, 12, 10);
  const matCache = new Map();
  const material = (team) => {
    if (!matCache.has(team)) {
      matCache.set(
        team,
        new THREE.MeshStandardMaterial({
          color: TEAM_COLOR[team] ?? TEAM_COLOR.NONE,
          roughness: 0.5,
          metalness: 0.1,
        })
      );
    }
    return matCache.get(team);
  };

  for (const item of data.items ?? []) {
    const mat = material(item.team);
    const marker = new THREE.Group();

    const stem = new THREE.Mesh(pin, mat);
    stem.position.y = 0.25;
    stem.castShadow = true;
    marker.add(stem);

    const head = new THREE.Mesh(cap, mat);
    head.position.y = 0.55;
    head.castShadow = true;
    marker.add(head);

    const p = normToWorld(item.x, item.y, 0);
    marker.position.copy(p);
    marker.userData.item = item; // iconType/team kept for future labelling
    group.add(marker);
  }

  return { group, meta: data };
}
