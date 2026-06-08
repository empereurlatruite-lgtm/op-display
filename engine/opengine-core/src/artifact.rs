//! Serde structs for the baked terrain artifact (`web/<slug>-terrain.json`).
//!
//! This is the seam between Rust (which computes the terrain) and the web app
//! (which builds the THREE geometry from the grid). It is a *processed grid*,
//! not a full mesh: `height` drives the displacement and `material` is the
//! Phase-2 wargame data layer. See `web/baked.js` for the consumer.

use serde::{Deserialize, Serialize};

pub const ARTIFACT_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Artifact {
    pub version: u32,
    pub meta: Meta,
    pub grid: Grid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
    pub hex_width: f32,
    pub hex_depth: f32,
    pub hex_top_y: f32,
    /// plane subdivisions; grid is `(seg_x+1) x (seg_y+1)`.
    pub seg_x: usize,
    pub seg_y: usize,
    /// default displacement multiplier; applied in JS so it can be tuned without
    /// re-baking (the grid stores relief-free height in `0..1`).
    pub relief: f32,
    pub slug: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grid {
    /// width  = seg_x + 1
    pub w: usize,
    /// height = seg_y + 1
    pub h: usize,
    /// `w*h` relief-free heights in `0..1` (sampled + blurred). Row 0 = image top.
    pub height: Vec<f32>,
    /// `w*h` material codes: 0=water 1=sand 2=road 3=rock. Phase-2 data layer.
    pub material: Vec<u8>,
}

impl Artifact {
    pub fn to_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}
