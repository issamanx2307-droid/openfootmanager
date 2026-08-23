//! SQLite-backed checkpoints for the canonical server career.

use std::path::{Path, PathBuf};

use db::game_database::GameDatabase;
use db::game_persistence::{GamePersistenceReader, GamePersistenceWriter};
use db::repositories::meta_repo;
use ofm_core::game::Game;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SqliteCareerStore {
    path: PathBuf,
    save_id: String,
    save_name: String,
}

impl SqliteCareerStore {
    pub fn open(path: impl AsRef<Path>) -> Result<(Self, Game, Uuid), String> {
        let path = path.as_ref().to_path_buf();
        let db = GameDatabase::open_save(&path).map_err(|_| "be.error.saveCorrupt".to_string())?;
        let game = GamePersistenceReader::read_game(&db)?;
        let meta = meta_repo::load_meta(db.conn())?
            .ok_or_else(|| "be.error.saveCorrupt".to_string())?;
        let career_id = Uuid::parse_str(&meta.save_id)
            .unwrap_or_else(|_| Uuid::new_v5(&Uuid::NAMESPACE_OID, meta.save_id.as_bytes()));
        Ok((
            Self { path, save_id: meta.save_id, save_name: meta.save_name },
            game,
            career_id,
        ))
    }

    pub fn checkpoint(&self, game: &Game) -> Result<(), String> {
        let db = GameDatabase::open(&self.path)?;
        GamePersistenceWriter::write_game(&db, game, &self.save_id, &self.save_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use domain::manager::Manager;
    use domain::team::Team;
    use ofm_core::clock::GameClock;
    use tempfile::tempdir;

    fn game() -> Game {
        let mut manager = Manager::new(
            Uuid::new_v4().to_string(), "Alex".into(), "Manager".into(), "1980-01-01".into(), "ENG".into(),
        );
        let team_id = Uuid::new_v4().to_string();
        manager.hire(team_id.clone());
        let mut team = Team::new(team_id, "Albion".into(), "ALB".into(), "England".into(), "Albion".into(), "Ground".into(), 20_000);
        team.manager_id = Some(manager.id.clone());
        Game::new(
            GameClock::new(Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap()),
            manager,
            vec![team],
            vec![],
            vec![],
            vec![],
        )
    }

    #[test]
    fn checkpoint_round_trips_the_canonical_game() {
        let directory = tempdir().unwrap();
        let path = directory.path().join("career.db");
        let initial = game();
        let db = GameDatabase::open(&path).unwrap();
        GamePersistenceWriter::write_game(&db, &initial, "save-one", "Albion Career").unwrap();
        drop(db);

        let (store, mut loaded, career_id) = SqliteCareerStore::open(&path).unwrap();
        assert_eq!(career_id, Uuid::new_v5(&Uuid::NAMESPACE_OID, b"save-one"));
        loaded.teams[0].formation = "4-3-3".into();
        store.checkpoint(&loaded).unwrap();

        let (_, restored, _) = SqliteCareerStore::open(&path).unwrap();
        assert_eq!(restored.teams[0].formation, "4-3-3");
    }
}
