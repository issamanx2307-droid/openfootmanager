-- Project Albion: versions pinned when a career is created/opened.
-- A singleton row deliberately keeps the career's compatibility contract
-- separate from mutable game metadata and from the migration ledger.
CREATE TABLE career_versions (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    app_version TEXT NOT NULL,
    protocol_version INTEGER NOT NULL,
    save_schema_version INTEGER NOT NULL,
    snapshot_schema_version INTEGER NOT NULL,
    match_engine_version TEXT NOT NULL,
    ruleset_id TEXT NOT NULL,
    ruleset_version INTEGER NOT NULL,
    rating_model_version TEXT NOT NULL
);
