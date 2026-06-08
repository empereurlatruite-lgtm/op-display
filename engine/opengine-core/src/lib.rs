//! opengine-core — the shared computational core for op-display.
//!
//! Phase 1 (now): generate the hex + terrain. `terrain` ports the heightmap
//! sampling/blur/classification that used to live ad-hoc in `web/terrain.js`,
//! and `bake()` emits a compact grid artifact (`web/<slug>-terrain.json`) that a
//! tiny JS loader turns into the THREE geometry.
//!
//! Phase 2 (future): the `wargame` module lets a leader place squads/boats/tanks
//! over the same baked grid (placement validity, line-of-sight, pathfinding).
//!
//! This crate is deliberately free of filesystem / image-decoding / CLI concerns
//! so it compiles unchanged to WASM later — the native `op-engine` binary does
//! PNG I/O and feeds raw pixels in.

pub mod artifact;
pub mod hex;
pub mod terrain;
pub mod wargame;

pub use artifact::{Artifact, Grid, Meta};
