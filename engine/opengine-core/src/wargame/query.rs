//! Terrain queries over the baked grid — the data layer the wargame stands on.
//!
//! `BakedGrid` is loadable straight from the Phase-1 artifact, so the same grid
//! that renders the terrain answers walkability / slope / (later) line-of-sight.

use crate::artifact::Artifact;
use crate::hex::{HEX_DEPTH, HEX_WIDTH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Material {
    Water,
    Sand,
    Road,
    Rock,
}

impl Material {
    pub fn from_code(c: u8) -> Material {
        match c {
            0 => Material::Water,
            1 => Material::Sand,
            3 => Material::Rock,
            _ => Material::Road,
        }
    }
}

/// Slope (world rise/run) at or above which terrain is considered an impassable
/// cliff for ground units by default.
pub const CLIFF_SLOPE: f32 = 0.6;

/// Read-only terrain queries in warapi-normalized `[0,1]²` coordinates.
pub trait TerrainQuery {
    /// Relief-scaled world height at `(nx, ny)`.
    fn height_at(&self, nx: f32, ny: f32) -> f32;
    fn material_at(&self, nx: f32, ny: f32) -> Material;
    /// Local slope magnitude (world rise / run).
    fn slope_at(&self, nx: f32, ny: f32) -> f32;

    fn is_water(&self, nx: f32, ny: f32) -> bool {
        matches!(self.material_at(nx, ny), Material::Water)
    }
    fn is_cliff(&self, nx: f32, ny: f32) -> bool {
        self.slope_at(nx, ny) >= CLIFF_SLOPE
    }
    fn is_walkable(&self, nx: f32, ny: f32) -> bool {
        !self.is_water(nx, ny) && !self.is_cliff(nx, ny)
    }

    /// Whether an observer at `a` can see a target at `b` over the terrain.
    /// Marches the straight sight ray (from each end's ground + a small eye
    /// height) and fails if intervening ground rises above the ray.
    fn line_of_sight(&self, a: [f32; 2], b: [f32; 2]) -> bool {
        const STEPS: usize = 64;
        const EYE: f32 = 0.05; // world units above ground at both ends
        let ha = self.height_at(a[0], a[1]) + EYE;
        let hb = self.height_at(b[0], b[1]) + EYE;
        for i in 1..STEPS {
            let t = i as f32 / STEPS as f32;
            let nx = a[0] + (b[0] - a[0]) * t;
            let ny = a[1] + (b[1] - a[1]) * t;
            let ground = self.height_at(nx, ny);
            let ray = ha + (hb - ha) * t;
            if ground > ray + 1e-4 {
                return false; // a ridge breaks the line
            }
        }
        true
    }
}

/// The baked terrain grid, ready for queries.
#[derive(Debug, Clone)]
pub struct BakedGrid {
    pub w: usize,
    pub h: usize,
    pub relief: f32,
    height: Vec<f32>,
    material: Vec<u8>,
}

impl BakedGrid {
    pub fn from_artifact(art: &Artifact) -> Self {
        BakedGrid {
            w: art.grid.w,
            h: art.grid.h,
            relief: art.meta.relief,
            height: art.grid.height.clone(),
            material: art.grid.material.clone(),
        }
    }

    #[inline]
    fn cell(nx: f32, n: usize) -> (usize, usize, f32) {
        let f = nx.clamp(0.0, 1.0) * (n - 1) as f32;
        let i0 = f.floor() as usize;
        let i1 = (i0 + 1).min(n - 1);
        (i0, i1, f - i0 as f32)
    }

    /// Bilinear relief-free height in `0..1` (row 0 = ny 0 = north).
    fn raw_height(&self, nx: f32, ny: f32) -> f32 {
        let (x0, x1, tx) = Self::cell(nx, self.w);
        let (y0, y1, ty) = Self::cell(ny, self.h);
        let a = self.height[y0 * self.w + x0];
        let b = self.height[y0 * self.w + x1];
        let c = self.height[y1 * self.w + x0];
        let d = self.height[y1 * self.w + x1];
        a * (1.0 - tx) * (1.0 - ty) + b * tx * (1.0 - ty) + c * (1.0 - tx) * ty + d * tx * ty
    }
}

impl TerrainQuery for BakedGrid {
    fn height_at(&self, nx: f32, ny: f32) -> f32 {
        self.raw_height(nx, ny) * self.relief
    }

    fn material_at(&self, nx: f32, ny: f32) -> Material {
        let (x0, _, tx) = Self::cell(nx, self.w);
        let (y0, _, ty) = Self::cell(ny, self.h);
        let x = if tx >= 0.5 { (x0 + 1).min(self.w - 1) } else { x0 };
        let y = if ty >= 0.5 { (y0 + 1).min(self.h - 1) } else { y0 };
        Material::from_code(self.material[y * self.w + x])
    }

    fn slope_at(&self, nx: f32, ny: f32) -> f32 {
        // Central finite difference in world units over one grid cell.
        let dnx = 1.0 / (self.w - 1) as f32;
        let dny = 1.0 / (self.h - 1) as f32;
        let dhx = self.height_at(nx + dnx, ny) - self.height_at(nx - dnx, ny);
        let dhy = self.height_at(nx, ny + dny) - self.height_at(nx, ny - dny);
        let run_x = 2.0 * dnx * HEX_WIDTH;
        let run_y = 2.0 * dny * HEX_DEPTH;
        ((dhx / run_x).powi(2) + (dhy / run_y).powi(2)).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{Grid, Meta, ARTIFACT_VERSION};

    fn grid_2x2(height: Vec<f32>, material: Vec<u8>) -> BakedGrid {
        let art = Artifact {
            version: ARTIFACT_VERSION,
            meta: Meta {
                hex_width: HEX_WIDTH,
                hex_depth: HEX_DEPTH,
                hex_top_y: 0.3,
                seg_x: 1,
                seg_y: 1,
                relief: 1.0,
                slug: "t".into(),
                source: "t".into(),
            },
            grid: Grid { w: 2, h: 2, height, material },
        };
        BakedGrid::from_artifact(&art)
    }

    #[test]
    fn material_and_height_round_trip() {
        // NW=water/flat, SE=rock/high.
        let g = grid_2x2(vec![0.0, 0.0, 0.0, 1.0], vec![0, 2, 2, 3]);
        assert_eq!(g.material_at(0.0, 0.0), Material::Water);
        assert_eq!(g.material_at(1.0, 1.0), Material::Rock);
        assert!(g.is_water(0.0, 0.0));
        assert!(g.height_at(1.0, 1.0) > g.height_at(0.0, 0.0));
    }

    // 5x5 flat grid with a single tall peak at the center.
    fn grid_with_peak() -> BakedGrid {
        let mut height = vec![0.0_f32; 25];
        height[2 * 5 + 2] = 1.0; // center peak
        let art = Artifact {
            version: ARTIFACT_VERSION,
            meta: Meta {
                hex_width: HEX_WIDTH,
                hex_depth: HEX_DEPTH,
                hex_top_y: 0.3,
                seg_x: 4,
                seg_y: 4,
                relief: 1.0,
                slug: "t".into(),
                source: "t".into(),
            },
            grid: Grid { w: 5, h: 5, height, material: vec![1; 25] },
        };
        BakedGrid::from_artifact(&art)
    }

    #[test]
    fn line_of_sight_blocked_by_ridge() {
        let g = grid_with_peak();
        // Across the central peak: blocked.
        assert!(!g.line_of_sight([0.0, 0.5], [1.0, 0.5]));
        // Along the flat top edge: clear.
        assert!(g.line_of_sight([0.0, 0.0], [1.0, 0.0]));
    }
}
