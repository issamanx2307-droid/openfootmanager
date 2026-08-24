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
| 4 | Competition/calendar | DONE |
| 5 | Core management systems | DONE |
| 6 | Albion match engine v1 | DONE |
| 7 | AI managers | DONE |
| 8 | Authoritative server | DONE |
| 9 | Desktop client migration | IN PROGRESS |
| 10 | Thai + accessibility | IN PROGRESS |
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

Phase 4 completed: audit the existing generic league, fixture generation,
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
from the settled final. Ruleset-owned automatic promotion, relegation and
playoff slots now flow into runtime competitions; the completed playoff winner
takes the final promotion slot. England's foundation distributes available clubs
through the Premier League, Championship, League One, League Two and National
League feeder ladder when sufficient clubs exist, and maps the ruleset by tier
instead of ambiguous participant count.

Phase 4 completion evidence: `cargo test -p ofm_core --lib` passes 524 tests,
including the 20-season structural pyramid regression and configured playoff
promotion. `cargo test -p ofm_core --test end_of_season_tests` passes 75
qualification, cup, history and rollover integrations. `cargo test -p
openfootmanager --lib --quiet` passes 226 tests (1 ignored), including England
foundation, five-tier feeder, FA Cup/EFL Cup/Community Shield and tier-specific
ruleset mapping coverage.

Phase 5 completion evidence: the existing domain and persistence layers cover
squad position/role and lineup validation, registration and automatic selection;
transfer search/listing, bids/counters, installments, add-ons, loans, transfer
windows and AI competition; contract terms, bonuses, negotiation, expiry and
free agents; finance ledger/budgets/commitments/revenue/costs with financial
rule hooks; medical condition/fatigue/sharpness/injury/suspension state; and
weekly/individual training with age, potential and gradual development. Scouting
now also records completed reports as manager-specific knowledge and exposes a
durable shortlist: migration `v046` persists both collections with the manager,
and the desktop `toggle_shortlist` command updates the active career state.

Focused verification passes: `cargo test -p ofm_core --test scouting_tests`
(28 passed), `cargo test -p db scouting_knowledge` (2 passed, covering
repository and full save/load round trips), `cargo test -p openfootmanager
toggle_shortlist_internal_updates_known_player_state` (1 passed), and `cargo
test -p ofm_core --test scenario_tests full_season_holds_invariants` (1 passed).
The scenario test drives a human-controlled club through 365 daily turns and
asserts gameplay invariants throughout the full season without database edits.

Phase 6 completion evidence: the `engine` crate provides the possession/zone
event chain, attribute and tactical/role effects, set pieces, cards, goalkeeper,
fitness, substitutions, live commands and report invariants. `AlbionV1Simulator`
now exposes versioned `MatchSimulator` and `LiveMatchSimulator` contracts with
an explicit `MatchSeed`, canonical input/report fingerprints and no caller RNG.
Its instant and untouched-live paths produce the identical replay fingerprint.
Playable fixtures derive their seed from stable fixture identity for both
instant simulation and live sessions, including knockout shootouts. The
`ofm-sim-bench` calibration and benchmark CLI accepts 100,000 matches.

Focused verification passes: `cargo test -p engine --lib simulator::tests`
(4 passed), `cargo test -p engine --test simulation_tests` (50 passed),
`cargo test -p engine --test live_match_tests` (60 passed), and `cargo test -p
ofm_core match_seed::tests` (1 passed). A 100,000-match calibration benchmark
was executed with `cargo run -p sim-bench -- --games 100000 --seed 42 --bench`.

Phase 7 completion evidence: existing persistent manager profiles, AI training,
depth-chart recruitment, target scoring, transfers, loans and affordability
guardrails were audited in `ofm_core`. The daily turn now includes a final
AI-squad continuity guardrail: a non-user club that falls below a balanced
16-player senior squad signs the best suitable non-retired free agent on a
one-year contract only when the club's annual wage soft cap allows it. Every
signing records the normal free-agent movement history; the user club is never
changed by this path. AI manager vacancies are also filled in the same daily
cycle as a firing, so tactical and lineup decisions never lose their manager
between turns. `cargo test -p ofm_core --lib ai_hiring::tests` passes 14 tests,
`cargo test -p ofm_core --lib ai_squad::tests` passes 1 test, and `cargo test
-p ofm_core --test scenario_tests` passes 4 tests. The latter runs a compact,
multi-season 737-day career and asserts each AI club retains a persistent
manager, can field at least 11 healthy players, has finite finance and retains
more than one tactical identity.

Phase 8 progress: added the standalone `albion_server` binary/crate with a
private HTTP/WebSocket transport foundation. It exposes `/healthz`, `/readyz`
and `/version`, protects join/reconnect flow with a private secret and issued
reconnect token, limits a career to host and guest slots, and rejects a club
claim already held by another manager. Its single in-memory command lane
validates protocol/career/manager identity, owns the monotonic revision and
caches command results by UUID so a retried `MarkReady` cannot apply twice.
The WebSocket sends typed `Hello`, `CommandAck`, `CommandRejected` and
`ReadyStateChanged` events. This is deliberately only the transport/session
foundation: canonical game persistence, command-to-game application, ready
advancement and live-match coordination remain in progress.

Phase 8 update: canonical `Game` mutation now covers tactics, starting XI and
training commands under the same revision/idempotency lane. `ALBION_SAVE` opens
an existing per-career SQLite database, checkpoints each accepted mutation, and
rolls memory back if its checkpoint fails. The ready barrier advances AI-only
dates until the next controlled-club fixture, then emits `MatchOpened` rather
than instant-simulating it. Focused server tests cover canonical ownership,
SQLite restart round-trips and the human-fixture stop condition.

Phase 8 update: the live coordinator now owns one live session per fixture,
marks both human sides as non-AI, validates tactical commands against the
sender's match side, and advances matches through a mutex-gated server clock.
It emits event batches/state snapshots and checkpoints the finished report
before `MatchFinished`. The two-manager command-lane regression proves that
competing mutations at one revision commit once and reject the stale command.

Phase 8 update: migration `v047` persists authoritative-session metadata beside
the canonical career. On a save-backed server, manager slots, club claims,
reconnect tokens and the current revision are restored before admission resumes.
Focused restart coverage creates a real career database, joins a manager, opens
the same save again and reconnects through the original token.

Phase 8 update: WebSocket connection state now implements the default disconnect
policy: a disconnected manager loses Ready status, advancement waits for both
managers to reconnect and re-ready, and any live fixture controlled by a
disconnected human pauses rather than silently assigning AI takeover.

Phase 9 progress: the React client now has an `albionServerService` transport
boundary for server join/reconnect, typed WebSocket events and command envelopes.
It keeps only manager-specific view snapshots and the server revision in its
cache; it never treats frontend state as authoritative. Focused tests cover
snapshot/revision updates and stale-revision resync behavior. Existing Tauri
screens have not yet been switched over, so Phase 9 remains in progress.

Phase 9 update: the desktop now owns a bundled host lifecycle. Starting a host
checkpoints the active indexed save, opens that exact SQLite career in
`albion_server`, binds an ephemeral loopback port and returns its URL to the
React service layer; stopping the host aborts only that owned task. This makes
the forthcoming host/join lobby able to use a real canonical server rather
than a manually launched console process.

Phase 9 update: `/multiplayer` is now a usable Thai lobby for an active career:
the manager can host with a private code or join a supplied host URL, performs
the version handshake and club claim, then receives the narrow dashboard view
through the authoritative WebSocket. The existing dashboard exposes a direct
entry point. Broader tab-by-tab replacement of legacy local mutations remains
in progress.

Phase 8 update: a server-level human-v-human regression now builds playable
eleven-player squads for both claimed clubs, drives both managers through the
Ready barrier, verifies one canonical `MatchOpened`, then advances the server
clock and observes its `MatchState`. This closes the prior gap where live-match
coordination was implemented but not exercised through the authoritative
session layer.

Phase 8 update: server events now pass through a shared broadcast lane rather
than returning only to the socket that issued a command. The real two-WebSocket
test verifies that the guest receives both the host's acknowledgement and the
shared Ready-state update, which is required for the host/guest UI to stay in
sync during shared play.

Phase 8 update: connection tracking is reference counted per manager, so an
extra browser tab or transient duplicate socket cannot mark a still-connected
manager as absent and pause the shared career. The disconnect regression now
covers both final-socket cleanup and the multi-tab case.

Phase 9 update: reconnect UX now persists only the server URL and issued
reconnect session token locally. The lobby offers one-click restoration through
the server's reconnect endpoint and clears stale data on failure; a focused
browser-storage regression covers save, restore and cleanup.

Phase 9 update: the first shared-game UI command now crosses the full client
boundary: “Ready” serializes `MarkReady` over the authenticated WebSocket and
the lobby reacts to broadcast Ready and MatchOpened events. This connects the
desktop flow to the authoritative ready barrier instead of advancing local
Tauri state.

Phase 10 progress: Thai (`th-TH`) is now a first-class selectable language,
including translated main-menu, dashboard navigation and settings essentials;
all untranslated keys fall back safely to English while the remaining locale
coverage is completed. The i18n lazy-load test verifies Thai locale resolution,
bundle loading and a translated menu label.

Phase 10 update: the app shell now provides a keyboard-visible skip link to
the routed content target, while Thai-capable system font fallbacks prevent
Latin-oriented heading fonts from degrading Thai glyphs. Reduced-motion and
high-contrast handling already present in the shared stylesheet remain in
effect for the new lobby and routes.

Phase 10 update: the multiplayer core flow now has an English/Thai copy layer
for all visible lobby actions, session outcomes and ready-barrier feedback.
This prevents the initial Thai-only lobby from blocking the English acceptance
flow while the rest of the large legacy locale is translated incrementally.

Phase 9 update: multiplayer users can now set formation and approach from the
lobby, which sends `SetTactics` through the authoritative WebSocket command
lane. This is the first core club-management mutation exposed in the desktop
server flow after Ready, alongside the existing server ownership/revision
checks.

Phase 9 update: the same server-owned flow now exposes weekly training
intensity and team focus through `SetTrainingPlan`. Both tactics and training
mutations therefore share the authenticated/revisioned WebSocket route rather
than the legacy desktop-local command path.

Phase 9 update: `MatchOpened` now activates a small server-backed live-match
controller in the lobby. It consumes the canonical match ID and sends the
typed `ApplyLiveMatchCommand`/`ChangeFormation` command, completing the first
desktop-to-live-match tactical path without local simulation.

Phase 9 update: the live controller now also renders canonical score, elapsed
minute and phase from `MatchState`, announces them politely for assistive
technology, and exits the live-control state on `MatchFinished`. The desktop
therefore displays server output rather than deriving its own match clock.

Phase 9 update: desktop-hosted careers now bind the authoritative server on
all interfaces, enabling the documented private LAN/Tailscale use case rather
than only a second client on the same machine. The lobby keeps a loopback URL
for the host and explains how to substitute the host's LAN/Tailscale address
when inviting a guest.

Phase 8/9 documentation: `PRIVATE_LAN_PLAY.md` now gives the executable
host/join/reconnect flow for the bundled lobby, including the LAN/Tailscale URL
substitution, firewall expectation, version mismatch behavior and live-match
disconnect policy.

Phase 8 update: `RespondTransferOffer` is now implemented in the canonical
command dispatcher (accept, reject and counter), locating the offer only on
the acting manager's club before calling the existing transfer rules. Focused
coverage creates a real incoming offer and verifies a canonical rejection is
persisted in the player offer state.

Phase 8 update: a full human-v-human live coordinator regression now proves
the disconnect policy: after both managers open a canonical live fixture, a
disconnected manager prevents ticks/events; reconnecting that manager allows
the same match to advance again. This covers the live pause behavior rather
than only the Ready-barrier side of disconnect handling.

Phase 8 update: the authoritative session checkpoint now includes every
in-progress live match (field state and event sequence). A SQLite restart
regression opens a human-v-human fixture, advances one server tick, recreates
the session from the same save, and verifies the match identifier, minute and
score are restored instead of opening a new fixture.

Phase 8 update: a one-human-versus-AI live regression now verifies that the
single human manager can pass Ready, open the canonical fixture and receive a
live `MatchState` without an AI-side connection or Ready acknowledgement.

Phase 8 update: the real two-WebSocket canonical-career integration now joins
both managers, drives both Ready commands and proves each client receives the
same `MatchOpened` identifier and first streamed `MatchState` clock/score.

Phase 8 completion evidence: `cargo test -p albion_server --quiet` passes 23
tests. Together they cover HTTP/WebSocket admission and distinct-club claims,
revision conflict/idempotent retry, Ready barriers, human-v-human and
human-v-AI live coordination, disconnect pause, reconnect, SQLite restart and
shared canonical live state observed by two real WebSocket clients.

Phase 9 update: the live match centre now consumes `MatchEventBatch` in
addition to score/time state, retaining a short canonical event feed with
polite assistive-technology announcements. This makes the streamed server
event chain visible to the player rather than leaving it transport-only.

Phase 9 update: after WebSocket reconnect, the server now replays
`MatchOpened` and the current `MatchState` for each live fixture controlled by
that manager. This lets the lobby re-enter the canonical live-match view after
a server restart without retaining a client-side match clock or match ID.

Phase 9 update: the lobby now renders its narrow canonical dashboard snapshot
as a club card (club name, in-game date, formation and approach), replacing
the development-only raw JSON display. The view remains server-sourced.

Phase 9 update: the manager-only dashboard snapshot now includes the owning
club's training focus/intensity. The lobby applies server `StateDelta` updates
only when their team id matches its canonical club card, so accepted tactics
and training commands visibly refresh without treating local form state as
authoritative.

Phase 10 update: live event batches now receive a fact-only English/Thai
formatter (minute, engine event type, side), replacing raw JSON in the match
centre. Focused service tests cover both languages, including mixed Thai/Latin
output such as the canonical side identifier.

Phase 10 update: the dashboard entry point for private multiplayer now uses
the shared locale catalogue, so its label follows the active English or Thai
language instead of remaining hard-coded in Thai.

Phase 10 update: the Thai catalogue now covers the core new-career wizard:
manager creation, validation errors, world selection, generated-history
choices and generation progress. The lazy-load regression asserts translated
keys from each wizard step rather than accepting English fallback text.

Phase 10 update: the private-play lobby now localizes visible tactic and
training option labels plus the corresponding canonical dashboard values,
while retaining stable English protocol values in every command payload.
