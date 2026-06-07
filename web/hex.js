import * as THREE from "three";

// A flat-top regular hexagon (vertices left/right, flat edges top/bottom) —
// the same orientation Foxhole uses for its hex maps.
export const HEX_RADIUS = 6; // center -> vertex
export const HEX_HEIGHT = 0.6; // extrusion thickness
export const HEX_TOP_Y = HEX_HEIGHT / 2;

// Bounding box of the flat-top hexagon (matches the warapi map-image box).
export const HEX_WIDTH = 2 * HEX_RADIUS; // vertex-to-vertex (x)
export const HEX_DEPTH = Math.sqrt(3) * HEX_RADIUS; // flat-to-flat (z)

// Six flat-top vertices in the XZ plane, angle offset 0 -> vertex at +x and -x.
export function hexCorners(radius = HEX_RADIUS) {
  const pts = [];
  for (let i = 0; i < 6; i++) {
    const a = (Math.PI / 3) * i; // 0,60,...,300
    pts.push(new THREE.Vector2(radius * Math.cos(a), radius * Math.sin(a)));
  }
  return pts;
}

/**
 * Build the hex: an extruded prism body plus a flat top face whose UVs map the
 * warapi bounding box [0..1]^2 onto the hexagon, so a map image lands 1:1 and
 * the cut corners naturally mask the neighbour-bleed at the image corners.
 *
 * Returns { group, top } — `top` is the mesh used as the click/raycast target.
 */
export function createHex({ radius = HEX_RADIUS, height = HEX_HEIGHT, texture = null } = {}) {
  const group = new THREE.Group();
  const corners2d = hexCorners(radius);

  // --- prism body (gives the tile its 3D thickness + side rim) ---
  const shape = new THREE.Shape(corners2d);
  const body = new THREE.ExtrudeGeometry(shape, {
    depth: height,
    bevelEnabled: false,
  });
  // Extrude builds in XY extruded along +Z; rotate so thickness runs along Y.
  body.rotateX(-Math.PI / 2);
  body.translate(0, height, 0); // base at y=0, top at y=height
  body.center(); // recenters all axes; top face ends near y=+height/2

  const bodyMat = new THREE.MeshStandardMaterial({
    color: 0x2b3a2f,
    roughness: 0.9,
    metalness: 0.05,
  });
  const bodyMesh = new THREE.Mesh(body, bodyMat);
  bodyMesh.castShadow = true;
  bodyMesh.receiveShadow = true;
  group.add(bodyMesh);

  // --- top face: a hexagonal ShapeGeometry with bounding-box UVs ---
  const topGeo = new THREE.ShapeGeometry(new THREE.Shape(corners2d));
  applyBoxUVs(topGeo, radius);
  topGeo.rotateX(-Math.PI / 2); // lay flat in XZ (was XY)

  const topMat = texture
    ? new THREE.MeshStandardMaterial({ map: texture, roughness: 0.95, metalness: 0 })
    : new THREE.MeshStandardMaterial({ color: 0x3a5a40, roughness: 0.9 });
  const topMesh = new THREE.Mesh(topGeo, topMat);
  topMesh.position.y = HEX_TOP_Y + 0.001; // sit just above the body's top face
  topMesh.receiveShadow = true;
  group.add(topMesh);

  // --- rim outline for readability ---
  const rim = new THREE.LineSegments(
    new THREE.EdgesGeometry(body),
    new THREE.LineBasicMaterial({ color: 0x10160e })
  );
  group.add(rim);

  group.userData.top = topMesh;
  return { group, top: topMesh };
}

// Map each vertex's (x,y) in the hexagon bounding box to UV [0..1].
// warapi y=0 is the TOP of the map, so flip V to match.
function applyBoxUVs(geometry, radius) {
  const halfW = radius; // x spans [-r, r]
  const halfH = (Math.sqrt(3) / 2) * radius; // y spans [-h, h]
  const pos = geometry.attributes.position;
  const uv = [];
  for (let i = 0; i < pos.count; i++) {
    const x = pos.getX(i);
    const y = pos.getY(i);
    uv.push((x + halfW) / (2 * halfW), (y + halfH) / (2 * halfH));
  }
  geometry.setAttribute("uv", new THREE.Float32BufferAttribute(uv, 2));
}

/**
 * Convert warapi-normalized (nx, ny) in [0,1] (0,0 = top-left of the hex box)
 * to a world position on the hex top face. Shared by markers + image so the
 * data overlay lines up with the texture exactly.
 */
export function normToWorld(nx, ny, lift = 0) {
  const x = (nx - 0.5) * HEX_WIDTH;
  const z = (ny - 0.5) * HEX_DEPTH; // ny=0 -> north (-Z), ny=1 -> south (+Z)
  return new THREE.Vector3(x, HEX_TOP_Y + lift, z);
}

/** Clamp an arbitrary world point onto the hex top (for manual op markers). */
export function onTop(point, lift = 0) {
  return new THREE.Vector3(point.x, HEX_TOP_Y + lift, point.z);
}
