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

## Phase status

| Phase | Title | Status |
|---|---|---|
| 0 | Baseline audit | DONE |
| 1 | Versions, protocol, rules foundation | IN PROGRESS |
| 2 | Relational per-career save | IN PROGRESS |
| 3 | Snapshot/data pipeline | NOT STARTED |
| 4 | Competition/calendar | NOT STARTED |
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

Next: finish Phase 1 by carrying the Albion rules-pack selection into career
creation rather than relying on only the domain defaults. Phase 2 has begun
with the append-only `career_versions` migration; continue the repository API
that persists and reads the pinned version contract.
