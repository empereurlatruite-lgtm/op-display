//! Placement + movement rules. `validate_placement` and `rules_for` are real
//! (and tested); the mutating `place` and `find_path` are Phase-2 `todo!()`.

use super::query::TerrainQuery;
use super::units::{Unit, UnitId};
use crate::hex::{hex_corners, HEX_DEPTH, HEX_RADIUS, HEX_WIDTH};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlaceError {
    OutOfBounds,
    Water,
    Cliff,
    Occupied,
}

/// Per-unit-kind terrain constraints.
#[derive(Debug, Clone, Copy)]
pub struct UnitKindRules {
    pub allow_water: bool,
    pub max_slope: f32,
}

/// Terrain rules per unit kind: boats need water, tanks can't climb steep
/// ground, infantry are the most forgiving.
pub fn rules_for(unit: &Unit) -> UnitKindRules {
    match unit {
        Unit::Boat { .. } => UnitKindRules {
            allow_water: true,
            max_slope: f32::INFINITY,
        },
        Unit::Tank { .. } => UnitKindRules {
            allow_water: false,
            max_slope: 0.4,
        },
        Unit::Squad { .. } => UnitKindRules {
            allow_water: false,
            max_slope: 0.8,
        },
    }
}

/// Minimum separation between two placed units, in normalized `[0,1]` distance.
pub const MIN_SEPARATION: f32 = 0.02;

/// The placed board state.
#[derive(Debug, Default, Clone)]
pub struct World {
    pub units: Vec<Unit>,
}

impl World {
    fn occupied_near(&self, at: [f32; 2]) -> bool {
        self.units.iter().any(|u| {
            let p = u.pos();
            let (dx, dy) = (p[0] - at[0], p[1] - at[1]);
            (dx * dx + dy * dy).sqrt() < MIN_SEPARATION
        })
    }
}

/// True if the warapi-normalized `[nx, ny]` falls inside the actual hexagon
/// (not just its bounding box). The grid/texture cover the box, but the clipped
/// corners are off-tile — so a drop there should be rejected.
pub fn point_in_hex(at: [f32; 2]) -> bool {
    // To world XZ centered on the hex.
    let x = (at[0] - 0.5) * HEX_WIDTH;
    let z = (at[1] - 0.5) * HEX_DEPTH;
    // Convex-polygon test: the point is inside iff it's on the same side of
    // every edge (corners are ordered around the hexagon).
    let c = hex_corners(HEX_RADIUS);
    let mut sign = 0.0_f32;
    for i in 0..6 {
        let a = c[i];
        let b = c[(i + 1) % 6];
        let cross = (b[0] - a[0]) * (z - a[1]) - (b[1] - a[1]) * (x - a[0]);
        if cross.abs() < 1e-6 {
            continue; // on an edge
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if cross.signum() != sign {
            return false;
        }
    }
    true
}

/// Check whether `unit` may occupy `at` (`[nx, ny]` in `[0,1]²`) given the
/// terrain. Pure/read-only — used both to validate a drop and to gate `place`.
pub fn validate_placement<Q: TerrainQuery>(
    q: &Q,
    unit: &Unit,
    at: [f32; 2],
) -> Result<(), PlaceError> {
    if !point_in_hex(at) {
        return Err(PlaceError::OutOfBounds);
    }
    let [nx, ny] = at;
    let rules = rules_for(unit);
    if !rules.allow_water && q.is_water(nx, ny) {
        return Err(PlaceError::Water);
    }
    if q.slope_at(nx, ny) > rules.max_slope {
        return Err(PlaceError::Cliff);
    }
    Ok(())
}

/// Validate terrain rules + occupancy, then insert `unit` into `world` and
/// return its id. `unit` should already carry its id and `at` as its position.
pub fn place<Q: TerrainQuery>(
    world: &mut World,
    q: &Q,
    unit: Unit,
    at: [f32; 2],
) -> Result<UnitId, PlaceError> {
    validate_placement(q, &unit, at)?;
    if world.occupied_near(at) {
        return Err(PlaceError::Occupied);
    }
    let id = unit.id();
    world.units.push(unit);
    Ok(id)
}

/// Phase-2: A* over the walkable grid honoring the unit's terrain rules.
pub fn find_path<Q: TerrainQuery>(
    _q: &Q,
    _unit: &Unit,
    _from: [f32; 2],
    _to: [f32; 2],
) -> Option<Vec<[f32; 2]>> {
    todo!("Phase 2: A* over walkable cells")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wargame::query::Material;

    /// Minimal terrain: all water (flat) — exercises the rule gates without a grid.
    struct AllWater;
    impl TerrainQuery for AllWater {
        fn height_at(&self, _: f32, _: f32) -> f32 {
            0.0
        }
        fn material_at(&self, _: f32, _: f32) -> Material {
            Material::Water
        }
        fn slope_at(&self, _: f32, _: f32) -> f32 {
            0.0
        }
    }

    fn boat(pos: [f32; 2]) -> Unit {
        Unit::Boat {
            id: crate::wargame::units::UnitId(1),
            faction: crate::wargame::units::Faction::Neutral,
            pos,
            heading: 0.0,
        }
    }
    fn tank(pos: [f32; 2]) -> Unit {
        Unit::Tank {
            id: crate::wargame::units::UnitId(2),
            faction: crate::wargame::units::Faction::Neutral,
            pos,
            heading: 0.0,
        }
    }

    #[test]
    fn boats_float_tanks_dont() {
        let q = AllWater;
        assert!(validate_placement(&q, &boat([0.5, 0.5]), [0.5, 0.5]).is_ok());
        assert_eq!(
            validate_placement(&q, &tank([0.5, 0.5]), [0.5, 0.5]),
            Err(PlaceError::Water)
        );
    }

    #[test]
    fn out_of_bounds_rejected() {
        let q = AllWater;
        assert_eq!(
            validate_placement(&q, &boat([1.5, 0.5]), [1.5, 0.5]),
            Err(PlaceError::OutOfBounds)
        );
    }

    /// Flat dry land — lets us exercise placement + occupancy.
    struct AllLand;
    impl TerrainQuery for AllLand {
        fn height_at(&self, _: f32, _: f32) -> f32 {
            0.0
        }
        fn material_at(&self, _: f32, _: f32) -> Material {
            Material::Sand
        }
        fn slope_at(&self, _: f32, _: f32) -> f32 {
            0.0
        }
    }

    #[test]
    fn hex_containment_rejects_clipped_corners() {
        assert!(point_in_hex([0.5, 0.5])); // center
        assert!(point_in_hex([0.5, 0.05])); // near top flat edge, inside
        // Bounding-box corners are the clipped-off hex corners -> outside.
        assert!(!point_in_hex([0.0, 0.0]));
        assert!(!point_in_hex([1.0, 1.0]));
        // A unit dropped in a clipped corner is OutOfBounds, not Water/Cliff.
        let q = AllLand;
        assert_eq!(
            validate_placement(&q, &boat([0.0, 0.0]), [0.0, 0.0]),
            Err(PlaceError::OutOfBounds)
        );
    }

    #[test]
    fn place_then_occupancy_blocks_neighbour() {
        let q = AllLand;
        let mut world = World::default();
        assert!(place(&mut world, &q, boat([0.5, 0.5]), [0.5, 0.5]).is_ok());
        assert_eq!(world.units.len(), 1);
        // A second unit right on top is rejected...
        assert_eq!(
            place(&mut world, &q, tank([0.505, 0.5]), [0.505, 0.5]),
            Err(PlaceError::Occupied)
        );
        // ...but one comfortably clear is fine.
        assert!(place(&mut world, &q, tank([0.7, 0.7]), [0.7, 0.7]).is_ok());
        assert_eq!(world.units.len(), 2);
    }
}
