//! Hex geometry constants + coordinate mapping — a 1:1 mirror of `web/hex.js`.
//!
//! `web/hex.js` stays the JS source of truth for runtime placement; this module
//! mirrors the same constants and formulas so Rust-side terrain/wargame math
//! lands on the exact same world points (preserving `normToWorld` alignment).

/// center -> vertex
pub const HEX_RADIUS: f32 = 6.0;
/// extrusion thickness
pub const HEX_HEIGHT: f32 = 0.6;
/// top face Y
pub const HEX_TOP_Y: f32 = HEX_HEIGHT / 2.0;
/// vertex-to-vertex (x): 2 * HEX_RADIUS
pub const HEX_WIDTH: f32 = 2.0 * HEX_RADIUS;
/// flat-to-flat (z): sqrt(3) * HEX_RADIUS. Literal kept in sync with `web/hex.js`
/// (`Math.sqrt(3) * 6`) since `sqrt` is not permitted in a `const`.
pub const HEX_DEPTH: f32 = 10.392_304_845_413_264;

/// Six flat-top hexagon corners in the XZ plane (matches `hexCorners` in hex.js).
/// Angle offset 0 -> a vertex at +x and -x; returned as `[x, y]` pairs.
pub fn hex_corners(radius: f32) -> [[f32; 2]; 6] {
    let mut pts = [[0.0_f32; 2]; 6];
    for (i, p) in pts.iter_mut().enumerate() {
        let a = (std::f32::consts::PI / 3.0) * i as f32; // 0,60,...,300
        *p = [radius * a.cos(), radius * a.sin()];
    }
    pts
}

/// Convert warapi-normalized `(nx, ny)` in `[0,1]` (`(0,0)` = top-left / NW of the
/// hex box) to a world position `[x, y, z]` on the hex top face. Mirror of
/// `normToWorld` in hex.js so overlays line up with the texture exactly.
pub fn norm_to_world(nx: f32, ny: f32, lift: f32) -> [f32; 3] {
    let x = (nx - 0.5) * HEX_WIDTH;
    let z = (ny - 0.5) * HEX_DEPTH; // ny=0 -> north (-Z), ny=1 -> south (+Z)
    [x, HEX_TOP_Y + lift, z]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_matches_sqrt3() {
        assert!((HEX_DEPTH - 3.0_f32.sqrt() * HEX_RADIUS).abs() < 1e-4);
    }

    #[test]
    fn center_maps_to_origin() {
        let [x, _y, z] = norm_to_world(0.5, 0.5, 0.0);
        assert!(x.abs() < 1e-6 && z.abs() < 1e-6);
    }

    #[test]
    fn corners_span_bounding_box() {
        let c = hex_corners(HEX_RADIUS);
        let max_x = c.iter().map(|p| p[0]).fold(f32::MIN, f32::max);
        assert!((max_x - HEX_RADIUS).abs() < 1e-5);
    }
}
