//! Import immutable Albion data snapshots into a playable world database.
//!
//! The snapshot stays outside career saves. Creating a new snapshot world never
//! mutates an existing career; New Game reads the resulting world JSON once.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use tauri::{AppHandle, Manager};

const SNAPSHOT_WORLD_FILENAME: &str = "albion-snapshot-world.json";
const SNAPSHOT_ERROR: &str = "be.error.worldReadFileFailed";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AlbionSnapshotStatus {
    pub cached: bool,
    pub world_database_path: Option<String>,
    pub season: Option<String>,
    pub snapshot_hash: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotDocument {
    schema_version: u32,
    season: String,
    clubs: Vec<SnapshotClub>,
    players: Vec<SnapshotPlayer>,
    content_hash: String,
}

#[derive(Debug, Deserialize)]
struct SnapshotClub {
    id: String,
    name: String,
    #[serde(default = "default_country")]
    country: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotPlayer {
    id: String,
    name: String,
    club_id: String,
    position: String,
    #[serde(default)]
    rating: Option<SnapshotRating>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotRating {
    overall: u8,
    potential: u8,
}

fn default_country() -> String {
    "ENG".to_string()
}

fn snapshot_world_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("databases").join(SNAPSHOT_WORLD_FILENAME))
        .map_err(|_| SNAPSHOT_ERROR.to_string())
}

fn canonical_json(value: &Value) -> Result<String, String> {
    Ok(match value {
        Value::Null => "null".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => {
            serde_json::to_string(value).map_err(|_| SNAPSHOT_ERROR.to_string())?
        }
        Value::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(canonical_json)
                .collect::<Result<Vec<_>, _>>()?
                .join(",")
        ),
        Value::Object(values) => {
            let mut keys = values.keys().collect::<Vec<_>>();
            keys.sort_unstable();
            let pairs = keys
                .into_iter()
                .map(|key| {
                    Ok(format!(
                        "{}:{}",
                        serde_json::to_string(key).map_err(|_| SNAPSHOT_ERROR.to_string())?,
                        canonical_json(&values[key])?
                    ))
                })
                .collect::<Result<Vec<_>, String>>()?;
            format!("{{{}}}", pairs.join(","))
        }
    })
}

fn snapshot_hash(raw: &Value) -> Result<String, String> {
    let mut content = raw.clone();
    content
        .as_object_mut()
        .ok_or_else(|| SNAPSHOT_ERROR.to_string())?
        .remove("contentHash");
    Ok(format!(
        "sha256:{:x}",
        Sha256::digest(canonical_json(&content)?.as_bytes())
    ))
}

fn verify_snapshot(raw: &Value) -> Result<SnapshotDocument, String> {
    let document: SnapshotDocument =
        serde_json::from_value(raw.clone()).map_err(|_| SNAPSHOT_ERROR.to_string())?;
    if document.schema_version != 1 || document.content_hash != snapshot_hash(raw)? {
        return Err(SNAPSHOT_ERROR.to_string());
    }
    if document.clubs.is_empty() || document.players.is_empty() {
        return Err(SNAPSHOT_ERROR.to_string());
    }
    let club_ids = document
        .clubs
        .iter()
        .map(|club| club.id.as_str())
        .collect::<HashSet<_>>();
    if club_ids.len() != document.clubs.len()
        || document
            .players
            .iter()
            .any(|player| !club_ids.contains(player.club_id.as_str()))
    {
        return Err(SNAPSHOT_ERROR.to_string());
    }
    Ok(document)
}

fn snapshot_position(position: &str) -> domain::player::Position {
    match position.to_ascii_uppercase().as_str() {
        "GK" | "GOALKEEPER" => domain::player::Position::Goalkeeper,
        "CB" | "LB" | "RB" | "LWB" | "RWB" | "DEFENDER" => domain::player::Position::Defender,
        "ST" | "CF" | "FW" | "FORWARD" => domain::player::Position::Forward,
        _ => domain::player::Position::Midfielder,
    }
}

fn short_name(name: &str) -> String {
    name.split_whitespace()
        .filter_map(|word| word.chars().next())
        .take(3)
        .collect::<String>()
        .to_ascii_uppercase()
}

fn apply_snapshot_rating(player: &mut domain::player::Player, rating: &SnapshotRating) {
    // The pipeline labels this estimate as `albion_rating_v1`. Source facts do
    // not claim to contain in-game attributes, so we translate its bounded
    // ability summary into a coherent baseline before deriving the live rating.
    let overall = rating.overall.clamp(1, 99);
    let attributes = &mut player.attributes;
    attributes.pace = overall;
    attributes.stamina = overall;
    attributes.strength = overall;
    attributes.agility = overall;
    attributes.passing = overall;
    attributes.shooting = overall;
    attributes.tackling = overall;
    attributes.dribbling = overall;
    attributes.defending = overall;
    attributes.positioning = overall;
    attributes.vision = overall;
    attributes.decisions = overall;
    attributes.composure = overall;
    attributes.aggression = overall;
    attributes.teamwork = overall;
    attributes.leadership = overall;
    attributes.handling = overall;
    attributes.reflexes = overall;
    attributes.aerial = overall;
    player.potential = rating.potential.clamp(overall, 99);
}

fn overlay_snapshot_roster(
    world: &mut ofm_core::generator::WorldData,
    mut clubs: Vec<SnapshotClub>,
    players: Vec<SnapshotPlayer>,
) -> Result<(), String> {
    clubs.sort_by(|left, right| left.id.cmp(&right.id));
    let mut generated = world
        .teams
        .iter()
        .enumerate()
        .filter(|(_, team)| team.country == "ENG")
        .map(|(index, team)| (index, team.id.clone(), team.reputation))
        .collect::<Vec<_>>();
    generated.sort_by(
        |(_, left_id, left_reputation), (_, right_id, right_reputation)| {
            right_reputation
                .cmp(left_reputation)
                .then_with(|| left_id.cmp(right_id))
        },
    );
    if generated.len() < clubs.len() {
        return Err(SNAPSHOT_ERROR.to_string());
    }

    let team_ids = generated
        .into_iter()
        .take(clubs.len())
        .zip(clubs)
        .map(|((index, generated_id, _), source)| {
            let team = &mut world.teams[index];
            team.name = source.name.clone();
            team.short_name = short_name(&source.name);
            team.country = source.country;
            (source.id, generated_id)
        })
        .collect::<HashMap<_, _>>();
    let targets = team_ids.values().cloned().collect::<HashSet<_>>();
    let templates = world.players.clone();
    world.players.retain(|player| {
        !player
            .team_id
            .as_ref()
            .is_some_and(|team_id| targets.contains(team_id))
    });
    for source in players {
        let team_id = team_ids
            .get(&source.club_id)
            .ok_or_else(|| SNAPSHOT_ERROR.to_string())?;
        let position = snapshot_position(&source.position);
        let template = templates
            .iter()
            .find(|player| {
                player.team_id.as_deref() == Some(team_id.as_str())
                    && player.natural_position.to_group_position() == position.to_group_position()
            })
            .or_else(|| {
                templates
                    .iter()
                    .find(|player| player.team_id.as_deref() == Some(team_id.as_str()))
            })
            .ok_or_else(|| SNAPSHOT_ERROR.to_string())?;
        let mut player = template.clone();
        player.id = format!("snapshot-{}", source.id);
        player.match_name = source.name.clone();
        player.full_name = source.name;
        player.position = position.clone();
        player.natural_position = position;
        player.team_id = Some(team_id.clone());
        player.career.clear();
        player.movement_history.clear();
        player.transfer_offers.clear();
        player.loan_offers.clear();
        player.active_loan = None;
        if let Some(rating) = &source.rating {
            apply_snapshot_rating(&mut player, rating);
        }
        ofm_core::player_rating::refresh_player_derived(&mut player, 2026);
        world.players.push(player);
    }
    Ok(())
}

fn build_snapshot_world(
    snapshot: SnapshotDocument,
) -> Result<ofm_core::generator::WorldData, String> {
    let mut world = ofm_core::generator::generate_world_data(
        &ofm_core::generator::DefinitionSources::embedded_only(),
    );
    overlay_snapshot_roster(&mut world, snapshot.clubs, snapshot.players)?;
    world.name = format!("Albion Snapshot {}", snapshot.season);
    world.description =
        "Playable world generated from an immutable Albion data snapshot".to_string();
    world.metadata = ofm_core::generator::WorldDataMetadata {
        format_version: 2,
        world_id: snapshot.content_hash,
        kind: ofm_core::generator::WorldDataKind::RosterBaseline,
        base_year: snapshot.season.get(0..4).and_then(|year| year.parse().ok()),
        snapshot_date: Some(snapshot.season),
    };
    Ok(world)
}

#[tauri::command]
pub fn get_albion_snapshot_status(app: AppHandle) -> Result<AlbionSnapshotStatus, String> {
    let path = snapshot_world_path(&app)?;
    let metadata = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<ofm_core::generator::WorldData>(&raw).ok())
        .map(|world| (world.metadata.snapshot_date, world.metadata.world_id));
    Ok(AlbionSnapshotStatus {
        cached: path.is_file(),
        world_database_path: path.is_file().then(|| path.to_string_lossy().to_string()),
        season: metadata
            .as_ref()
            .and_then(|(season, _)| season.clone()),
        snapshot_hash: metadata.map(|(_, hash)| hash),
    })
}

/// Imports only a published, hash-verified snapshot and writes a fresh world
/// database. Existing careers are not opened or modified here.
#[tauri::command]
pub fn import_albion_snapshot(
    app: AppHandle,
    source_path: String,
) -> Result<AlbionSnapshotStatus, String> {
    let raw: Value = serde_json::from_str(
        &std::fs::read_to_string(source_path).map_err(|_| SNAPSHOT_ERROR.to_string())?,
    )
    .map_err(|_| SNAPSHOT_ERROR.to_string())?;
    let world = build_snapshot_world(verify_snapshot(&raw)?)?;
    let path = snapshot_world_path(&app)?;
    std::fs::create_dir_all(path.parent().ok_or_else(|| SNAPSHOT_ERROR.to_string())?)
        .map_err(|_| SNAPSHOT_ERROR.to_string())?;
    std::fs::write(
        &path,
        ofm_core::generator::export_world_to_json(&world)
            .map_err(|_| SNAPSHOT_ERROR.to_string())?,
    )
    .map_err(|_| SNAPSHOT_ERROR.to_string())?;
    get_albion_snapshot_status(app)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_snapshot(alex_club_id: &str, content_hash: &str) -> SnapshotDocument {
        SnapshotDocument {
            schema_version: 1,
            season: "2026/27".to_string(),
            clubs: vec![
                SnapshotClub {
                    id: "northbridge".to_string(),
                    name: "Northbridge FC".to_string(),
                    country: "ENG".to_string(),
                },
                SnapshotClub {
                    id: "riverside".to_string(),
                    name: "Riverside Town".to_string(),
                    country: "ENG".to_string(),
                },
            ],
            players: vec![
                SnapshotPlayer {
                    id: "alex".to_string(),
                    name: "Alex Porter".to_string(),
                    club_id: alex_club_id.to_string(),
                    position: "ST".to_string(),
                    rating: None,
                },
                SnapshotPlayer {
                    id: "sam".to_string(),
                    name: "Sam Reed".to_string(),
                    club_id: "riverside".to_string(),
                    position: "GK".to_string(),
                    rating: None,
                },
            ],
            content_hash: content_hash.to_string(),
        }
    }

    fn player_team_name(world: &ofm_core::generator::WorldData, player_id: &str) -> String {
        let player = world
            .players
            .iter()
            .find(|player| player.id == player_id)
            .expect("player exists");
        world
            .teams
            .iter()
            .find(|team| Some(&team.id) == player.team_id.as_ref())
            .expect("player team exists")
            .name
            .clone()
    }

    #[test]
    fn accepts_the_node_pipeline_hash_format() {
        let raw: Value = serde_json::json!({
            "schemaVersion": 1, "season": "2026/27", "clubs": [{"id": "northbridge", "name": "Northbridge"}],
            "players": [{"id": "alex", "name": "Alex Porter", "clubId": "northbridge", "position": "ST"}],
            "provenance": {"provider": "deterministic-fixture", "rawInputHash": null, "pipelineVersion": "albion-data-v1"}
        });
        let hash = snapshot_hash(&raw).expect("hash should calculate");
        let mut published = raw;
        published
            .as_object_mut()
            .expect("object")
            .insert("contentHash".to_string(), Value::String(hash));
        assert_eq!(
            verify_snapshot(&published)
                .expect("published snapshot")
                .season,
            "2026/27"
        );
    }

    #[test]
    fn verifies_the_published_node_fixture_hash() {
        let published: Value = serde_json::json!({
            "schemaVersion": 1, "season": "2026/27",
            "clubs": [
                {"id": "northbridge-fc", "name": "Northbridge FC", "country": "ENG"},
                {"id": "riverside-town", "name": "Riverside Town", "country": "ENG"}
            ],
            "players": [
                {"id": "alex-porter", "name": "Alex Porter", "clubId": "northbridge-fc", "position": "ST"},
                {"id": "sam-reed", "name": "Sam Reed", "clubId": "riverside-town", "position": "GK"}
            ],
            "provenance": {
                "provider": "manual-json",
                "rawInputHash": "sha256:47c309c411c1415debb4eb3e46f99e94327738982494280cf39d22bcb4686782",
                "pipelineVersion": "albion-data-v1"
            },
            "contentHash": "sha256:96112a117374a0af56faa08b31a5fa08b93327af193bcee536bf0fc819221b21"
        });
        assert_eq!(
            verify_snapshot(&published)
                .expect("Node-published fixture")
                .players
                .len(),
            2
        );
    }

    #[test]
    fn imported_rating_estimate_sets_a_coherent_playable_baseline() {
        let mut world = ofm_core::generator::generate_world_data(
            &ofm_core::generator::DefinitionSources::embedded_only(),
        );
        overlay_snapshot_roster(
            &mut world,
            vec![SnapshotClub {
                id: "test".to_string(),
                name: "Test Albion".to_string(),
                country: "ENG".to_string(),
            }],
            vec![SnapshotPlayer {
                id: "player".to_string(),
                name: "Rated Player".to_string(),
                club_id: "test".to_string(),
                position: "ST".to_string(),
                rating: Some(SnapshotRating {
                    overall: 67,
                    potential: 79,
                }),
            }],
        )
        .expect("generated England teams should accept a source roster");
        let player = world
            .players
            .iter()
            .find(|player| player.id == "snapshot-player")
            .expect("rated player should be imported");
        assert_eq!(player.attributes.shooting, 67);
        assert_eq!(player.potential, 79);
    }

    #[test]
    fn snapshot_b_creates_a_new_world_without_mutating_snapshot_a_career_seed() {
        let snapshot_a_world =
            build_snapshot_world(fixture_snapshot("northbridge", "sha256:snapshot-a"))
                .expect("snapshot A builds a playable world");
        let snapshot_b_world =
            build_snapshot_world(fixture_snapshot("riverside", "sha256:snapshot-b"))
                .expect("snapshot B builds a playable world");
        assert_eq!(snapshot_a_world.metadata.world_id, "sha256:snapshot-a");
        assert_eq!(snapshot_b_world.metadata.world_id, "sha256:snapshot-b");
        assert_eq!(
            player_team_name(&snapshot_a_world, "snapshot-alex"),
            "Northbridge FC"
        );
        assert_eq!(
            player_team_name(&snapshot_b_world, "snapshot-alex"),
            "Riverside Town"
        );
        assert_eq!(
            player_team_name(&snapshot_a_world, "snapshot-alex"),
            "Northbridge FC"
        );
    }
}
