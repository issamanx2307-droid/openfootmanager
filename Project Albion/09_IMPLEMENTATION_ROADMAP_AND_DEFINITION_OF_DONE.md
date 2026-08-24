# Project Albion — Implementation Roadmap and Definition of Done

## Execution rule for the coding agent

Create and maintain:
`docs/project-albion/IMPLEMENTATION_STATUS.md`

Statuses:
- NOT STARTED
- IN PROGRESS
- DONE
- BLOCKED

A phase is DONE only after implementation + tests + build gates.

Do not stop after planning/scaffolding. Continue in order until v1 release criteria pass.

If production real-world data is unavailable, mark only production data population as BLOCKED/EXTERNAL. Complete the entire importer, game, server, UI and test system using synthetic fixtures.

---

# Phase 0 — Baseline audit

## Tasks
- clone/fork OpenFootManager;
- record exact upstream commit;
- read upstream agent/architecture/game/match/save docs;
- install deps;
- run all upstream tests;
- run desktop app;
- identify domain/engine/db/core/Tauri/frontend boundaries;
- create Project Albion status file;
- document existing failures only if genuinely pre-existing.

## Acceptance
- reproducible local build;
- baseline tests pass or pre-existing failures precisely recorded;
- current save and engine paths understood.

---

# Phase 1 — Versions, protocol and rules foundation

## Tasks
- central app/protocol/save/snapshot/engine/rules/rating versions;
- create/adapt `albion_rules`;
- define ruleset schema;
- initial England rules pack;
- competition/substitution/registration/window config;
- create `albion_protocol`;
- typed command/event/view enums;
- TS type generation or reliable shared schema;
- stable error codes;
- compatibility handshake.

## Acceptance
- rules parse/validate;
- protocol round-trip tests;
- UI/core no longer relies on inappropriate hardcoded season rule constants.

---

# Phase 2 — Relational per-career save

## Tasks
- one SQLite DB per career;
- migrations;
- repositories;
- transactions;
- version metadata;
- autosave;
- backup rotation;
- integrity check;
- save/open/reopen headless;
- migrate relevant upstream save if practical.

## Acceptance
- create, mutate, close, reopen exact state;
- migrations tested;
- failed risky migration does not destroy last good backup.

---

# Phase 3 — Snapshot/data pipeline

## Tasks
- `tools/data_pipeline`;
- normalized schemas;
- manual CSV;
- manual JSON;
- deterministic test provider;
- provider interface;
- identity mapping;
- aliases/fuzzy review;
- provenance;
- validation;
- diff JSON + human report;
- rating model v1;
- immutable publish/hash;
- career seed importer.

## Synthetic data
Create fictional test clubs/players sufficient for all automated paths. Do not commit unapproved real data as test fixtures.

## Acceptance
- Snapshot A publishes;
- Snapshot B contains transfers;
- diff correct;
- identity stable;
- old career unchanged;
- new career reflects B;
- same inputs produce same content hash.

---

# Phase 4 — Competition/calendar

## Tasks
- generic league;
- standings/tie-breakers;
- schedule;
- playoffs;
- promotion/relegation;
- National League feeder background;
- FA Cup;
- EFL Cup;
- Community Shield;
- continental formats;
- qualification;
- season rollover;
- history.

## Acceptance
Fast-sim at least 20 structural seasons:
- correct memberships;
- no duplicate impossible fixtures;
- playoff/promotions correct;
- cups/continental winners;
- rollover idempotent.

---

# Phase 5 — Core management systems

## Squad
- player positions/roles;
- XI/bench validation;
- registration;
- auto-selection.

## Transfers
- list/search;
- bids/counters;
- installments;
- basic add-ons;
- loans;
- deadline;
- AI competition.

## Contracts
- wage/term/role;
- bonuses baseline;
- negotiation;
- expiry/free agent.

## Finance
- ledger;
- transfer/wage budgets;
- commitments;
- revenue/cost;
- financial-rule hook.

## Medical
- condition;
- fatigue;
- sharpness;
- injuries;
- suspensions.

## Training/development
- weekly plans;
- individual focus;
- age curves;
- potential;
- gradual development.

## Scouting
- manager-specific knowledge;
- assignment;
- report;
- shortlist.

## Acceptance
Each system has domain + persistence tests.
One headless human-controlled club can complete full season without manual DB edits.

---

# Phase 6 — Albion match engine v1

## Tasks
- stable simulator interface;
- explicit deterministic seed;
- possession/zone event chain;
- action selection/resolution;
- attribute mapping;
- tactical effects;
- roles;
- set pieces;
- cards;
- goalkeeper;
- fitness;
- substitutions;
- live command support;
- reports/invariants;
- engine version;
- calibration CLI;
- benchmarks.

## Acceptance
- exact deterministic replay;
- instant/live no-intervention consistency;
- 100k calibration runner;
- broad plausible distributions;
- tactical trade-offs;
- stat invariants;
- acceptable performance.

Do not delay networking for perfection beyond acceptance.

---

# Phase 7 — AI managers

## Tasks
- persistent manager profile;
- depth chart;
- legal XI;
- tactical identity;
- in-match changes;
- fatigue rotation;
- recruitment needs;
- target scoring;
- surplus selling;
- renewals;
- loans/prospects;
- affordability;
- guardrails;
- basic AI hire/fire if safe.

## Acceptance
Multi-season AI run:
- legal lineups;
- sane squad composition;
- transfer activity;
- no systematic budget corruption;
- contract management;
- visible tactical variation.

---

# Phase 8 — Authoritative server

## Tasks
- `albion_server` binary;
- health/ready/version;
- typed WebSocket;
- join secret;
- host/guest slots;
- club claim;
- command lane;
- revision;
- idempotency;
- manager-specific views;
- Ready barrier;
- advancement;
- reconnect;
- disconnect policy;
- live match coordinator;
- human-v-human;
- simultaneous human-v-AI;
- save lifecycle.

## Acceptance
Automated two-client integration:
- connect;
- different club enforced;
- simultaneous commands consistent;
- duplicate retry safe;
- Ready works;
- live match works;
- reconnect;
- restart;
- same canonical state.

---

# Phase 9 — Complete desktop client migration

## Tasks
- network service abstraction;
- replace authoritative local Tauri mutations with server commands;
- server view cache;
- dashboard;
- inbox;
- squad;
- tactics;
- schedule;
- competitions;
- transfers;
- contracts;
- scouting;
- training;
- medical;
- finances;
- staff/club;
- profiles;
- match preview;
- live match centre;
- post-match;
- host/join lobby;
- Ready;
- reconnect UX;
- localized errors.

## Acceptance
Core game flow requires no console/DB editing.

---

# Phase 10 — Thai + accessibility

## Tasks
- audit strings;
- complete English keys;
- full Thai v1 translation;
- mixed Thai/Latin tests;
- keyboard;
- focus;
- screen reader labels;
- non-color state;
- reduced motion;
- contrast;
- 1280×720 checks;
- deployment seam: configurable CORS origins, standalone Linux server config,
  persistent career path and health/readiness probes;
- Oracle smoke deployment with HTTPS WebSocket proxy, restart/reconnect and
  SQLite backup verification.

## Acceptance
Core E2E completable in both English and Thai without major clipped/blocked UI;
the same authoritative multiplayer core can run from a clean Linux service
environment and be reached through an HTTPS WebSocket endpoint. Production
account authentication and multi-career tenancy remain out of scope for this
phase.

---

# Phase 11 — Production-intended snapshot

## Tasks
When operator supplies/approves actual data:
- import;
- resolve identities;
- validate league memberships;
- verify freeze-date roster;
- generate Albion ratings;
- validate financial/reputation seeds;
- inspect outliers;
- publish snapshot;
- create Season Reset smoke career.

Never fabricate missing real facts.

If dataset is not present:
- provide exact CSV/JSON templates and instructions;
- keep test snapshot synthetic;
- continue remaining engineering.

---

# Phase 12 — Balance, soak, hardening

## Tasks
- 100k+ match calibration;
- 10-season full soak;
- periodic process restart;
- DB integrity checks;
- network chaos;
- two-machine private-network smoke;
- performance profile;
- AI economy fixes;
- backup/restore;
- Windows package;
- licenses/notices;
- hosting guide;
- data updater guide;
- clean-machine install.

## Acceptance
All v1 release gates pass.

---

# Phase 13 — Release candidate checklist

## Build
- [ ] frontend tests pass
- [ ] production frontend build passes
- [ ] Rust fmt passes
- [ ] Rust clippy passes
- [ ] Rust workspace tests pass
- [ ] protocol types/schema synchronized
- [ ] data/rules validation passes

## Data
- [ ] snapshot manifest valid
- [ ] snapshot content hash valid
- [ ] no critical identity conflict
- [ ] Season Reset has zero real results
- [ ] source snapshot/rules/rating versions stored

## English gameplay
- [ ] Premier League season
- [ ] Championship season
- [ ] League One season
- [ ] League Two season
- [ ] correct promotion/relegation
- [ ] playoffs
- [ ] FA Cup
- [ ] EFL Cup
- [ ] Community Shield path
- [ ] continental qualification/competition
- [ ] transfers
- [ ] contracts
- [ ] finance
- [ ] injuries/discipline
- [ ] training/development
- [ ] scouting uncertainty
- [ ] youth
- [ ] AI clubs

## Match engine
- [ ] deterministic replay
- [ ] live match finish
- [ ] tactical commands apply
- [ ] substitutions legal
- [ ] stats reconcile
- [ ] calibration sanity bands
- [ ] performance acceptable

## Multiplayer
- [ ] Host Game
- [ ] guest join
- [ ] same club rejected
- [ ] Ready barrier
- [ ] human-v-human one state
- [ ] separate simultaneous matches
- [ ] reconnect
- [ ] duplicate command safe
- [ ] stale revision handled
- [ ] server restart/reload

## Save
- [ ] transactions
- [ ] autosave
- [ ] rotating backup
- [ ] migration
- [ ] integrity check
- [ ] restore
- [ ] 10-season soak with restarts
- [ ] snapshot update cannot mutate active career

## UI
- [ ] all core routes
- [ ] English
- [ ] Thai
- [ ] 1280×720
- [ ] keyboard core flow
- [ ] Ready/connection obvious
- [ ] actionable errors
- [ ] hidden scouting values not leaked

## Release
- [ ] clean Windows install
- [ ] two-machine private play
- [ ] LAN/Tailscale documentation
- [ ] server binary bundled/available
- [ ] licenses/notices
- [ ] no credentials
- [ ] diagnostics
- [ ] blocker bugs = 0
- [ ] critical bugs = 0

---

# Definition of Done per feature

A human-facing feature is DONE only when:
1. domain behavior exists;
2. persistence exists;
3. server/protocol exists;
4. UI exists;
5. English + Thai text exists;
6. tests exist;
7. failures/errors are handled;
8. docs/status updated;
9. full quality gates pass.

A backend transfer function with no multiplayer/UI path is not a completed transfer feature.

---

# Autonomous decision rules

When unspecified:

Use upstream pattern if:
- tested;
- compatible with server authority;
- meets requirement.

Use configuration if:
- football rules change;
- balance constants are tunable;
- timeouts vary by environment.

Use deterministic seed if:
- random result becomes persistent.

Use DB transaction if:
- multiple records form one football action.

Use server validation if:
- client wants canonical state change.

Reduce scope only for explicit non-goals or optional embellishment. Do not cut required features to claim completion.

---

# Prohibited shortcuts

Do not:
- sync save files as multiplayer;
- simulate human-v-human twice;
- store canonical world in browser;
- import real results into Season Reset;
- update active careers from new snapshots;
- leak hidden scouting attributes;
- use float for money;
- skip migrations;
- give AI hidden money/ratings cheat;
- invent real-world facts;
- require an LLM to play;
- disable tests;
- stop at mockups/TODOs.

---

# Final acceptance scenario

Demonstrate on two Windows PCs:

1. Host launches Project Albion.
2. Host creates `Albion Friends` from a published post-window snapshot.
3. Host selects one English club.
4. Server shows private connection information.
5. Guest connects.
6. Guest selects a different English club.
7. Both see same date/world.
8. Both independently manage squad/tactics.
9. At least one transfer negotiation completes.
10. Both press Ready.
11. Server advances and AI clubs act.
12. Both reach fixtures.
13. Human-v-AI match can complete.
14. Human-v-human fixture uses one shared live state.
15. Result updates one shared table.
16. Guest disconnects/reconnects.
17. Career remains correct.
18. Server shuts down/restarts.
19. Save resumes exactly.
20. Season ends correctly.
21. cups/promotions/continental slots resolve.
22. next season begins.
23. automated equivalent survives 10 seasons.

When this scenario and every checklist item pass, Project Albion v1 is complete. Remaining polish belongs to v1.1+.
