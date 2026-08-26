# Single-player acceptance runbook

This runbook is the visual and playable complement to unit and integration
tests. The automated career scenario uses deterministic seed `10_042` in
`ofm_core/tests/scenario_tests.rs`; it is replayed by
`npm run test:career-scenarios`. The desktop new-career flow currently remains
random, so record the save ID and fixture date when reporting a manual failure.

## Fast automated gate

Run `npm run check:release:quick` during development. It validates linting,
tracked assets, the production web build, Rust formatting and clippy, then runs
the deterministic career scenario suite. Run `npm run check:release` before a
release candidate; it adds the full frontend and Rust workspace test suites.
The frontend suite runs in four deterministic shards so an exhausted test
worker cannot prevent the release gate from reporting which segment failed.

## Demo-career flow

1. Create or load a career, select a club, and open the dashboard.
2. Confirm the squad has a legal XI; change a player, tactic and training
   focus, then advance one day.
3. Visit Finances, Scouting and Transfers. Check that a financial warning,
   scouting assignment and transfer action either complete or show a translated,
   actionable error.
4. Use **Skip to Match Day**, complete one live match with a substitution, then
   verify the result appears in Schedule, News and the league table.
5. Save, close the application, reopen the same career, and confirm the date,
   result, lineup and selected tactic persist.

## Visual acceptance

Check Dashboard, Squad, Tactics, Transfers, Training, Finances, Schedule and
the Match Centre at normal desktop width and the narrowest supported width.
For the Match Centre, also follow
[MATCH_2D_VISUAL_QA.md](project-albion/MATCH_2D_VISUAL_QA.md). Record the save,
fixture date, viewport and any console error in the pull request when a visual
change is material.

## Pass criteria

- No unhandled error, blank panel or console error during the demo flow.
- The match result is reflected consistently in all three post-match views.
- Save/load preserves the user-facing career state.
- Thai and English labels remain actionable; no missing-key token is visible.
- `npm run check:release` passes.
