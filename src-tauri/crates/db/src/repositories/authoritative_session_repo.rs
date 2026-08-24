//! Persistence for the server-owned, non-gameplay session state.

use rusqlite::{Connection, params};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoritativeSessionRecord {
    pub career_revision: u64,
    pub claims_json: String,
    pub reconnect_tokens_json: String,
    pub updated_at: String,
}

pub fn load(conn: &Connection) -> Result<Option<AuthoritativeSessionRecord>, String> {
    conn.query_row(
        "SELECT career_revision, claims_json, reconnect_tokens_json, updated_at FROM authoritative_session WHERE id = 'singleton'",
        [],
        |row| Ok(AuthoritativeSessionRecord {
            career_revision: row.get::<_, i64>(0)?.max(0) as u64,
            claims_json: row.get(1)?,
            reconnect_tokens_json: row.get(2)?,
            updated_at: row.get(3)?,
        }),
    )
    .map(Some)
    .or_else(|error| match error {
        rusqlite::Error::QueryReturnedNoRows => Ok(None),
        _ => Err(error),
    })
    .map_err(|_| "be.error.gamePersistence.loadFailed".to_string())
}

pub fn upsert(conn: &Connection, record: &AuthoritativeSessionRecord) -> Result<(), String> {
    conn.execute(
        "INSERT INTO authoritative_session (id, career_revision, claims_json, reconnect_tokens_json, updated_at)
         VALUES ('singleton', ?1, ?2, ?3, ?4)
         ON CONFLICT(id) DO UPDATE SET career_revision = excluded.career_revision, claims_json = excluded.claims_json,
         reconnect_tokens_json = excluded.reconnect_tokens_json, updated_at = excluded.updated_at",
        params![record.career_revision as i64, record.claims_json, record.reconnect_tokens_json, record.updated_at],
    )
    .map(|_| ())
    .map_err(|_| "be.error.gamePersistence.writeFailed".to_string())
}
