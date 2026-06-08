//! Terrain computation — the Phase-1 core. A faithful port of the height
//! sampling / blur / classification that used to run in `web/terrain.js`, so the
//! baked grid reproduces the previous visual result.
//!
//! Everything here takes raw pixel slices (no image decoding) so the crate stays
//! WASM-ready; `op-engine` decodes the PNGs and feeds the buffers in.

use crate::artifact::{Artifact, Grid, Meta, ARTIFACT_VERSION};
use crate::hex::{HEX_DEPTH, HEX_TOP_Y, HEX_WIDTH};

/// A decoded pixel buffer. `data` is row-major; `channels` per pixel.
pub struct PixelBuf<'a> {
    pub data: &'a [u8],
    pub w: usize,
    pub h: usize,
    pub channels: usize,
}

/// Nearest-pixel image->grid index, matching terrain.js:
/// `px = min(iw-1, floor(gx/(gw-1) * (iw-1)))`.
#[inline]
fn src_index(g: usize, gn: usize, n: usize) -> usize {
    if gn <= 1 {
        return 0;
    }
    let p = ((g as f32 / (gn - 1) as f32) * (n - 1) as f32).floor() as usize;
    p.min(n - 1)
}

/// Sample a single-channel (luminance) source into a `gw x gh` height grid in
/// `0..1`. Mirror of the `fromColor=false` branch of `imageToHeight`.
pub fn sample_height_grid(src: &PixelBuf, gw: usize, gh: usize) -> Vec<f32> {
    let mut grid = vec![0.0_f32; gw * gh];
    for gy in 0..gh {
        let py = src_index(gy, gh, src.h);
        for gx in 0..gw {
            let px = src_index(gx, gw, src.w);
            let i = (py * src.w + px) * src.channels;
            grid[gy * gw + gx] = src.data[i] as f32 / 255.0;
        }
    }
    grid
}

/// Infer a height grid from map *colors* (the no-heightmap fallback). Mirror of
/// the `fromColor=true` branch of `imageToHeight`.
pub fn sample_height_from_color(src: &PixelBuf, gw: usize, gh: usize) -> Vec<f32> {
    let mut grid = vec![0.0_f32; gw * gh];
    for gy in 0..gh {
        let py = src_index(gy, gh, src.h);
        for gx in 0..gw {
            let px = src_index(gx, gw, src.w);
            let i = (py * src.w + px) * src.channels;
            grid[gy * gw + gx] = color_to_height(src.data[i], src.data[i + 1], src.data[i + 2]);
        }
    }
    grid
}

/// Classify each grid cell of the *texture* into a material code:
/// 0=water 1=sand 2=road 3=rock. Phase-2 wargame data layer.
pub fn classify_material(src: &PixelBuf, gw: usize, gh: usize) -> Vec<u8> {
    let mut mat = vec![2_u8; gw * gh];
    for gy in 0..gh {
        let py = src_index(gy, gh, src.h);
        for gx in 0..gw {
            let px = src_index(gx, gw, src.w);
            let i = (py * src.w + px) * src.channels;
            mat[gy * gw + gx] = material_code(src.data[i], src.data[i + 1], src.data[i + 2]);
        }
    }
    mat
}

/// Map a map-pixel color to a `0..1` height. Mirror of `colorToHeight` in
/// terrain.js (water lowest, sand mid, rock/cliffs high).
pub fn color_to_height(r: u8, g: u8, b: u8) -> f32 {
    let (r, g, b) = (r as i32, g as i32, b as i32);
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    if b > r && b > g && b - r > 12 {
        0.06 // water
    } else if mx - mn < 28 && r < 120 {
        0.95 // dark rock / cliffs
    } else if r > 150 && g > 120 && r >= g && g >= b {
        0.42 // sand
    } else {
        0.5 // roads, structures, vegetation
    }
}

/// Same classification as `color_to_height`, but as a discrete material code.
pub fn material_code(r: u8, g: u8, b: u8) -> u8 {
    let (r, g, b) = (r as i32, g as i32, b as i32);
    let mx = r.max(g).max(b);
    let mn = r.min(g).min(b);
    if b > r && b > g && b - r > 12 {
        0 // water
    } else if mx - mn < 28 && r < 120 {
        3 // rock / cliff
    } else if r > 150 && g > 120 && r >= g && g >= b {
        1 // sand
    } else {
        2 // road / structure / vegetation
    }
}

/// Separable 3x3 box blur, `passes` times, edge-clamped. Verbatim port of
/// `blur()` in terrain.js.
pub fn blur(grid: &mut [f32], w: usize, h: usize, passes: usize) {
    let mut tmp = vec![0.0_f32; grid.len()];
    for _ in 0..passes {
        for y in 0..h {
            for x in 0..w {
                let (mut s, mut n) = (0.0_f32, 0.0_f32);
                for k in -1i32..=1 {
                    let xx = x as i32 + k;
                    if xx >= 0 && (xx as usize) < w {
                        s += grid[y * w + xx as usize];
                        n += 1.0;
                    }
                }
                tmp[y * w + x] = s / n;
            }
        }
        for y in 0..h {
            for x in 0..w {
                let (mut s, mut n) = (0.0_f32, 0.0_f32);
                for k in -1i32..=1 {
                    let yy = y as i32 + k;
                    if yy >= 0 && (yy as usize) < h {
                        s += tmp[yy as usize * w + x];
                        n += 1.0;
                    }
                }
                grid[y * w + x] = s / n;
            }
        }
    }
}

/// Build the full baked artifact from optional height + texture buffers.
///
/// `height` is a single-channel luminance buffer (the real Derp heightmap). When
/// absent, relief is inferred from the texture colors (the `--from-color` path).
/// `texture` drives the material grid (and the color fallback).
#[allow(clippy::too_many_arguments)]
pub fn bake(
    height: Option<&PixelBuf>,
    texture: Option<&PixelBuf>,
    seg_x: usize,
    seg_y: usize,
    relief: f32,
    slug: &str,
    source: &str,
) -> Artifact {
    let gw = seg_x + 1;
    let gh = seg_y + 1;
    let from_color = height.is_none();

    let mut hgrid = if let Some(h) = height {
        sample_height_grid(h, gw, gh)
    } else if let Some(t) = texture {
        sample_height_from_color(t, gw, gh)
    } else {
        vec![0.0_f32; gw * gh]
    };

    // Real heightmap only needs anti-aliasing (keep cliff steps); the color
    // fallback needs heavy smoothing of speckled rock — same passes as the JS.
    blur(&mut hgrid, gw, gh, if from_color { 8 } else { 3 });

    // Round to 4dp to keep the committed artifact compact + diff-friendly.
    for v in hgrid.iter_mut() {
        *v = (*v * 10_000.0).round() / 10_000.0;
    }

    let material = match texture {
        Some(t) => classify_material(t, gw, gh),
        None => vec![2_u8; gw * gh],
    };

    Artifact {
        version: ARTIFACT_VERSION,
        meta: Meta {
            hex_width: HEX_WIDTH,
            hex_depth: HEX_DEPTH,
            hex_top_y: HEX_TOP_Y,
            seg_x,
            seg_y,
            relief,
            slug: slug.to_string(),
            source: source.to_string(),
        },
        grid: Grid {
            w: gw,
            h: gh,
            height: hgrid,
            material,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_classification_matches_js() {
        assert_eq!(material_code(40, 80, 160), 0); // blue -> water
        assert_eq!(material_code(30, 30, 30), 3); // dark low-sat -> rock
        assert_eq!(material_code(200, 170, 120), 1); // warm -> sand
        assert_eq!(material_code(140, 140, 90), 2); // else -> road/veg
        assert!((color_to_height(40, 80, 160) - 0.06).abs() < 1e-6);
    }

    #[test]
    fn blur_reduces_a_spike() {
        let (w, h) = (5, 5);
        let mut g = vec![0.0_f32; w * h];
        g[2 * w + 2] = 1.0;
        let peak_before = g[2 * w + 2];
        blur(&mut g, w, h, 1);
        assert!(g[2 * w + 2] < peak_before);
        assert!(g[2 * w + 1] > 0.0); // spread to neighbour
    }

    #[test]
    fn bake_produces_expected_grid_dims() {
        // 2x2 luminance source, 4x3 plane subdivisions.
        let lum = [0u8, 255, 128, 64];
        let src = PixelBuf { data: &lum, w: 2, h: 2, channels: 1 };
        let art = bake(Some(&src), None, 4, 3, 0.8, "test", "lum");
        assert_eq!(art.grid.w, 5);
        assert_eq!(art.grid.h, 4);
        assert_eq!(art.grid.height.len(), 20);
        assert_eq!(art.grid.material.len(), 20); // defaults to road when no texture
        assert!(art.grid.height.iter().all(|&v| (0.0..=1.0).contains(&v)));
    }
}
