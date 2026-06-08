//! Unit model — the pieces a leader places on the board.

use serde::{Deserialize, Serialize};

/// Stable identifier for a placed unit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UnitId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Faction {
    Warden,
    Colonial,
    Neutral,
}

/// Which kind of piece to spawn (the discriminant without the per-instance data).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Squad,
    Boat,
    Tank,
}

/// A placeable piece. `pos` is warapi-normalized `[nx, ny]` in `[0,1]²`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Unit {
    Squad {
        id: UnitId,
        faction: Faction,
        size: u8,
        pos: [f32; 2],
    },
    Boat {
        id: UnitId,
        faction: Faction,
        pos: [f32; 2],
        heading: f32,
    },
    Tank {
        id: UnitId,
        faction: Faction,
        pos: [f32; 2],
        heading: f32,
    },
}

impl Unit {
    /// Spawn a unit of `kind` at `pos` (warapi-normalized). `size` applies to
    /// squads, `heading` to vehicles; both are ignored where not relevant.
    pub fn spawn(
        kind: Kind,
        id: UnitId,
        faction: Faction,
        pos: [f32; 2],
        size: u8,
        heading: f32,
    ) -> Unit {
        match kind {
            Kind::Squad => Unit::Squad {
                id,
                faction,
                size,
                pos,
            },
            Kind::Boat => Unit::Boat {
                id,
                faction,
                pos,
                heading,
            },
            Kind::Tank => Unit::Tank {
                id,
                faction,
                pos,
                heading,
            },
        }
    }

    pub fn kind(&self) -> Kind {
        match self {
            Unit::Squad { .. } => Kind::Squad,
            Unit::Boat { .. } => Kind::Boat,
            Unit::Tank { .. } => Kind::Tank,
        }
    }

    pub fn id(&self) -> UnitId {
        match *self {
            Unit::Squad { id, .. } | Unit::Boat { id, .. } | Unit::Tank { id, .. } => id,
        }
    }

    pub fn faction(&self) -> Faction {
        match *self {
            Unit::Squad { faction, .. }
            | Unit::Boat { faction, .. }
            | Unit::Tank { faction, .. } => faction,
        }
    }

    pub fn pos(&self) -> [f32; 2] {
        match *self {
            Unit::Squad { pos, .. } | Unit::Boat { pos, .. } | Unit::Tank { pos, .. } => pos,
        }
    }

    /// Squad strength, if this is a squad.
    pub fn size(&self) -> Option<u8> {
        match *self {
            Unit::Squad { size, .. } => Some(size),
            _ => None,
        }
    }

    /// Facing in radians, if this is a vehicle.
    pub fn heading(&self) -> Option<f32> {
        match *self {
            Unit::Boat { heading, .. } | Unit::Tank { heading, .. } => Some(heading),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accessors_work_across_variants() {
        let s = Unit::Squad {
            id: UnitId(1),
            faction: Faction::Warden,
            size: 12,
            pos: [0.5, 0.5],
        };
        assert_eq!(s.id(), UnitId(1));
        assert_eq!(s.faction(), Faction::Warden);
        assert_eq!(s.pos(), [0.5, 0.5]);
    }
}
