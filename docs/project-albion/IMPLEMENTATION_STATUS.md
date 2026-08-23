# Project Albion — Implementation Status

Controlling spec: `Project Albion/00_MASTER_AI_BUILD_PROMPT.md` (docs 00-09 read in full,
2026-08-23).

This file is the single source of truth for phase progress. Update it after every
phase/session. Statuses: NOT STARTED | IN PROGRESS | DONE | BLOCKED.

## Baseline

- Upstream repo: https://github.com/openfootmanager/openfootmanager
- Upstream commit at audit time: `0b5ae8b223bc3cbb114aa34a47a655ed852ad8a6`
  (2026-08-19, merge of PR #488 refactor/tournaments-fixtures-view)
- Local working tree: `D:\fm26\openfootballmanager` (only untracked addition at
  audit time: `Project Albion/` spec docs)
- Node: v24.18.0 / npm 11.16.0
- Rust: cargo 1.97.1 / rustc 1.97.1 (2026-06-30 / 2026-07-14 builds)


## Existing crate map (vs. target architecture in 02_TECHNICAL_ARCHITECTURE.md)

| Target crate       | Existing equivalent            | Notes |
|---------------------|--------------------------------|-------|
| domain               | `src-tauri/crates/domain`       | present, extend |
| engine               | `src-tauri/crates/engine`       | present; currently instant-only, single-process, no seed/version persisted per spec — needs Phase 6 work |
| db                   | `src-tauri/crates/db`           | present, SQLite-based; needs per-career schema + migrations per 03 |
| ofm_core             | `src-tauri/crates/ofm_core`     | present; owns calendar/world-gen/tournaments today, closest thing to "core" |
| albion_rules         | none                            | NEW — Phase 1 |
| albion_protocol      | none                            | NEW — Phase 1 |
| albion_server        | none                            | NEW — Phase 8 |
| albion_ai            | none                            | NEW — Phase 7 (some AI logic may already live in ofm_core; needs audit) |
| tools/data_pipeline  | none                            | NEW — Phase 3 |
| Tauri client         | `src-tauri/src` + `src/`        | today Tauri commands mutate local state directly — must become thin client per Phase 9 |
| ofm-cli, sim-bench   | extra upstream crates           | keep; useful for calibration (Phase 6) and headless runs |

Upstream is single-player, single-process, Tauri-mutates-directly. No multiplayer,
no server, no protocol, no snapshot pipeline exist yet — Phases 1, 3, 6 (partial),
7, 8, 9 are all substantially new work, not adaptation.

## Baseline test run (pre-existing, before any Albion change)

Environment note: system `NODE_ENV` was set to `production`, which silenced npm
devDependencies (vitest etc. did not install). Fixed locally per-session with
`npm install --include=dev`; not a repo bug, just document it for future sessions.

### Frontend — `npm test` (vitest)
1443 passed / 6 failed / 1449 total. Pre-existing, not caused by Project Albion:
- `src/pages/MainMenu.test.tsx` — "stores the nationality as an ISO code..." (es, de)
- `src/components/tactics/TacticsTab.test.tsx` — 3 failures (toolbar, preset fallback, apply preset)
- `src/components/transfers/TransfersTab.test.tsx` — pagination test
- `src/components/menu/PackageEditor/CountryForm.test.tsx` — canonical name test

`npm run build` — passes cleanly (vite build succeeds, only a non-fatal plugin-timings notice).

### Backend — `cargo test --workspace`
The prior `ofm_core` fixture failure caused by Windows CRLF line endings was
fixed in Phase 1 by normalizing the on-disk fixture before comparison. The
focused regression test now passes. Run the full workspace suite before a PR.

### Re-verification — 2026-08-23

The current upstream `develop` commit was fetched and tested in an isolated
worktree at `D:\fm26\ofm-upstream-baseline`, detached at
`0b5ae8b223bc3cbb114aa34a47a655ed852ad8a6`. This leaves the Albion branch and
its working tree untouched.

- `npm ci` and `npm run build` passed.
- `npm test` completed with 1,444 passed and 5 pre-existing failures across
  `MainMenu.test.tsx` (one timeout), `TacticsTab.test.tsx` (three timeouts),
  and `CountryForm.test.tsx` (the nation-picker option was unavailable).
- `cargo fmt --check` failed on pre-existing formatting throughout the
  upstream tree.
- `cargo clippy --workspace --all-targets -- -D warnings` passed.
- `cargo test --workspace --quiet` passed every crate except one upstream
  `ofm_core` fixture assertion: it compares a CRLF-generated JSON string with
  an LF fixture (`the_frontend_fixture_lists_every_field_every_definition_serializes`);
  514 of 515 `ofm_core` tests passed.
- `npm run tauri dev` launched the desktop window successfully as
  `Openfoot Manager v0.3.0-dev · 0b5ae8b` (Vite on `http://localhost:1420`).

## Phase status

| Phase | Title | Status |
|---|---|---|
| 0 | Baseline audit | DONE |
| 1 | Versions, protocol, rules foundation | DONE |
| 2 | Relational per-career save | DONE |
| 3 | Snapshot/data pipeline | DONE |
| 4 | Competition/calendar | IN PROGRESS |
| 5 | Core management systems | NOT STARTED |
| 6 | Albion match engine v1 | NOT STARTED |
| 7 | AI managers | NOT STARTED |
| 8 | Authoritative server | NOT STARTED |
| 9 | Desktop client migration | NOT STARTED |
| 10 | Thai + accessibility | NOT STARTED |
| 11 | Production snapshot | BLOCKED (external data pending) |
| 12 | Balance, soak, hardening | NOT STARTED |
| 13 | Release candidate checklist | NOT STARTED |

## Next session entry point

Phase 1 now passes competition-owned substitution limits and windows into live
match setup. `CompetitionRules` owns the defaults (5 substitutes, 3 windows,
half-time exempt), and focused engine/core regressions cover both enforcement
and setup propagation. The engine's standalone constructor intentionally keeps
the same defaults for simulations and legacy tests.

FPL-based careers now load the bundled England 2026/27 rules pack at creation,
apply its top-flight substitution settings to the generated English competition,
and retain the selected ruleset id/version in the live game state. The ruleset
files are included in the desktop bundle; a focused startup regression protects
the mapping. Per-career SQLite metadata now persists that ruleset identity via
an append-only v044 migration. Save creation, regular saves, and saves with
stats also write the matching `career_versions` contract; loading a pinned
career validates it against the runtime and backfills the contract for older
pinned saves. Random and legacy careers remain usable without an invented
ruleset.

Phase 1 completion evidence: `cargo test -p albion_rules` passes 13 validation and
round-trip tests; `cargo test -p albion_protocol` passes 6 envelope and exact
compatibility tests. Ruleset-owned substitution, transfer-calendar, and squad
registration constraints are now copied into the playable career and persisted
with it. The active England/FPL path has no season-specific football rule
constant outside the selected ruleset; remaining engine and season-context
defaults are intentionally retained only for simulations and legacy careers
without a pinned ruleset.

Phase 2 completion evidence: each career has its own migrated SQLite database;
the repository suite covers create/mutate/close/reopen state, schema migrations,
and the default snapshot rotation now retains a recoverable database before an
overwrite. `test_save_game_updates_existing` asserts that snapshot behavior.

Phase 3 progress: `tools/data_pipeline` now has deterministic recursive content
hashing, manual JSON and CSV adapters, fixture coverage, FK/duplicate validation,
provenance and a machine/human diff. The desktop app verifies that hash again
before importing a snapshot, converts the immutable source into a separate
playable world database, and only uses it for subsequently-created careers.
The Settings screen exposes the import action; New Game prefers the imported
snapshot over the FPL baseline and binds the existing England rules pack. Each
new snapshot career also pins its published content hash in the career lockfile.
`albion-rating-v1` now creates a deterministic, explicitly low-confidence
fallback estimate from the stable player ID; the importer applies that estimate
to the new career's playable attribute and potential baseline.

Evidence: `npm run test:data-pipeline`; `cargo test -p openfootmanager
snapshot_data --lib`; `npm run build`. The Rust test verifies the exact content
hash emitted by the Node fixture, so the import boundary is covered rather than
only each implementation in isolation.

Phase 3 completion evidence: `npm run test:data-pipeline` covers manual JSON
and CSV adapters, deterministic fixture publication, hash verification, FK and
duplicate validation, transfer diffing, stable IDs across Snapshot A/B, and an
ambiguous-name case that fails publication until an explicit identity review is
recorded. `cargo test -p openfootmanager snapshot_data --lib` verifies the
exact Node-produced hash at the import boundary, the rating baseline, and the
Snapshot A/B import flow: B carries the transfer into a new playable world
while A remains unchanged. Phase 2's save close/reopen and backup coverage
continues to protect the resulting per-career state.

Phase 4 started: audit the existing generic league, fixture generation,
promotion/relegation and season rollover paths; add the structural multi-season
regression coverage before extending any missing competition formats. The first
regression drives a three-tier pyramid through 20 seasons, asserting that every
club belongs to exactly one tier and every regenerated fixture is valid. England's
national cup is now represented as the FA Cup while retaining the generic
knockout/calendar engine. The calendar core can now turn an explicit league-table
range into a visible knockout playoff and resolve a `PlayoffWinner` berth from
its completed final, rather than silently treating that berth as a table place.

Evidence: `cargo test -p ofm_core
promotion::tests::twenty_structural_seasons_preserve_pyramid_membership_and_schedules`
and `cargo test -p openfootmanager england_foundation_uses_the_fa_cup_identity
--lib` pass. `cargo test -p ofm_core league_playoff --lib` and `cargo test -p
ofm_core playoff_berth --lib` cover playoff entrant selection and qualification
from the settled final.
