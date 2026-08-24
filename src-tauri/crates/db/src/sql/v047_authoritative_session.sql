CREATE TABLE IF NOT EXISTS authoritative_session (
    id TEXT PRIMARY KEY CHECK (id = 'singleton'),
    career_revision INTEGER NOT NULL DEFAULT 0,
    claims_json TEXT NOT NULL DEFAULT '{}',
    reconnect_tokens_json TEXT NOT NULL DEFAULT '{}',
    updated_at TEXT NOT NULL DEFAULT ''
);
