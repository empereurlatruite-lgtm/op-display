import * as THREE from "three";
import {
  hexCorners,
  HEX_RADIUS,
  HEX_WIDTH,
  HEX_DEPTH,
  HEX_HEIGHT,
  HEX_TOP_Y,
} from "./hex.js";

/**
 * Build the Endless Shore tile as a 3D-elevated hexagon:
 *  - an extruded hex "rock" body for the sides,
 *  - a displaced terrain surface textured with the map (same palette),
 *    its height derived from the map colors (water low, sand mid, rock high).
 *
 * Real heightmap data exists (pickles976/Foxhole-Map-3D, Foxholestats/LogiWaze)
 * but is large/external; this generates relief from the map image as a start.
 *
 * Returns { group, top } where `top` is the displaced surface (raycast target).
 */
export async function buildHexTerrain({
  textureUrl = "./endless-shore.png",
  heightUrl = "./endless-shore-height.png",
  segX = 320,
  segY = 277,
  relief = 0.8,
} = {}) {
  const group = new THREE.Group();

  // --- rock body (sides + base) ---
  const bodyGeo = new THREE.ExtrudeGeometry(new THREE.Shape(hexCorners()), {
    depth: HEX_HEIGHT,
    bevelEnabled: false,
  });
  bodyGeo.rotateX(-Math.PI / 2);
  bodyGeo.translate(0, HEX_HEIGHT, 0);
  bodyGeo.center();
  const body = new THREE.Mesh(
    bodyGeo,
    new THREE.MeshStandardMaterial({ color: 0x6b5d44, roughness: 0.95 })
  );
  body.receiveShadow = true;
  group.add(body);

  // --- terrain surface ---
  let img, heightImg;
  try {
    img = await loadImage(textureUrl);
  } catch {
    img = null;
  }
  try {
    heightImg = await loadImage(heightUrl);
  } catch {
    heightImg = null;
  }

  const geo = new THREE.PlaneGeometry(HEX_WIDTH, HEX_DEPTH, segX, segY);
  let material;

  if (img) {
    // Displace by the real Derp heightmap if present, else infer from map colors.
    const heightSrc = heightImg || img;
    const fromColor = !heightImg;
    const { sample } = imageToHeight(heightSrc, segX + 1, segY + 1, fromColor);
    displace(geo, sample, relief);

    const tex = new THREE.Texture(img);
    tex.colorSpace = THREE.SRGBColorSpace;
    tex.needsUpdate = true;
    tex.anisotropy = 8;
    material = new THREE.MeshStandardMaterial({
      map: tex,
      alphaMap: hexMaskTexture(),
      alphaTest: 0.5,
      roughness: 0.95,
      metalness: 0,
    });
  } else {
    material = new THREE.MeshStandardMaterial({
      color: 0x3a5a40,
      alphaMap: hexMaskTexture(),
      alphaTest: 0.5,
      roughness: 0.9,
    });
  }

  geo.rotateX(-Math.PI / 2); // lay flat; displacement (was +Z) becomes +Y
  geo.computeVertexNormals();

  const surface = new THREE.Mesh(geo, material);
  surface.position.y = HEX_TOP_Y;
  surface.castShadow = true;
  surface.receiveShadow = true;
  group.add(surface);

  // rim outline
  group.add(
    new THREE.LineSegments(
      new THREE.EdgesGeometry(bodyGeo),
      new THREE.LineBasicMaterial({ color: 0x2a2418 })
    )
  );

  group.userData.top = surface;
  return { group, top: surface };
}

function loadImage(url) {
  return new Promise((resolve, reject) => {
    const i = new Image();
    i.crossOrigin = "anonymous";
    i.onload = () => resolve(i);
    i.onerror = reject;
    i.src = url;
  });
}

/**
 * Draw the map to a canvas, classify each pixel's color into a height,
 * smooth it, and return { tex, sample(u,v) } where sample returns 0..1 height.
 */
function imageToHeight(img, gw, gh, fromColor = false) {
  const canvas = document.createElement("canvas");
  canvas.width = img.naturalWidth;
  canvas.height = img.naturalHeight;
  const ctx = canvas.getContext("2d", { willReadFrequently: true });
  ctx.drawImage(img, 0, 0);
  const { data, width, height } = ctx.getImageData(0, 0, canvas.width, canvas.height);

  // Build a low-res height grid by sampling the image.
  // Real heightmap: luminance IS the elevation (white = high cliffs/mesas).
  // Fallback: classify map colors into heights.
  const grid = new Float32Array(gw * gh);
  for (let gy = 0; gy < gh; gy++) {
    for (let gx = 0; gx < gw; gx++) {
      const px = Math.min(width - 1, Math.floor((gx / (gw - 1)) * (width - 1)));
      const py = Math.min(height - 1, Math.floor((gy / (gh - 1)) * (height - 1)));
      const i = (py * width + px) * 4;
      grid[gy * gw + gx] = fromColor
        ? colorToHeight(data[i], data[i + 1], data[i + 2])
        : data[i] / 255; // grayscale luminance
    }
  }
  // Light blur: real heightmap only needs anti-aliasing (keep the cliff steps);
  // the color fallback needs heavy smoothing of speckled rock.
  blur(grid, gw, gh, fromColor ? 8 : 3);

  // Bilinear sampler over UV (v flipped: grid row 0 = image top = v=1).
  const sample = (u, v) => {
    const fx = THREE.MathUtils.clamp(u, 0, 1) * (gw - 1);
    const fy = THREE.MathUtils.clamp(1 - v, 0, 1) * (gh - 1);
    const x0 = Math.floor(fx), y0 = Math.floor(fy);
    const x1 = Math.min(gw - 1, x0 + 1), y1 = Math.min(gh - 1, y0 + 1);
    const tx = fx - x0, ty = fy - y0;
    const a = grid[y0 * gw + x0], b = grid[y0 * gw + x1];
    const c = grid[y1 * gw + x0], d = grid[y1 * gw + x1];
    return (
      a * (1 - tx) * (1 - ty) +
      b * tx * (1 - ty) +
      c * (1 - tx) * ty +
      d * tx * ty
    );
  };

  return { sample };
}

// Map a map-pixel color to a 0..1 height: water lowest, sand mid, rock/cliffs high.
function colorToHeight(r, g, b) {
  const mx = Math.max(r, g, b), mn = Math.min(r, g, b);
  if (b > r && b > g && b - r > 12) return 0.06; // water
  if (mx - mn < 28 && r < 120) return 0.95; // dark rock / cliffs
  if (r > 150 && g > 120 && r >= g && g >= b) return 0.42; // sand
  return 0.5; // roads, structures, vegetation
}

// Simple separable box blur, `passes` times, in place.
function blur(grid, w, h, passes) {
  const tmp = new Float32Array(grid.length);
  for (let p = 0; p < passes; p++) {
    for (let y = 0; y < h; y++)
      for (let x = 0; x < w; x++) {
        let s = 0, n = 0;
        for (let k = -1; k <= 1; k++) {
          const xx = x + k;
          if (xx >= 0 && xx < w) { s += grid[y * w + xx]; n++; }
        }
        tmp[y * w + x] = s / n;
      }
    for (let y = 0; y < h; y++)
      for (let x = 0; x < w; x++) {
        let s = 0, n = 0;
        for (let k = -1; k <= 1; k++) {
          const yy = y + k;
          if (yy >= 0 && yy < h) { s += tmp[yy * w + x]; n++; }
        }
        grid[y * w + x] = s / n;
      }
  }
}

// Displace plane vertices along +Z by sampled height.
function displace(geo, sample, relief) {
  const pos = geo.attributes.position;
  const uv = geo.attributes.uv;
  for (let i = 0; i < pos.count; i++) {
    const h = sample(uv.getX(i), uv.getY(i));
    pos.setZ(i, h * relief);
  }
  pos.needsUpdate = true;
}

// A white flat-top hexagon on transparent — used as alphaMap to clip the
// rectangular terrain plane to the hex outline.
function hexMaskTexture(size = 1024) {
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = size;
  const ctx = canvas.getContext("2d");
  ctx.clearRect(0, 0, size, size);
  ctx.fillStyle = "#fff";
  ctx.beginPath();
  // map hex corners (x in [-W/2,W/2], z in [-D/2,D/2]) into canvas UV space
  hexCorners().forEach((c, idx) => {
    const u = (c.x / HEX_WIDTH + 0.5) * size;
    const w = (c.y / HEX_DEPTH + 0.5) * size;
    idx === 0 ? ctx.moveTo(u, w) : ctx.lineTo(u, w);
  });
  ctx.closePath();
  ctx.fill();
  const tex = new THREE.CanvasTexture(canvas);
  tex.needsUpdate = true;
  return tex;
}
