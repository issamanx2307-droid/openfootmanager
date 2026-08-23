use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use tauri::{AppHandle, Manager};

const SOURCE_REPOSITORY: &str = "https://github.com/olbauday/FPL-Core-Insights";
const SOURCE_BASE_URL: &str =
    "https://raw.githubusercontent.com/olbauday/FPL-Core-Insights/main/data/2026-2027";
const DATA_SOURCE_ERROR: &str = "be.error.dataSource.updateFailed";
const FPL_WORLD_FILENAME: &str = "fpl-core-insights-2026-2027.json";
const FPL_WORLD_ID: &str = "fpl-core-insights-2026-2027";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FplDataSourceStatus {
    pub repository: String,
    pub cached: bool,
    pub updated_at: Option<String>,
    pub files: Vec<String>,
    pub world_database_path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FplTeamRow {
    code: u32,
    name: String,
    short_name: String,
}

#[derive(Debug, Deserialize)]
struct FplPlayerRow {
    player_id: u32,
    first_name: String,
    second_name: String,
    web_name: String,
    team_code: u32,
    position: String,
}

fn source_dir(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("data-sources").join("fpl-core-insights"))
        .map_err(|_| DATA_SOURCE_ERROR.to_string())
}

fn world_database_path(app: &AppHandle) -> Result<std::path::PathBuf, String> {
    app.path()
        .app_data_dir()
        .map(|path| path.join("databases").join(FPL_WORLD_FILENAME))
        .map_err(|_| DATA_SOURCE_ERROR.to_string())
}

#[tauri::command]
pub fn get_fpl_data_source_status(app: AppHandle) -> Result<FplDataSourceStatus, String> {
    let dir = source_dir(&app)?;
    let files = ["players.csv", "teams.csv"]
        .into_iter()
        .filter(|name| dir.join(name).is_file())
        .map(str::to_string)
        .collect::<Vec<_>>();
    let updated_at = std::fs::read_to_string(dir.join("updated_at.txt")).ok();
    let world_database_path = world_database_path(&app)?;

    Ok(FplDataSourceStatus {
        repository: SOURCE_REPOSITORY.to_string(),
        cached: files.len() == 2,
        updated_at,
        files,
        world_database_path: world_database_path
            .is_file()
            .then(|| world_database_path.to_string_lossy().to_string()),
    })
}

/// Refresh the Premier League baseline from the publisher's repository. The
/// cache is separate from career saves, so existing careers remain pinned to
/// their creation snapshot until the import pipeline deliberately uses it.
#[tauri::command]
pub async fn update_fpl_data_source(app: AppHandle) -> Result<FplDataSourceStatus, String> {
    tauri::async_runtime::spawn_blocking(move || update_fpl_data_source_sync(&app))
        .await
        .map_err(|_| DATA_SOURCE_ERROR.to_string())?
}

fn update_fpl_data_source_sync(app: &AppHandle) -> Result<FplDataSourceStatus, String> {
    let dir = source_dir(app)?;
    std::fs::create_dir_all(&dir).map_err(|_| DATA_SOURCE_ERROR.to_string())?;

    for file_name in ["players.csv", "teams.csv"] {
        let response = reqwest::blocking::get(format!("{SOURCE_BASE_URL}/{file_name}"))
            .map_err(|_| DATA_SOURCE_ERROR.to_string())?
            .error_for_status()
            .map_err(|_| DATA_SOURCE_ERROR.to_string())?;
        let bytes = response.bytes().map_err(|_| DATA_SOURCE_ERROR.to_string())?;
        if bytes.is_empty() || bytes.len() > 50 * 1024 * 1024 {
            return Err(DATA_SOURCE_ERROR.to_string());
        }
        std::fs::write(dir.join(file_name), bytes).map_err(|_| DATA_SOURCE_ERROR.to_string())?;
    }

    let updated_at = chrono::Utc::now().to_rfc3339();
    std::fs::write(dir.join("updated_at.txt"), &updated_at)
        .map_err(|_| DATA_SOURCE_ERROR.to_string())?;
    build_fpl_world_database(app, &dir, &updated_at)?;
    get_fpl_data_source_status(app.clone())
}

/// Builds a playable world from the cached FPL roster.  The source provides
/// authoritative Premier League team and player identities, while the game's
/// normal generator deliberately supplies the rest of the football world and
/// the simulation attributes that the source does not publish.
fn build_fpl_world_database(
    app: &AppHandle,
    cache_dir: &std::path::Path,
    updated_at: &str,
) -> Result<(), String> {
    let teams = read_csv::<FplTeamRow>(&cache_dir.join("teams.csv"))?;
    let players = read_csv::<FplPlayerRow>(&cache_dir.join("players.csv"))?;
    if teams.len() != 20 || players.is_empty() {
        return Err(DATA_SOURCE_ERROR.to_string());
    }

    let mut world = ofm_core::generator::generate_world_data(
        &ofm_core::generator::DefinitionSources::embedded_only(),
    );
    overlay_fpl_roster(&mut world, teams, players)?;
    world.name = "FPL Core Insights".to_string();
    world.description = "Premier League roster baseline from FPL Core Insights".to_string();
    world.metadata = ofm_core::generator::WorldDataMetadata {
        format_version: 2,
        world_id: FPL_WORLD_ID.to_string(),
        kind: ofm_core::generator::WorldDataKind::RosterBaseline,
        base_year: Some(2026),
        snapshot_date: Some(updated_at.to_string()),
    };

    let path = world_database_path(app)?;
    let parent = path.parent().ok_or_else(|| DATA_SOURCE_ERROR.to_string())?;
    std::fs::create_dir_all(parent).map_err(|_| DATA_SOURCE_ERROR.to_string())?;
    let json = ofm_core::generator::export_world_to_json(&world)
        .map_err(|_| DATA_SOURCE_ERROR.to_string())?;
    std::fs::write(path, json).map_err(|_| DATA_SOURCE_ERROR.to_string())
}

fn read_csv<T: for<'de> Deserialize<'de>>(path: &std::path::Path) -> Result<Vec<T>, String> {
    let mut reader = csv::Reader::from_path(path).map_err(|_| DATA_SOURCE_ERROR.to_string())?;
    reader
        .deserialize()
        .collect::<Result<Vec<T>, _>>()
        .map_err(|_| DATA_SOURCE_ERROR.to_string())
}

fn fpl_position(position: &str) -> domain::player::Position {
    match position {
        "Goalkeeper" => domain::player::Position::Goalkeeper,
        "Defender" => domain::player::Position::Defender,
        "Midfielder" => domain::player::Position::Midfielder,
        "Forward" => domain::player::Position::Forward,
        _ => domain::player::Position::Midfielder,
    }
}

fn overlay_fpl_roster(
    world: &mut ofm_core::generator::WorldData,
    mut fpl_teams: Vec<FplTeamRow>,
    fpl_players: Vec<FplPlayerRow>,
) -> Result<(), String> {
    fpl_teams.sort_by_key(|team| team.code);
    let mut english_teams = world
        .teams
        .iter()
        .enumerate()
        .filter(|(_, team)| team.country == "ENG")
        .map(|(index, team)| (index, team.id.clone(), team.reputation))
        .collect::<Vec<_>>();
    english_teams.sort_by(|(_, left_id, left_reputation), (_, right_id, right_reputation)| {
        right_reputation.cmp(left_reputation).then_with(|| left_id.cmp(right_id))
    });
    if english_teams.len() < fpl_teams.len() {
        return Err(DATA_SOURCE_ERROR.to_string());
    }

    let fpl_team_ids = english_teams
        .into_iter()
        .take(fpl_teams.len())
        .zip(fpl_teams)
        .map(|((index, generated_id, _), source)| {
            let team = &mut world.teams[index];
            team.name = source.name;
            team.short_name = source.short_name;
            (source.code, generated_id)
        })
        .collect::<HashMap<_, _>>();

    let target_team_ids = fpl_team_ids.values().cloned().collect::<HashSet<_>>();
    let generated_players = world.players.clone();
    world.players.retain(|player| {
        !player
            .team_id
            .as_ref()
            .is_some_and(|team_id| target_team_ids.contains(team_id))
    });

    for source in fpl_players {
        let Some(team_id) = fpl_team_ids.get(&source.team_code) else {
            continue;
        };
        let position = fpl_position(&source.position);
        let template = generated_players
            .iter()
            .find(|player| {
                player.team_id.as_deref() == Some(team_id.as_str())
                    && player.natural_position.to_group_position() == position.to_group_position()
            })
            .or_else(|| {
                generated_players
                    .iter()
                    .find(|player| player.team_id.as_deref() == Some(team_id.as_str()))
            })
            .ok_or_else(|| DATA_SOURCE_ERROR.to_string())?;
        let mut player = template.clone();
        player.id = format!("fpl-{}", source.player_id);
        player.match_name = if source.web_name.trim().is_empty() {
            source.second_name.clone()
        } else {
            source.web_name
        };
        player.full_name = format!("{} {}", source.first_name.trim(), source.second_name.trim())
            .trim()
            .to_string();
        player.position = position.clone();
        player.natural_position = position;
        player.team_id = Some(team_id.clone());
        player.career.clear();
        player.movement_history.clear();
        player.transfer_offers.clear();
        player.loan_offers.clear();
        player.active_loan = None;
        ofm_core::player_rating::refresh_player_derived(&mut player, 2026);
        world.players.push(player);
    }

    if world.players.iter().filter(|player| {
        player.team_id
            .as_ref()
            .is_some_and(|team_id| target_team_ids.contains(team_id))
    }).count() == 0 {
        return Err(DATA_SOURCE_ERROR.to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_fpl_positions_to_playable_position_groups() {
        assert_eq!(fpl_position("Goalkeeper"), domain::player::Position::Goalkeeper);
        assert_eq!(fpl_position("Defender"), domain::player::Position::Defender);
        assert_eq!(fpl_position("Midfielder"), domain::player::Position::Midfielder);
        assert_eq!(fpl_position("Forward"), domain::player::Position::Forward);
        assert_eq!(fpl_position("unexpected"), domain::player::Position::Midfielder);
    }

    #[test]
    fn overlays_source_identity_onto_a_playable_generated_world() {
        let mut world = ofm_core::generator::generate_world_data(
            &ofm_core::generator::DefinitionSources::embedded_only(),
        );
        overlay_fpl_roster(
            &mut world,
            vec![FplTeamRow {
                code: 1,
                name: "Test Albion".to_string(),
                short_name: "TAL".to_string(),
            }],
            vec![FplPlayerRow {
                player_id: 99,
                first_name: "Test".to_string(),
                second_name: "Player".to_string(),
                web_name: "T. Player".to_string(),
                team_code: 1,
                position: "Forward".to_string(),
            }],
        )
        .expect("generated England teams should accept a source roster");

        assert!(world.teams.iter().any(|team| team.name == "Test Albion"));
        let player = world
            .players
            .iter()
            .find(|player| player.id == "fpl-99")
            .expect("source player should be imported");
        assert_eq!(player.full_name, "Test Player");
        assert_eq!(player.natural_position, domain::player::Position::Forward);
        assert!(player.team_id.is_some());
    }
}
