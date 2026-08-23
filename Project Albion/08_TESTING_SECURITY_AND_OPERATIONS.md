# Project Albion — Testing, Security and Operations

## Quality strategy

Project Albion is a long-running stateful simulation. A corruption bug in season 6 or after reconnect is more serious than a cosmetic defect.

Testing layers:
1. domain unit tests;
2. property/invariant tests;
3. match calibration/statistics;
4. database migration/integration;
5. protocol/server;
6. frontend;
7. full E2E;
8. long soak.

## Mandatory CI gates

Backend:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Frontend:

```bash
npm test
npm run build
```

Also run project lint/typecheck/schema generation if present.

Never disable tests merely to make CI green.

## Domain invariant tests

At minimum:
- starting XI has 11 distinct eligible players;
- valid goalkeeper rule;
- contract dates ordered;
- fees/wages non-negative;
- buyer cannot buy own player;
- player cannot have two permanent current clubs;
- table P = W + D + L;
- points/tie-break sorting correct;
- promotion/relegation counts correct;
- registration obeys rules.

## Property-based tests

Strong candidates:
- schedule generation;
- standings sorting;
- transfer installment sum;
- buyer/seller money transfer conservation;
- serde round trips;
- ruleset parser;
- match stat reconciliation;
- idempotent season rollover.

Use `proptest` or project-standard equivalent.

## Match tests

Determinism:
- same input + seed + version => exact same report/events.

Variation:
- different seeds produce distribution.

Invariants:
- score/events/stats reconcile.

Strength:
- stronger synthetic team wins more over large N.

Tactics:
- trade-offs exist;
- no obvious dominant extreme.

Performance:
- benchmark in release mode.

## Monte Carlo suite

Slow/nightly/manual:
- 100k+ matches;
- multiple rating gaps;
- several tactical profiles.

Export:
- JSON/CSV aggregate;
- goals;
- H/D/A;
- scorelines;
- shots/SOT;
- possession;
- cards;
- injuries;
- stronger-team win rate.

Use broad regression bands rather than brittle exact averages.

## Competition tests

For each season ruleset:
- participant count;
- fixture count;
- no duplicate impossible fixture;
- home/away pairing;
- playoffs;
- promotion/relegation;
- cup rounds/winner;
- continental qualification;
- season rollover.

Run structural fast-sim at least 20 seasons.

## Finance tests

Money is integer/exact.

Test:
- completed transfer creates correct buyer/seller ledger;
- installment creates future liability/receivable;
- canceled deal leaks no money;
- duplicate command does not double pay;
- wage aggregate correct;
- prize paid once;
- rollover idempotent;
- numeric bounds safe.

## Data pipeline tests

Fixture scenarios:
- player moves club;
- same name + different DOB;
- accents/aliases;
- missing DOB;
- loan;
- duplicate source row;
- promoted/relegated club;
- external-ID remap.

Test:
- identity;
- ambiguity review;
- diff;
- hash;
- immutable published snapshot;
- active career unaffected.

## Database tests

Every migration:
- creates previous version fixture;
- runs migration;
- verifies rows/invariants;
- verifies schema version;
- tests failure safety if practical.

Other:
- foreign keys;
- transaction atomicity;
- save reopen;
- backup restore;
- integrity check.

Do not test only clean new DBs.

## Protocol tests

Golden/round-trip:
- every command;
- every event;
- error envelope;
- optional unknown fields where compatibility allows;
- handshake.

Version:
- incompatible protocol rejected clearly;
- compatible patch app accepted.

## Concurrency tests

Simulate two clients:
- simultaneous Ready;
- simultaneous same-player bid;
- duplicate command;
- stale revision;
- reconnect;
- server restart;
- same club claim;
- tactical commands at same safe boundary.

Assertions:
- one revision sequence;
- one canonical side effect;
- no corruption;
- both clients converge.

## Network chaos

Inject:
- disconnect;
- delayed client requests;
- duplicate retry;
- reconnect;
- server pause/restart.

WebSocket orders frames on one connection, but retries across connections can duplicate intent. Idempotency must handle it.

## E2E happy path

Automate/script:
1. launch server;
2. create career from synthetic snapshot;
3. connect A;
4. connect B;
5. claim two clubs;
6. change tactics;
7. make transfer bid;
8. both Ready;
9. reach fixture;
10. play human-v-human live match;
11. save/checkpoint;
12. disconnect guest;
13. reconnect;
14. advance;
15. restart server;
16. reopen;
17. verify state.

## Release-blocking 10-season soak

Headless harness uses two deterministic auto-ready human bots or test clients.

Include:
- transfers;
- contracts;
- injuries;
- training/development;
- scouting;
- youth;
- AI;
- promotions;
- cups;
- continental competition;
- finance.

At intervals:
- close server/core;
- reopen DB;
- continue.

Each season assert:
- correct club membership counts;
- no orphan rows;
- no duplicate current permanent contracts;
- every required competition has valid conclusion;
- finances numeric;
- player population sane;
- youth produced;
- history persisted;
- no unresolved impossible fixture;
- migration/version metadata intact.

Export a summary artifact.

## Performance tests

Measure:
- DB open;
- player search;
- squad query;
- day processing;
- instant match;
- matchday batch;
- season rollover;
- WS payload;
- checkpoint.

Create regression alerts after baseline is known.

## Security scope

Private two-user game, but standard safety still applies.

Validate all network input:
- IDs;
- enums;
- string length;
- money bounds;
- collection size;
- manager authorization.

Never:
- accept raw SQL from client;
- accept arbitrary remote filesystem path;
- log join/reconnect secret;
- commit API credentials;
- trust client club ownership.

## Bind policy

Defaults:
- localhost for dev/embedded local;
- explicit host choice to bind LAN/private interface.

Recommend private network for remote friend.

Do not expose public internet by default.

## Secrets

Join/reconnect tokens:
- generated cryptographically;
- not logged;
- stored locally with reasonable OS/app protection;
- revocable/rotatable.

Data-source credentials:
- env/secure local config;
- never committed.

Provide `.env.example` only when needed and with placeholders.

## Dependency audits

Where supported:
- cargo audit;
- npm audit review;
- lockfiles committed.

Do not blindly auto-upgrade major dependencies without tests.

## Save integrity

Before risky migration:
- backup.

On suspicious shutdown:
- integrity check.

On corruption:
- preserve original;
- offer/use last known-good backup;
- produce diagnostics.

Never overwrite the only good copy.

## Logging

Structured levels:
- ERROR
- WARN
- INFO
- DEBUG
- TRACE

Production default INFO.

Context fields:
- career_id
- revision
- manager_id
- command_id
- match_id
- fixture_id
- versions

Do not leak hidden scouting data to wrong client log.

## Diagnostics bundle

Recommended:
`Export Diagnostics`

Contains:
- app/server/protocol/save versions;
- ruleset/snapshot IDs;
- recent logs;
- DB integrity result;
- sanitized config.

Exclude:
- join secrets;
- source credentials;
- full career DB unless user explicitly selects.

## Crash handling

Server:
- panic hook/tracing;
- transactional critical operations;
- safe restart.

Client crash:
- cannot corrupt canonical save because server owns state.

Live match:
- checkpoint safe states to permit recovery/replay.

## Backup policy

Default:
- autosave/checkpoint after critical actions and configured game-day interval;
- retain 5 rotating backups;
- extra season-rollover backup;
- manual export.

Profile to avoid excessive blocking.

## Windows release packaging

Primary release:
- Tauri desktop client;
- bundled or adjacent `albion-server`;
- licenses/notices;
- hosting guide;
- data updater tool or maintainer package;
- no embedded credentials.

## Developer setup

Document:
- Rust toolchain;
- Node;
- Tauri prerequisites;
- install;
- dev run;
- server run;
- two-client local run;
- tests;
- data fixtures;
- package.

Prefer a helper command/script such as:
`npm run dev:multiplayer`
that can launch server + two local client instances in dev.

## Environment config

Configurable:
- bind address;
- log level;
- save dir;
- snapshot dir;
- dev ports;
- autosave settings.

Keep gameplay rules in rulesets, not env vars.

## Release policy

Use SemVer for app.

Pre-1.0 still gets safe migrations.

Release candidate must pass:
- CI;
- soak;
- clean Windows install;
- two-machine private-network smoke;
- previous supported save migration;
- English/Thai smoke;
- license review.

## Bug severity

Blocker:
- save loss/corruption;
- cannot complete season;
- wrong club controlled;
- divergent multiplayer;
- ordinary retry duplicates money/transfer;
- deterministic engine reproducibility broken.

Critical:
- core transfer/contracts broken;
- match cannot finish;
- promotion/relegation wrong;
- reconnect routinely fails.

Major:
- significant workaround exists.

Minor:
- cosmetic/localization.

v1 ships with zero known blocker or critical defects.

## Definition of Done

Operations/QA complete when:
- CI reproducibly green;
- backups restore;
- diagnostics available;
- two machines connect privately;
- server restart recovers career;
- 10-season soak passes;
- no secrets in repo;
- protocol boundaries validated;
- clean Windows package works.
