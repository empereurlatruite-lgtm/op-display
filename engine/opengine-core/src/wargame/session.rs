//! Interactive session — the JSON command layer the browser drives (via the
//! `opengine-wasm` ABI). Kept here in core so it is testable natively; the wasm
//! crate is only memory plumbing around `Session::dispatch_json`.
//!
//! Protocol: JSON in, JSON out. One `Session` holds the loaded terrain grid and
//! the placed `World`. Positions are warapi-normalized `[nx, ny]` so they
//! round-trip through the JS `normToWorld`.

use serde::{Deserialize, Serialize};

use crate::artifact::Artifact;
use crate::wargame::placement::{place, rules_for, validate_placement, World};
use crate::wargame::query::{BakedGrid, TerrainQuery};
use crate::wargame::units::{Faction, Kind, Unit, UnitId};

/// A command from the UI.
#[derive(Debug, Deserialize)]
#[serde(tag = "cmd", rename_all = "camelCase")]
pub enum Command {
    /// Load the baked terrain artifact (grid + meta) for queries.
    LoadArtifact { artifact: Box<Artifact> },
    /// Preview whether a unit could be placed at `at` (for hover feedback).
    Validate {
        kind: Kind,
        at: [f32; 2],
    },
    /// Place a unit; returns its new id or an error.
    Place {
        kind: Kind,
        faction: Faction,
        at: [f32; 2],
        #[serde(default = "default_size")]
        size: u8,
        #[serde(default)]
        heading: f32,
    },
    /// All placed units.
    List,
    /// Remove one unit by id.
    Remove { id: u32 },
    /// Remove all units.
    Clear,
    /// Whether an observer at `from` can see a target at `to`.
    LineOfSight { from: [f32; 2], to: [f32; 2] },
}

fn default_size() -> u8 {
    9
}

/// A unit flattened for the wire (the `Unit` enum is awkward to serialize raw).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UnitDto {
    pub id: u32,
    pub kind: Kind,
    pub faction: Faction,
    pub pos: [f32; 2],
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u8>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading: Option<f32>,
}

impl From<&Unit> for UnitDto {
    fn from(u: &Unit) -> Self {
        UnitDto {
            id: u.id().0,
            kind: u.kind(),
            faction: u.faction(),
            pos: u.pos(),
            size: u.size(),
            heading: u.heading(),
        }
    }
}

/// The response envelope. Only the fields relevant to a command are populated.
#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Response {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub valid: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visible: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<Vec<UnitDto>>,
}

impl Response {
    fn ok() -> Self {
        Response {
            ok: true,
            ..Default::default()
        }
    }
    fn err(msg: impl Into<String>) -> Self {
        Response {
            ok: false,
            error: Some(msg.into()),
            ..Default::default()
        }
    }
}

/// Holds the loaded terrain + placed units across commands.
#[derive(Default)]
pub struct Session {
    grid: Option<BakedGrid>,
    world: World,
    next_id: u32,
}

impl Session {
    pub fn new() -> Self {
        Session::default()
    }

    /// Parse + run one command, returning the JSON response. Never panics on bad
    /// input — malformed JSON becomes an `ok: false` response.
    pub fn dispatch_json(&mut self, input: &str) -> String {
        let resp = match serde_json::from_str::<Command>(input) {
            Ok(cmd) => self.dispatch(cmd),
            Err(e) => Response::err(format!("bad command: {e}")),
        };
        serde_json::to_string(&resp)
            .unwrap_or_else(|_| r#"{"ok":false,"error":"serialize failed"}"#.to_string())
    }

    pub fn dispatch(&mut self, cmd: Command) -> Response {
        match cmd {
            Command::LoadArtifact { artifact } => {
                self.grid = Some(BakedGrid::from_artifact(&artifact));
                Response::ok()
            }
            Command::Validate { kind, at } => match &self.grid {
                None => Response::err("no terrain loaded"),
                Some(grid) => {
                    let probe = Unit::spawn(kind, UnitId(0), Faction::Neutral, at, 1, 0.0);
                    match validate_placement(grid, &probe, at) {
                        Ok(()) => Response {
                            ok: true,
                            valid: Some(true),
                            ..Default::default()
                        },
                        Err(e) => Response {
                            ok: true,
                            valid: Some(false),
                            error: Some(format!("{e:?}")),
                            ..Default::default()
                        },
                    }
                }
            },
            Command::Place {
                kind,
                faction,
                at,
                size,
                heading,
            } => {
                let Some(grid) = self.grid.as_ref() else {
                    return Response::err("no terrain loaded");
                };
                let id = UnitId(self.next_id);
                let unit = Unit::spawn(kind, id, faction, at, size, heading);
                match place(&mut self.world, grid, unit, at) {
                    Ok(_) => {
                        self.next_id += 1;
                        Response {
                            ok: true,
                            id: Some(id.0),
                            ..Default::default()
                        }
                    }
                    Err(e) => Response {
                        ok: false,
                        error: Some(format!("{e:?}")),
                        ..Default::default()
                    },
                }
            }
            Command::List => Response {
                ok: true,
                units: Some(self.world.units.iter().map(UnitDto::from).collect()),
                ..Default::default()
            },
            Command::Remove { id } => {
                let before = self.world.units.len();
                self.world.units.retain(|u| u.id().0 != id);
                if self.world.units.len() < before {
                    Response::ok()
                } else {
                    Response::err("no such unit")
                }
            }
            Command::Clear => {
                self.world.units.clear();
                Response::ok()
            }
            Command::LineOfSight { from, to } => match &self.grid {
                None => Response::err("no terrain loaded"),
                Some(grid) => Response {
                    ok: true,
                    visible: Some(grid.line_of_sight(from, to)),
                    ..Default::default()
                },
            },
        }
    }

    /// Terrain rules a unit kind obeys (exposed for UI hints).
    pub fn rules_json(kind: Kind) -> String {
        let probe = Unit::spawn(kind, UnitId(0), Faction::Neutral, [0.5, 0.5], 1, 0.0);
        let r = rules_for(&probe);
        format!(
            r#"{{"allowWater":{},"maxSlope":{}}}"#,
            r.allow_water, r.max_slope
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // 2x2 grid: NW corner water+flat, everything else dry sand, SE raised.
    fn load() -> Session {
        let mut s = Session::new();
        let artifact = serde_json::json!({
            "version": 1,
            "meta": {
                "hexWidth": 12.0, "hexDepth": 10.3923, "hexTopY": 0.3,
                "segX": 1, "segY": 1, "relief": 1.0, "slug": "t", "source": "t"
            },
            "grid": { "w": 2, "h": 2, "height": [0.0, 0.0, 0.0, 0.2], "material": [0, 1, 1, 1] }
        })
        .to_string();
        let r = s.dispatch_json(&format!(r#"{{"cmd":"loadArtifact","artifact":{artifact}}}"#));
        assert!(r.contains(r#""ok":true"#), "{r}");
        s
    }

    #[test]
    fn place_squad_on_land_ok_tank_on_water_rejected() {
        let mut s = load();
        // squad on dry interior ground (SE-ish, inside the hex)
        let r = s.dispatch_json(r#"{"cmd":"place","kind":"squad","faction":"warden","at":[0.5,0.8]}"#);
        assert!(r.contains(r#""ok":true"#) && r.contains(r#""id":0"#), "{r}");
        // tank on an interior water cell (NW-ish) -> rejected
        let r = s.dispatch_json(r#"{"cmd":"place","kind":"tank","faction":"colonial","at":[0.3,0.3]}"#);
        assert!(r.contains(r#""ok":false"#) && r.contains("Water"), "{r}");
        // boat on the same water -> ok
        let r = s.dispatch_json(r#"{"cmd":"place","kind":"boat","faction":"colonial","at":[0.3,0.3]}"#);
        assert!(r.contains(r#""ok":true"#), "{r}");
        // clipped corner -> OutOfBounds, not Water
        let r = s.dispatch_json(r#"{"cmd":"place","kind":"boat","faction":"colonial","at":[0.0,0.0]}"#);
        assert!(r.contains(r#""ok":false"#) && r.contains("OutOfBounds"), "{r}");
    }

    #[test]
    fn line_of_sight_command() {
        let mut s = load();
        let r = s.dispatch_json(r#"{"cmd":"lineOfSight","from":[0.4,0.5],"to":[0.6,0.5]}"#);
        assert!(r.contains(r#""ok":true"#) && r.contains(r#""visible":"#), "{r}");
    }

    #[test]
    fn list_and_clear() {
        let mut s = load();
        s.dispatch_json(r#"{"cmd":"place","kind":"squad","faction":"warden","at":[0.5,0.8]}"#);
        let r = s.dispatch_json(r#"{"cmd":"list"}"#);
        assert!(r.contains(r#""kind":"squad""#) && r.contains(r#""faction":"warden""#), "{r}");
        let r = s.dispatch_json(r#"{"cmd":"clear"}"#);
        assert!(r.contains(r#""ok":true"#));
        let r = s.dispatch_json(r#"{"cmd":"list"}"#);
        assert!(r.contains(r#""units":[]"#), "{r}");
    }

    #[test]
    fn bad_json_is_graceful() {
        let mut s = Session::new();
        let r = s.dispatch_json("not json");
        assert!(r.contains(r#""ok":false"#));
    }
}
