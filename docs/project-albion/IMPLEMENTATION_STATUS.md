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
| 12 | Balance, soak, hardening | IN PROGRESS |
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

Phase 9 update: the same canonical dashboard snapshot now exposes the owning
club's next scheduled fixture (date, competition and both club names). The
lobby renders it as a schedule card, with a regression proving the view is
derived from the server schedule rather than local frontend state.

Phase 9 update: the lobby now renders a compact server-owned squad view (name,
position, condition and injury marker). The server regression builds two
eleven-player clubs and proves manager A receives exactly its own 11-player
roster, not the opponent's roster or hidden attributes.

Phase 9 update: a compact canonical inbox now renders the five latest relevant
message summaries in the lobby. The manager-specific server filter exposes
only messages for the controlled club (plus global messages), with a
regression proving another club's private message is omitted.

Phase 9 update: when a manager joins or reconnects, the lobby now initializes
its tactic formation, approach and training form controls from the canonical
dashboard snapshot rather than client defaults. Subsequent commands still use
the stable protocol values derived from those server values.

Phase 9 update: pending incoming transfer offers for the manager's own players
now appear in the canonical lobby view with accept/reject actions. Those actions
send `RespondTransferOffer` over the authenticated WebSocket, and the offer is
removed only after the server's resulting state delta. The canonical regression
covers offer projection and the existing mutation regression covers rejection.

Phase 9 update: the canonical lobby club card now includes the manager club's
server-sourced financial balance, rendered as its raw in-game integer rather
than inventing a currency or using frontend-local financial state.

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

Phase 9 update: commands that identify football domain records now use stable
string IDs end-to-end, rather than requiring UUID-shaped IDs. This preserves
the bundled FPL importer's `fpl-<source id>` player identities for line-ups,
transfers, contracts and live substitutions; protocol round-trips and the
canonical incoming-offer regression exercise non-UUID FPL-style IDs.

Phase 9 update: the canonical lobby now supports selecting exactly eleven fit
players from the server-provided squad for the upcoming fixture. It sends only
the selected stable IDs, fixture ID and formation through `SetStartingXi`; the
server validates ownership, fitness and uniqueness, then returns the resulting
selection in its state delta.

Phase 9 update: `albionServerService` now serializes every ergonomic React
command into the protocol's required internally-tagged `{ type, body }` wire
form before WebSocket transmission. A focused regression covers the FPL-style
starting-XI payload, so lobby actions no longer depend on an invalid enum shape.
The matching protocol regression deserializes that exact wire shape.

Phase 10 update: canonical `CommandRejected` events now display their stable
error code as actionable English or Thai copy in the lobby, including stale
state, invalid line-ups, budget limits and unavailable match commands. No
server prose is trusted or shown; the focused service test covers both locales.

Phase 10 update: the private-play lobby now announces connection and command
status through an atomic polite live region. Training intensity has a textual
range value, while every starting-XI checkbox describes the player's condition
or injury state in addition to its visible non-colour indicator.

Phase 10 update: the existing Settings flow for updating the bundled FPL Core
Insights source and importing an Albion snapshot now uses English/Thai locale
keys instead of hard-coded English. It remains a desktop action that invokes
the existing data-source commands; a production Vite build passes with the
localized Settings page.

Phase 9 update: the canonical dashboard now exposes a narrow transfer market
of transfer-listed players only (identity, position and known market value),
excluding hidden attributes. The lobby can submit a whole-currency bid through
`SubmitTransferBid`; it converts the displayed amount to protocol minor units
and leaves budget/window/negotiation validation authoritative on the server.

## Phase 9 release-slice decision (2026-08-24)

Phase 9 is accepted for the **authoritative private multiplayer core loop**,
not as a claim that every legacy single-player Tauri screen has been migrated.
The accepted loop is: host or join a career, claim a club, reconnect, pass the
Ready barrier, inspect the server-owned club/dashboard/inbox/squad/fixture and
finances, set tactics/training/starting XI, submit or answer transfer activity,
and play the server-owned live match. All mutations in that loop cross the
authenticated, revisioned server command boundary.

The legacy dashboard's scouting, medical, staff, profiles, competitions,
full-schedule, match-preview and post-match modules remain local single-player
surfaces. They are explicitly deferred follow-up migration work, rather than
silently being treated as server-authoritative multiplayer features. This scope
decision permits Phase 10 validation to concentrate on the accepted core loop
in English and Thai.

Phase 10 update: Settings now has complete Thai copy, including language,
accessibility, match preferences and the FPL data update controls. When the
frontend runs outside Tauri (for local browser demos and visual checks), it
persists settings in browser storage; the desktop runtime continues to use its
existing Tauri settings command. The fallback is covered by the settings-store
regression, and a 1280×720 browser check confirms the Thai Settings screen has
no horizontal overflow.

Phase 10 update: the app shell now sets the document `lang` attribute from the
chosen locale. Screen readers therefore receive `th` for the Thai main menu
and Settings flow instead of the former English document language.

Deployment preparation update: the standalone server now applies configurable
CORS origins through `ALBION_CORS_ORIGINS` while retaining a permissive private
LAN default. Oracle deployment instructions cover `ALBION_BIND`,
`ALBION_JOIN_SECRET`, `ALBION_SAVE`, systemd, health/readiness probes, HTTPS
WebSocket proxying and SQLite backup/restart smoke checks. This is preparation
for the first single-career Oracle VM, not account authentication or
multi-career tenancy.

Phase 10 update: Thai now covers the full club-selection and simulation-scope
flow, including region, competition and dependency explanations. The locale
audit consequently drops from 78 to 63 missing top-level key groups; remaining
groups belong to legacy management screens outside the accepted multiplayer
core loop.

Oracle deployment update: the repository now contains a deployable bundle with
an environment template, systemd unit, Caddyfile, native-on-Oracle build
script, consistent SQLite online-backup script and public probe script. The
only deployment-specific remaining work is to supply the Oracle VM, DNS name,
career database and private join secret, then execute the documented smoke
test.

2D Enhanced Match Renderer update: the local Match Centre now uses a
presentation-only Canvas renderer over the existing authoritative snapshot,
with deterministic formation placement, half-time mirroring, canonical event
timeline playback, replay, camera/zoom and a persisted reduced-motion setting.
The multiplayer protocol now includes a read-only engine snapshot in every
`MatchState`, so a host, guest or reconnecting client can rebuild the same
score, line-ups, cards, substitutions and event history without a client
simulation. The client validates this payload, deduplicates ordered event
batches, and falls back to the commentary/stat view if Canvas is unavailable.

Verification for this slice: `cargo test -p albion_protocol -p albion_server
--quiet` (8 + 27 tests), `cargo test -p openfootmanager
commands::settings::tests --quiet` (4 tests), and focused Vitest renderer,
adapter, settings and MatchSimulation suites. The end-to-end two-human
acceptance scenario in `10_2D_ENHANCED_MATCH_RENDERER.md` still remains in
progress; in particular, visual QA across two browser/desktop clients and the
developer diagnostics overlay are not yet accepted as complete.

2D Enhanced Match Renderer update: player markers now expose authoritative
yellow-card and injury state with a card badge and a non-colour medical cross.
The renderer continues to remove sent-off players from the resolved formation;
these visual states are derived from the snapshot/event stream and cannot
modify match state.

2D Enhanced Match Renderer update: role abbreviations can now be toggled in
the Match Centre and persist as a local presentation preference alongside
camera, highlight and player-name choices.  The renderer derives concise role
labels from the existing snapshot player role; it does not create a tactical
state or send a command to the match engine.

2D/multiplayer acceptance update: the authoritative server now pauses a
human-v-human live match at half-time and extra-time half-time until every
connected manager controlling that fixture sends the existing idempotent
`MarkReady` command.  Readiness is consumed when play resumes and is cleared
on disconnect.  The lobby exposes the Ready action only before a match or at
an authoritative interval, avoiding an invalid command while play is live.
The server regression covers one-manager and two-manager readiness at
half-time; the server crate suite passes 28 tests.

2D/multiplayer acceptance update: a live formation command now remains
`pending` in the lobby until the matching server command acknowledgement,
then becomes `accepted` and only `applied` when the authoritative `MatchState`
arrives.  A matching rejection is rendered as rejected.  No client-side
formation or renderer state is changed optimistically.

2D/multiplayer acceptance update: the multiplayer Match Centre now offers a
local-only replay of already-published goals, penalties, key shots/saves and
red-card events.  The replay reuses the authoritative semantic event in the
existing Canvas presentation layer and can be closed to return safely to the
current live state; it never invokes or pauses match simulation.

2D/multiplayer acceptance update: host and guest can now choose 1×, 2× or 4×
local presentation speed in the multiplayer Match Centre.  This changes only
Canvas timeline playback on that client; it is neither sent to nor applied by
the authoritative server clock.

2D/multiplayer acceptance update: the multiplayer Match Centre now displays
the home/away possession split directly from the authoritative MatchState
snapshot, with numeric percentages and a labelled bar.  It does not derive
possession from client event timing or Canvas state.

2D/multiplayer acceptance update: the client now retains the authoritative
`MatchFinished.report` after runtime validation and renders a compact
post-match H2H summary for shots, shots on target and possession.  A malformed
or incomplete report is safely omitted rather than guessed or rendered as
client-generated football data.

2D/multiplayer acceptance update: the server suite now contains a single
two-human full-match regression.  It opens a canonical fixture, applies a
server-authorized substitution from the bench, observes the half-time ready
barrier, runs to `MatchFinished`, and compares the published final score with
the persisted completed fixture.  `cargo test -p albion_server` passes 29
tests and strict server clippy passes.

Phase 12 update: strict `cargo clippy --workspace --all-targets -- -D warnings`
now passes after correcting protocol-test numeric grouping and small
non-behavioural lint violations in rules, server and snapshot code. A new
10-season daily-turn soak regression checks structural game invariants every
31 days and passes in 8.30 seconds. The 100,000-match seed-42 sim-bench
calibration was also executed. Remaining Phase 12 work is operational: clean
machine/package verification and real two-machine private-network smoke.

2D localization update: the Canvas event overlay now receives a localized
label from its owning Match Centre instead of rendering a protocol event name.
Thai translations for the full match event-type set were completed, including
goal, penalty goal, red card and substitution. The H2H view uses its existing
authoritative-event formatter; neither path changes canonical match data.

2D diagnostics update: development builds can now enable
`?albion2dDebug=1` to inspect the active semantic event, presentation clip,
match minute, ball coordinates, active player IDs, FPS, frame time and entity
count. The same developer-only mode draws normalized pitch-zone guides and
the current ball movement path. It is not included in normal user mode and
does not alter the authoritative event stream or match state.
