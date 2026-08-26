# Project Albion — 2D Match Visual QA

## Purpose

Use this short, runtime-first check whenever the Match Centre presentation
changes. A green build is not visual acceptance: inspect a running match before
calling the work complete. The renderer remains presentation-only; this process
must never validate or alter match outcomes.

## Run

1. Start the desktop app with `npm run tauri dev` and create or load a career
   with a playable fixture.
2. Open the Match Centre and set the 2D camera to **Full Pitch**, then
   **Follow Ball**, then **Tactical**.
3. Replay a goal, card, injury, substitution, save, and an event close to each
   touchline or goal line. Test both halves if a saved fixture is available.
4. Repeat at the smallest supported Match Centre width and at a normal desktop
   width. Toggle player names, role labels, formation shape, commentary and
   reduced motion.

## Acceptance checklist

- The pitch stays at its 105:68 ratio and no empty canvas edge appears while
  Follow Ball tracks a boundary event.
- All active markers remain inside the pitch; goalkeepers, sent-off, injured
  and booked players remain visually distinct.
- The ball, highlighted participants, event cue and score agree with the
  authoritative event feed. Replaying an event changes no game data.
- Tactical mode makes team shape readable; camera and accessibility preferences
  respond without a console error.
- Reduced motion leaves the latest state legible without a perpetual animation.

## Evidence to record in the pull request

Record the fixture or test scenario, viewport sizes, event types checked and
the commit tested. Attach one desktop screenshot or a short screen recording
when the visual change is material. Report any failure with the event minute,
camera mode and viewport so it is reproducible.
