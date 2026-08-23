# Project Albion — Multiplayer and Network Protocol

## Scope

v1:
- one authoritative server;
- two human manager slots;
- private LAN/Tailscale-style network;
- reconnect;
- no public matchmaking;
- no public account service.

The two players are trusted friends, but server authority is still required to prevent accidental divergence and duplicate actions.

## Host flow

1. Host clicks Host Game.
2. Selects new/existing career.
3. New career selects snapshot/ruleset and host club.
4. Tauri launches/connects to `albion-server`.
5. Server opens DB, runs migrations, validates versions.
6. Host joins manager slot.
7. UI displays private address + join code.
8. Guest connects.
9. Guest selects a different eligible club.
10. Both enter shared career.

## Join secret

Use cryptographically random session secret.
A short friendly code may map to the full secret.

Requirements:
- never log plaintext;
- rotate on host request;
- private network recommended;
- no need for full cloud identity.

## Reconnect token

After successful join, issue reconnect token bound to:
- career;
- manager slot;
- expiration/session policy.

Client stores locally.

Reconnect:
- authenticate;
- restore same manager;
- compare revision;
- fetch missing delta or full manager-specific snapshot.

Never accidentally create a second manager on reconnect.

## Session states

Server:
- Starting
- Lobby
- Active
- Paused
- Saving
- Closing
- Error

Manager:
- Empty
- Connected
- DisconnectedGrace
- Disconnected

Game-time:
- WaitingForReady
- Advancing
- HumanMatchPending
- LiveMatch
- PostMatch
- SeasonTransition

Use explicit enums.

## Ready barrier

Track both managers.

When Manager A Ready + Manager B Ready:
1. lock barrier generation;
2. process dates until next human blocker;
3. emit progress/current date;
4. stop at blocker;
5. clear readiness.

`MarkReady` is idempotent.

Manager can cancel Ready before advance lock.

Default force advance: disabled.

Optional host force advance may be enabled only in session settings and must respect mandatory lineup/match safety.

## Command envelope

```json
{
  "protocol_version": 1,
  "message_id": "uuid",
  "kind": "command",
  "career_id": "uuid",
  "manager_id": "uuid",
  "expected_revision": 824,
  "payload": {
    "type": "SubmitTransferBid",
    "body": {
      "player_id": "uuid",
      "upfront_minor": 2500000000
    }
  }
}
```

All money integers.

## Acknowledgement

Success:

```json
{
  "kind": "command_ack",
  "command_id": "uuid",
  "applied_revision": 825,
  "result": {}
}
```

Failure:

```json
{
  "kind": "command_rejected",
  "command_id": "uuid",
  "current_revision": 825,
  "error": {
    "code": "STALE_REVISION",
    "params": {}
  }
}
```

Errors localize client-side.

## Revision policy

Strict revision for:
- transfer/finance;
- contract;
- registration;
- time advance;
- match commands where order matters.

Relaxed/idempotent can be used for:
- mark message read;
- shortlist metadata;
- cosmetic/local preferences.

Document policy per command.

## Broadcast audiences

Never broadcast hidden information.

Audience:
- All;
- Manager(id);
- MatchParticipants(match_id);
- HostAdmin.

Examples:
- completed transfer: All;
- private scouting report: Manager only;
- unsubmitted tactic form state: client local;
- accepted live tactic command: participant/own side according to visibility rule.

## Manager-specific views

Do not serialize true hidden player attributes and rely on UI hiding them.

Server builds a view based on scouting knowledge:
- factual public data;
- known/ranged game estimates;
- hidden values omitted.

This is required even in friendly private play.

## WebSocket events

Required:
- Hello
- CommandAck
- CommandRejected
- StateDelta
- ViewSnapshot
- ReadyStateChanged
- GameTimeChanged
- MatchOpened
- MatchEventBatch
- MatchState
- MatchFinished
- ServerNotice
- Ping
- Pong

## Live match session

Properties:
- match_id;
- fixture_id;
- seed;
- phase;
- match second;
- participants;
- simulation state;
- command sequence;
- event cursor.

Human-v-human subscribes both clients to the same instance.

## Match event streaming

Batch format concept:

```json
{
  "type": "MatchEventBatch",
  "match_id": "uuid",
  "from_seq": 120,
  "to_seq": 128,
  "events": []
}
```

Reconnect:
- current match snapshot;
- then retained events after cursor where possible.

UI does not infer canonical score from animations.

## Tactical commands

Server validates:
- sender controls side;
- current phase;
- player eligibility;
- substitution counts/windows;
- no duplicates;
- tactic values valid.

Response records `effective_match_second`.

## Human-v-human

One simulation only.

Rules:
- each controls own team;
- one shared score/time;
- shared match speed;
- halftime tactical break waits for both `ReadyForSecondHalf`;
- command application at deterministic safe points.

No client may locally simulate and submit a result.

## Separate simultaneous human matches

If:
- Human A vs AI;
- Human B vs AI;
- same/overlapping matchday;

server can create two live sessions.

World cannot advance past the matchday barrier until both are completed.

The player who finishes early may browse but cannot move shared world beyond the unfinished match.

## Disconnect outside match

- keep server/career alive;
- mark user disconnected;
- Ready barrier waits if needed;
- guest can reconnect.

## Disconnect during human-v-human

Default:
- pause at next safe boundary;
- allow reconnect grace.

Optional preconfigured policy:
- AI takeover after grace.

Never silently enable AI takeover mid-career without session policy.

## Disconnect during human-v-AI

Pause that human live match.
Other human's separate match may continue.
Shared world waits.

## Heartbeat

Use ping/pong every roughly 15–30 seconds with configurable timeout.

Reconnect with capped exponential backoff.

Detect app sleep and network change.

## Server restart

Graceful:
1. stop commands;
2. finish/rollback transaction;
3. checkpoint DB;
4. close sessions;
5. restart;
6. clients reconnect/resync.

Crash during live match:
- persist checkpoints at safe intervals;
- minimum at halftime and accepted tactical changes, plus periodic game-minute checkpoint if cheap;
- replay from seed + checkpoint if needed.

## Protocol source of truth

Prefer Rust typed models + generated TypeScript definitions.

Avoid independently hand-maintaining both sides.

Support:
- tagged enums;
- explicit protocol version;
- tolerant optional fields where safe;
- major incompatibility rejection.

## Session settings

v1:
- career_name;
- host_force_advance=false;
- disconnect_ai_takeover=false;
- halftime timeout optional/off;
- live speed policy;
- autosave interval;
- backup count.

Settings that affect fairness are visible to both.

## Server CLI

Concept:

```bash
albion-server   --save "/path/career.db"   --bind 0.0.0.0:38421   --private-mode   --log-level info
```

New career can be created by client or server command.

## Health

`/healthz`: process alive.
`/readyz`: DB migrated + career loaded + command lane running.
`/version`: version information only, no secrets.

## Private network

LAN:
- bind private LAN;
- guest uses local IP.

Remote:
- Tailscale recommended;
- both on same tailnet;
- host uses Tailscale IP/DNS;
- no router port forward.

Tailscale is documentation, not runtime dependency.

## Concurrency tests

Automate:
1. same player bid simultaneously;
2. both Ready simultaneously;
3. duplicate Ready retry;
4. transfer retry after timeout;
5. guest disconnect during advancement;
6. server restart;
7. both tactical commands in same match boundary;
8. stale lineup command;
9. both attempt same club claim.

Expected:
- one canonical revision sequence;
- no duplicate money/action;
- explicit conflict/rejection where needed;
- both clients converge.

## Definition of Done

- two client processes connect;
- join flow needs no dev console;
- protocol mismatch understandable;
- same club cannot be claimed twice;
- server is authoritative;
- reconnect restores same manager;
- duplicate command idempotent;
- simultaneous mutations remain consistent;
- Ready barrier works;
- shared human-v-human works;
- disconnect/server restart does not corrupt save;
- private LAN/Tailscale instructions are complete.
