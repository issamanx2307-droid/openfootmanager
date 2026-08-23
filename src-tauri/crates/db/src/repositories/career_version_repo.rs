use rusqlite::{Connection, params};

const CAREER_VERSION_LOAD_ERROR: &str = "be.error.careerVersions.loadFailed";
const CAREER_VERSION_WRITE_ERROR: &str = "be.error.careerVersions.writeFailed";

/// Immutable compatibility contract pinned when a career is created.
///
/// Values are intentionally stored as individual columns rather than a JSON
/// blob so migrations and compatibility checks can inspect them directly.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CareerVersions {
    pub app_version: String,
    pub protocol_version: u32,
    pub save_schema_version: u32,
    pub snapshot_schema_version: u32,
    pub match_engine_version: String,
    pub ruleset_id: String,
    pub ruleset_version: u32,
    pub rating_model_version: String,
}

/// Persist the singleton career compatibility contract.
pub fn upsert_career_versions(conn: &Connection, versions: &CareerVersions) -> Result<(), String> {
    conn.execute(
        "INSERT INTO career_versions (
            id, app_version, protocol_version, save_schema_version,
            snapshot_schema_version, match_engine_version, ruleset_id,
            ruleset_version, rating_model_version
         ) VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
         ON CONFLICT(id) DO UPDATE SET
            app_version = excluded.app_version,
            protocol_version = excluded.protocol_version,
            save_schema_version = excluded.save_schema_version,
            snapshot_schema_version = excluded.snapshot_schema_version,
            match_engine_version = excluded.match_engine_version,
            ruleset_id = excluded.ruleset_id,
            ruleset_version = excluded.ruleset_version,
            rating_model_version = excluded.rating_model_version",
        params![
            versions.app_version,
            versions.protocol_version,
            versions.save_schema_version,
            versions.snapshot_schema_version,
            versions.match_engine_version,
            versions.ruleset_id,
            versions.ruleset_version,
            versions.rating_model_version,
        ],
    )
    .map_err(|_| CAREER_VERSION_WRITE_ERROR.to_string())?;
    Ok(())
}

/// Load the compatibility contract. `None` means the career predates the
/// Albion metadata migration and must be handled non-destructively by the
/// caller rather than silently assumed compatible.
pub fn load_career_versions(conn: &Connection) -> Result<Option<CareerVersions>, String> {
    let mut statement = conn
        .prepare(
            "SELECT app_version, protocol_version, save_schema_version,
                    snapshot_schema_version, match_engine_version, ruleset_id,
                    ruleset_version, rating_model_version
             FROM career_versions WHERE id = 1",
        )
        .map_err(|_| CAREER_VERSION_LOAD_ERROR.to_string())?;
    let mut rows = statement
        .query_map([], |row| {
            Ok(CareerVersions {
                app_version: row.get(0)?,
                protocol_version: row.get(1)?,
                save_schema_version: row.get(2)?,
                snapshot_schema_version: row.get(3)?,
                match_engine_version: row.get(4)?,
                ruleset_id: row.get(5)?,
                ruleset_version: row.get(6)?,
                rating_model_version: row.get(7)?,
            })
        })
        .map_err(|_| CAREER_VERSION_LOAD_ERROR.to_string())?;

    match rows.next() {
        Some(Ok(versions)) => Ok(Some(versions)),
        Some(Err(_)) => Err(CAREER_VERSION_LOAD_ERROR.to_string()),
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::game_database::GameDatabase;

    fn versions() -> CareerVersions {
        CareerVersions {
            app_version: "0.1.0".into(),
            protocol_version: 1,
            save_schema_version: 43,
            snapshot_schema_version: 1,
            match_engine_version: "albion-engine-v1".into(),
            ruleset_id: "england-2026-27-v1".into(),
            ruleset_version: 1,
            rating_model_version: "albion-rating-v1".into(),
        }
    }

    #[test]
    fn career_versions_round_trip_and_upsert() {
        let database = GameDatabase::open_in_memory().unwrap();
        assert_eq!(load_career_versions(database.conn()).unwrap(), None);

        let initial = versions();
        upsert_career_versions(database.conn(), &initial).unwrap();
        assert_eq!(
            load_career_versions(database.conn()).unwrap(),
            Some(initial)
        );

        let mut updated = versions();
        updated.ruleset_version = 2;
        upsert_career_versions(database.conn(), &updated).unwrap();
        assert_eq!(
            load_career_versions(database.conn()).unwrap(),
            Some(updated)
        );
    }
}
