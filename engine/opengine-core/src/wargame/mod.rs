//! Wargame scaffold (Phase 2). These types/traits compile and are unit-tested
//! today, but the interactive logic (LOS, pathfinding, the mutating `place`) is
//! left as `todo!()` so Phase 2 is purely additive — no rework of Phase 1.
//!
//! Everything operates over the baked terrain grid (`query::BakedGrid`) in
//! warapi-normalized `[0,1]²` coordinates, so positions round-trip through the
//! JS `normToWorld` when this is driven from the browser (compiled to WASM).

pub mod placement;
pub mod query;
pub mod session;
pub mod units;

pub use placement::{
    find_path, place, rules_for, validate_placement, PlaceError, UnitKindRules, World,
};
pub use query::{BakedGrid, Material, TerrainQuery};
pub use session::{Command, Response, Session, UnitDto};
pub use units::{Faction, Kind, Unit, UnitId};
