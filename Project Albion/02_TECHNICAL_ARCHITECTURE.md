# Project Albion — Technical Architecture

## Objective

Evolve OpenFootManager into a shared-core, server-authoritative, two-player application while preserving tested football logic. The architecture must support a headless server, two Tauri desktop clients, deterministic match simulation, relational saves, immutable data snapshots and automated tests without GUI.

## Runtime processes

### `albion-client`
Tauri desktop application:
- React + TypeScript UI;
- local settings and cache;
- WebSocket/HTTP client;
- can launch a local server child process in Host mode;
- never owns canonical football state.

### `albion-server`
Headless Rust executable:
- opens one career;
- accepts up to two manager sessions;
- validates all commands;
- owns game clock;
- runs game core, AI and match engine;
- persists canonical state;
- broadcasts versioned updates.

### `albion-data`
Offline maintenance tool:
- imports real-world source files/providers;
- normalizes;
- resolves identity;
- validates;
- builds game attributes;
- produces diff;
- publishes immutable snapshot.

## Context

```text
Data sources/manual files
        ↓
 albion-data
        ↓
 immutable snapshot
        ↓ new career only
┌───────────────┐     private network      ┌─────────────────────┐
│ Client A      │<=========================>│ albion-server       │
└───────────────┘      WS + minimal HTTP    │ game core + AI      │
                                             │ match engine + DB   │
┌───────────────┐<=========================>│                     │
│ Client B      │                           └─────────┬───────────┘
└───────────────┘                                     ↓
                                                career SQLite
```

## Workspace target

Adapt rather than blindly move upstream files.

```text
src-tauri/crates/
├─ domain/            # entities + invariants
├─ engine/            # deterministic match engine
├─ db/                # SQLite repositories/migrations
├─ ofm_core/          # calendar/orchestration
├─ albion_rules/      # versioned competition/business rules
├─ albion_ai/         # AI manager decisions
├─ albion_protocol/   # wire commands/events/views
└─ albion_server/     # authoritative server
```

Frontend remains under `src/` unless upstream structure changes.

## Dependency boundaries

### Domain
Must not depend on Tauri, React, HTTP, WebSocket or concrete SQLite connection.

### Engine
Inputs:
- immutable `MatchInput`;
- explicit `MatchSeed`;
- engine configuration/version.

Outputs:
- event list;
- match report.

No DB/network/wall-clock.

### Core
Coordinates:
- date advancement;
- competition service;
- transfer service;
- contract service;
- training/development;
- medical;
- scouting;
- finance;
- AI;
- match application.

### DB
Implements persistence traits:
- transactions;
- repositories;
- migrations;
- backups;
- integrity checks.

### Protocol
Contains versioned serde messages and view models. Avoid leaking internal DB/domain structs.

### Server
Owns mutation order and manager sessions.

### Client
Owns only local display state, forms, settings and cache.

## Ports/adapters

Use traits at changing boundaries.

```rust
pub trait MatchSimulator {
    fn simulate(&self, input: MatchInput, seed: MatchSeed) -> Result<MatchReport>;
}

pub trait SnapshotLoader {
    fn validate(&self, snapshot: &SnapshotPackage) -> Result<SnapshotManifest>;
    fn seed_career(&self, tx: &mut dyn CareerTx, snapshot: &SnapshotPackage) -> Result<()>;
}

pub trait CareerRepository {
    fn load_meta(&self) -> Result<CareerMeta>;
    fn begin_tx(&mut self) -> Result<Box<dyn CareerTx + '_>>;
}
```

Equivalent safe design is allowed if current upstream code suggests a cleaner pattern.

## Mutation model

Use one logical mutation lane per career.

Recommended server design:
- Tokio task owns `CareerSession`;
- network handlers send typed requests into MPSC channel;
- session processes one state-changing command at a time;
- expensive simulation can use workers from immutable inputs;
- result application returns to canonical lane in deterministic order.

Command lifecycle:

```text
receive
→ authenticate
→ validate protocol/session
→ validate expected revision
→ domain validation
→ DB transaction
→ apply
→ increment revision
→ commit
→ broadcast acknowledgement + deltas
```

No two handlers directly write canonical DB concurrently.

## Career revision

Maintain `career_revision: u64`.

Every committed canonical mutation increments revision.

Clients keep:
- last confirmed revision;
- pending command IDs;
- connection/session token.

Revision gap:
- request resync;
- server sends manager-specific view snapshot;
- client discards stale canonical cache.

## Idempotency

Every state-changing command includes a UUID `command_id`.

Server retains recent command results or an idempotency table for critical commands.

Retry of same command:
- never applies transfer/payment/tactic action twice;
- returns the prior result.

## Command/query split

Commands:
- SetStartingXI
- SetTactics
- SubmitTransferBid
- RespondTransferOffer
- SubmitContractOffer
- SetTrainingPlan
- MarkReady
- ApplyLiveMatchCommand

Queries/views:
- Dashboard
- Squad
- PlayerProfile
- CompetitionTable
- Schedule
- TransferCentre
- Scouting
- Finances
- MatchView

Do not give clients direct SQL.

## SQLite strategy

One DB per career.

Enable:
- foreign keys;
- WAL when appropriate;
- busy timeout;
- transactions;
- indexed queries.

Never use binary floating point for money.

Do not keep the full canonical career only as one JSON blob.

## Versioning

Persist:
- application version;
- protocol version;
- save schema;
- source snapshot;
- snapshot schema;
- ruleset;
- match engine;
- rating model.

Compatibility handshake must compare relevant versions.

## Server transport

Minimal HTTP:
- `GET /healthz`
- `GET /readyz`
- `GET /version`
- `POST /api/v1/session/join`
- `POST /api/v1/session/reconnect`
- `WS /ws`

Most game interaction uses typed WebSocket messages.

## Wire envelope

Example:

```json
{
  "protocol_version": 1,
  "message_id": "uuid",
  "kind": "command",
  "career_id": "uuid",
  "manager_id": "uuid",
  "expected_revision": 1234,
  "payload": {
    "type": "MarkReady",
    "body": {}
  }
}
```

Use tagged Rust enums with serde, not arbitrary string maps in internal code.

## Server response model

Required:
- Hello
- CommandAck
- CommandRejected
- StateDelta
- ViewSnapshot
- GameTimeChanged
- ReadyStateChanged
- MatchOpened
- MatchEventBatch
- MatchState
- MatchFinished
- ServerNotice
- Ping/Pong

Errors are stable codes + parameters, not raw English/backtrace.

Examples:
- AUTH_INVALID
- PROTOCOL_INCOMPATIBLE
- STALE_REVISION
- CLUB_ALREADY_CONTROLLED
- INVALID_LINEUP
- TRANSFER_WINDOW_CLOSED
- INSUFFICIENT_TRANSFER_BUDGET
- INSUFFICIENT_WAGE_BUDGET
- REGISTRATION_INVALID
- MATCH_COMMAND_NOT_ALLOWED
- SAVE_CORRUPT
- SNAPSHOT_INVALID

Client localizes codes.

## Tauri role

After server migration, Tauri should be thin:
- desktop window/shell;
- settings paths;
- start/stop bundled server for Host Game;
- file picker/import/export;
- reconnect token storage;
- OS integration.

Do not leave football truth split between Tauri local state and server.

## Embedded host mode

Best UX:

1. Host clicks Host Game.
2. Chooses new/existing career.
3. Tauri launches `albion-server` child process.
4. Waits for `/readyz`.
5. Host client joins locally.
6. UI shows private address/join code.
7. Guest connects remotely.

Server must also work as a standalone headless executable.

## Ruleset design

Store season-specific rules in versioned files such as:

```text
data/rulesets/
  england_2026_27.yaml
  continental_2026_27.yaml
```

Rules can define:
- club count;
- round-robin rounds;
- points/tie breakers;
- promotion/relegation;
- playoffs;
- registration;
- substitute count/windows;
- transfer windows;
- loan restrictions;
- financial constraints;
- cup format;
- continental qualification.

Game code validates rulesets rather than hardcoding one season forever.

## Domain events

Emit significant events after successful mutations:
- TransferCompleted
- ContractSigned
- PlayerInjured
- PlayerRecovered
- MatchFinished
- PlayerSuspended
- CompetitionWon
- ClubPromoted
- ClubRelegated
- SeasonRolledOver
- YouthPlayerCreated
- ManagerHired/Fired

Use them for history, inbox/news and multiplayer deltas.

## Day processing phases

Make deterministic and explicit:

```text
10 deadlines/expiries
20 scheduled payments
30 fitness recovery
40 training
50 medical recovery
60 scouting
70 AI planning
80 registrations/windows
90 fixtures/matches
100 discipline/standings
110 finances
120 inbox/news
130 autosave/checkpoint
140 next-blocker detection
```

Exact phase numbering can differ; ordering must be tested and documented.

## Performance targets

Release build, ordinary desktop:
- common server query p95 under ~100 ms locally/private network;
- normal command acknowledgement under ~250 ms excluding intentional simulation;
- instant match preferably <10 ms and hard target <25 ms;
- English matchday simulation comfortably sub-second where possible;
- 10-season soak runs much faster than real time.

Measure before optimizing.

## Observability

Use structured `tracing`.

Include when relevant:
- career_id
- revision
- manager_id
- command_id
- fixture_id
- match_id
- versions

Never log join/reconnect secrets.

## Architecture acceptance

Architecture phase is sound when:
- engine tests run with no DB/network;
- server tests run with no Tauri GUI;
- frontend can run against fake protocol service;
- data tool runs independently;
- career can open headlessly;
- concurrent client commands produce one revision sequence;
- reconnect cannot duplicate a state-changing action.
