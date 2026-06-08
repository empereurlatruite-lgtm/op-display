import * as THREE from "three";

// Faction colors (Foxhole: Wardens blue, Colonials green).
const FACTION_COLOR = {
  warden: 0x3a6ea5,
  colonial: 0x5b8a3a,
  neutral: 0x9aa4ad,
};

/**
 * Build a small 3D token for a placed unit. `unit` is the DTO the Rust engine
 * returns ({ id, kind, faction, pos, heading?, size? }). The returned group sits
 * at the origin; the caller positions it on the terrain and tags it. Distinct
 * silhouettes per kind so Squad / Boat / Tank read at a glance.
 */
export function createUnitMesh(unit) {
  const color = FACTION_COLOR[unit.faction] ?? FACTION_COLOR.neutral;
  const mat = new THREE.MeshStandardMaterial({ color, roughness: 0.55, metalness: 0.25 });
  const dark = new THREE.MeshStandardMaterial({ color: 0x1c2128, roughness: 0.7 });
  const group = new THREE.Group();

  if (unit.kind === "squad") {
    // a troop token: short cylinder + a dome.
    const body = new THREE.Mesh(new THREE.CylinderGeometry(0.22, 0.26, 0.28, 18), mat);
    body.position.y = 0.14;
    const dome = new THREE.Mesh(new THREE.SphereGeometry(0.2, 16, 12), mat);
    dome.position.y = 0.36;
    group.add(body, dome);
  } else if (unit.kind === "boat") {
    // an elongated hull + a small cabin.
    const hull = new THREE.Mesh(new THREE.BoxGeometry(0.36, 0.16, 0.9), mat);
    hull.position.y = 0.1;
    const cabin = new THREE.Mesh(new THREE.BoxGeometry(0.24, 0.16, 0.3), dark);
    cabin.position.set(0, 0.24, -0.12);
    group.add(hull, cabin);
  } else {
    // tank: hull + turret + barrel.
    const hull = new THREE.Mesh(new THREE.BoxGeometry(0.46, 0.2, 0.64), mat);
    hull.position.y = 0.12;
    const turret = new THREE.Mesh(new THREE.BoxGeometry(0.28, 0.16, 0.3), mat);
    turret.position.y = 0.3;
    const barrel = new THREE.Mesh(new THREE.CylinderGeometry(0.04, 0.04, 0.5, 10), dark);
    barrel.rotation.x = Math.PI / 2;
    barrel.position.set(0, 0.3, 0.34);
    group.add(hull, turret, barrel);
  }

  group.traverse((o) => {
    if (o.isMesh) {
      o.castShadow = true;
      o.receiveShadow = true;
    }
  });

  if (typeof unit.heading === "number") group.rotation.y = unit.heading;
  group.userData.unitId = unit.id;
  group.userData.unit = unit;
  return group;
}

/** Tracks placed unit meshes for batch disposal (mirrors MarkerLayer). */
export class UnitLayer {
  constructor(scene) {
    this.scene = scene;
    this.items = new Map(); // id -> group
  }
  add(group) {
    this.items.set(group.userData.unitId, group);
    this.scene.add(group);
    return group;
  }
  remove(id) {
    const g = this.items.get(id);
    if (!g) return;
    this.scene.remove(g);
    dispose(g);
    this.items.delete(id);
  }
  clear() {
    for (const g of this.items.values()) {
      this.scene.remove(g);
      dispose(g);
    }
    this.items.clear();
  }
}

function dispose(obj) {
  obj.traverse((o) => {
    if (o.isMesh) {
      o.geometry?.dispose();
      o.material?.dispose();
    }
  });
}
