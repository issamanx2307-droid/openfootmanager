# Project Albion — 2D Enhanced Match Renderer Implementation Prompt

> **File:** `10_2D_ENHANCED_MATCH_RENDERER.md`
> **Purpose:** Mandatory implementation specification for the Project Albion 2D Enhanced live match experience.
> **Status:** Required for v1. This document upgrades the former optional compact zone visualization into a required 2D Enhanced renderer.

---

## 0. Controlling instruction for the coding AI

You are working inside the existing **Project Albion** repository, which is based on OpenFootManager and the Project Albion design pack.

Before changing code:

1. Read `00_MASTER_AI_BUILD_PROMPT.md`.
2. Read `01_PRODUCT_VISION_AND_SCOPE.md` through `09_IMPLEMENTATION_ROADMAP_AND_DEFINITION_OF_DONE.md`.
3. Read this file in full.
4. Audit the current repository and reuse existing abstractions where they are compatible.
5. Start implementation immediately after the audit.
6. Do **not** stop at a plan, prototype, TODO list, mock-only screen, or scaffolding.
7. Do not ask the user architecture questions that are answered by these documents.
8. Make reasonable engineering decisions when minor implementation details are unspecified.
9. Keep the match simulation authoritative on the server/game engine.
10. The 2D renderer is a **presentation layer only**. It must never decide football outcomes.

This feature is complete only when the Definition of Done at the end of this document is satisfied.

---

# 1. Product goal

Implement a polished **2D Enhanced match view** inspired by classic Championship Manager / Football Manager 2D presentations, but designed specifically for Project Albion.

The user must be able to watch a live match and visually understand:

- team shape;
- possession;
- pressing;
- player movement;
- passes;
- carries;
- dribbles;
- crosses;
- shots;
- goalkeeper actions;
- tackles;
- interceptions;
- set pieces;
- substitutions;
- cards;
- injuries;
- score changes;
- tactical changes.

The result should feel significantly better than a text-only match centre while remaining much cheaper and simpler than a 3D engine.

The visualizer must make tactics readable. A high press should look different from a low block. Wide play should look different from narrow possession. Counter-attacks should visibly transition quickly.

---

# 2. Non-negotiable architecture

Use this data flow:

```text
Authoritative Match Engine
        │
        │ semantic match events + state
        ▼
Presentation Adapter / Timeline Compiler
        │
        │ deterministic visual commands
        ▼
2D Match Renderer
        │
        ├─ pitch
        ├─ players
        ├─ ball
        ├─ camera
        ├─ overlays
        ├─ effects
        └─ replay
```

The renderer **must not** feed values back into match simulation.

The following must never affect simulation results:

- actual monitor refresh rate;
- frame timing;
- dropped frames;
- browser/Tauri animation timing;
- camera position;
- zoom;
- renderer interpolation;
- local device performance;
- replay playback speed;
- UI state.

The authoritative simulation remains deterministic from:

```text
match input
+ match seed
+ match-engine version
+ validated tactical commands
```

The 2D presentation may have its own deterministic `presentation_seed`, but it is visual-only.

---

# 3. Technology choice

Project Albion frontend uses React/TypeScript/Tauri.

Preferred rendering stack:

1. **PixiJS 8** for the 2D scene if no equivalent renderer already exists.
2. Otherwise reuse an existing compatible high-performance scene/canvas layer.
3. Use plain Canvas2D only if introducing PixiJS would create a serious incompatibility.

Do not use DOM elements for 22 continuously moving players.

The renderer must run inside the existing desktop client and must not require a separate game engine such as Unity or Godot.

Recommended module boundary:

```text
client/src/features/match2d/
├── Match2DView.tsx
├── renderer/
│   ├── MatchRenderer.ts
│   ├── PitchRenderer.ts
│   ├── PlayerRenderer.ts
│   ├── BallRenderer.ts
│   ├── CameraController.ts
│   ├── OverlayRenderer.ts
│   └── EffectsRenderer.ts
├── presentation/
│   ├── PresentationTimeline.ts
│   ├── EventToAnimationCompiler.ts
│   ├── PositionResolver.ts
│   ├── MovementPlanner.ts
│   └── ReplayController.ts
├── state/
│   ├── match2dStore.ts
│   └── selectors.ts
├── components/
│   ├── MatchHUD.tsx
│   ├── MatchControls.tsx
│   ├── CommentaryPanel.tsx
│   ├── MatchStatsPanel.tsx
│   └── TacticalBreakOverlay.tsx
└── __tests__/
```

Adapt paths to the real repository layout instead of forcing this exact tree.

---

# 4. Presentation data contract

Do not couple the renderer directly to engine-internal structs.

Create a stable presentation DTO layer.

Example:

```ts
export type MatchPresentationEvent =
  | PassPresentationEvent
  | CarryPresentationEvent
  | DribblePresentationEvent
  | CrossPresentationEvent
  | ShotPresentationEvent
  | SavePresentationEvent
  | TacklePresentationEvent
  | InterceptionPresentationEvent
  | FoulPresentationEvent
  | CardPresentationEvent
  | GoalPresentationEvent
  | InjuryPresentationEvent
  | SubstitutionPresentationEvent
  | RestartPresentationEvent
  | TacticalChangePresentationEvent
  | PeriodPresentationEvent;
```

Common fields:

```ts
type BasePresentationEvent = {
  eventId: string;
  matchId: string;
  sequence: number;
  matchSecond: number;
  type: string;
  teamId?: string;
  actorPlayerId?: string;
  targetPlayerId?: string;
  zoneFrom?: PitchZone;
  zoneTo?: PitchZone;
  outcome?: string;
  important: boolean;
};
```

If the engine already emits richer information, preserve it.

If the engine does not contain exact x/y player coordinates, **do not modify football outcomes just to get coordinates**. Generate presentation coordinates in the presentation layer using:

- formation;
- player role;
- current phase;
- possession;
- pitch zone;
- team tactic;
- event actor;
- event target;
- deterministic presentation seed.

---

# 5. Coordinate model

Use normalized logical coordinates independent of pixels.

```text
x: 0.0 → 1.0
y: 0.0 → 1.0
```

The canonical coordinate system is always:

```text
Home attacks left → right in first half.
```

Handle side switching at halftime through a transform, not by mutating stored event meaning.

Create types such as:

```ts
type PitchPoint = {
  x: number;
  y: number;
};
```

Rendering converts normalized coordinates to actual screen coordinates after applying:

- pitch padding;
- aspect ratio;
- camera transform;
- zoom.

Never bake screen pixels into simulation/presentation data.

---

# 6. Pitch design

Render a professional top-down football pitch.

Required:

- full touch lines;
- halfway line;
- centre circle;
- centre spot;
- penalty areas;
- six-yard boxes;
- penalty spots;
- penalty arcs;
- corner arcs;
- goals;
- subtle mowing/grass bands;
- out-of-play margins.

The pitch must resize cleanly with the window.

Aspect ratio should visually resemble a real football pitch rather than a square game board.

Do not depend on copyrighted stadium artwork.

Optional later polish:

- subtle stadium edge;
- crowd ambience layer;
- weather tint;
- night lighting.

Do not delay v1 for optional polish.

---

# 7. Player representation

Each on-pitch player is rendered as a compact 2D marker.

Required visual information:

- team color;
- shirt/marker number;
- optional abbreviated player name;
- selected/highlighted state;
- booking indicator;
- injury indicator;
- substituted-player transition.

Recommended appearance:

```text
filled circular marker
+ contrasting border
+ shirt number
+ optional name label on zoom/highlight
```

Do not use real player photographs as moving sprites.

Support a colorblind-friendly mode by combining:

- color;
- border pattern/shape;
- team direction;
- label contrast.

Goalkeepers must be visually distinguishable from outfield players.

---

# 8. Ball representation

The ball must be clearly visible at normal zoom.

Required states:

- controlled by player;
- passing;
- crossing;
- shot;
- loose ball;
- save/parry;
- out of play;
- restart;
- penalty.

Use curved or linear trajectories as appropriate.

Ball movement must be derived from the presentation timeline, not physics that can produce a different football outcome.

---

# 9. Deterministic visual position resolver

Because the match engine can operate with logical zones rather than exact coordinates, create a deterministic **PositionResolver**.

Inputs:

```text
team formation
player slot
player role/duty
team mentality
team width
defensive line
pressing
phase
possession team
ball zone
current event
presentation seed
match second
```

Output:

```text
target presentation position for each active player
```

Rules:

- goalkeeper stays in plausible goalkeeper space;
- centre-backs remain structurally deeper than midfielders in neutral shape;
- fullbacks move higher when role/tactic demands it;
- winger/inside-forward positioning differs;
- forwards stretch or drop based on role;
- team without ball compresses toward ball side;
- high press increases vertical engagement;
- low block reduces team depth;
- attacking width changes lateral spread;
- transition state allows temporary distortion of shape.

Do not make all players chase the ball.

Avoid random jitter.

Use deterministic micro-offsets so markers do not perfectly overlap.

---

# 10. Movement planner

Create animation segments rather than teleporting players.

Example:

```ts
type MovementSegment = {
  entityId: string;
  from: PitchPoint;
  to: PitchPoint;
  startPresentationMs: number;
  durationMs: number;
  easing: EasingKind;
};
```

Movement planner requirements:

- interpolate between tactical shape points;
- accelerate/decelerate smoothly;
- avoid visually impossible instantaneous jumps;
- preserve event timing;
- support possession transitions;
- support recovery runs;
- support forward runs;
- support goalkeeper movement;
- keep motion readable at 1x, 2x and 4x.

If simulation events are sparse, insert **visual-only movement segments** between semantic events.

These segments may improve visual continuity but must never create new simulated stats or outcomes.

---

# 11. Core event animations

Implement at minimum the following.

## 11.1 Short pass

- actor moves/sets;
- ball travels to target;
- receiving player controls;
- surrounding team shape adjusts.

## 11.2 Progressive/long pass

- longer ball travel;
- receiving target moves into space;
- defenders react visually.

## 11.3 Through ball

- ball enters space ahead of receiver;
- receiver accelerates onto it;
- defensive line turns/recover-runs.

## 11.4 Cross

- wide player delivers toward box;
- attackers and defenders converge;
- trajectory visually differs from ground pass.

## 11.5 Carry

- ball remains with actor;
- actor moves through space;
- nearby defenders adjust.

## 11.6 Dribble

- attacker-defender contest is visually highlighted;
- successful dribble continues;
- failed dribble transfers ball possession.

## 11.7 Tackle

- defender closes;
- short contest cue;
- ball state changes according to engine result.

## 11.8 Interception

- interceptor moves into passing lane;
- ball possession changes.

## 11.9 Shot

- clear shot trajectory;
- visual emphasis;
- goal/save/block outcome follows engine event.

## 11.10 Goalkeeper save

- keeper moves/dive-style translation;
- claim, parry or save state;
- rebound only if engine produces it.

## 11.11 Goal

- ball reaches goal;
- scoreboard updates only at authoritative event boundary;
- short goal effect;
- optional simple celebration movement;
- replay becomes available.

## 11.12 Foul/card

- play stops;
- foul location visible;
- yellow/red card overlay;
- red-carded player removed from formation after authoritative update.

## 11.13 Injury

- injured player indicator;
- play state reflects engine event;
- no graphic injury detail.

## 11.14 Substitution

- outgoing/incoming overlay;
- marker swap at valid stoppage;
- formation re-resolves after substitution.

## 11.15 Corner/free kick/penalty/throw-in/kickoff

Each restart must reset players into plausible deterministic presentation positions before the next action chain.

---

# 12. Match camera

Provide these camera modes:

### A. Full Pitch — default

Entire pitch visible.

Best for tactical understanding.

### B. Dynamic

Zoom/pan toward important action while preserving enough context.

### C. Tactical

Slightly more zoomed out with clearer team-shape labels/lines.

Default v1 can ship with Full Pitch plus Dynamic if Tactical would delay release.

Camera requirements:

- smooth pan;
- smooth zoom;
- no sudden jumps;
- reset control;
- preserve orientation clarity;
- do not hide the ball;
- respect accessibility setting for reduced motion.

During human-v-human online matches, both users may use different local camera modes. Camera state is not synchronized and has no gameplay effect.

---

# 13. Tactical visualization

Provide optional overlays that help the manager understand tactics.

Required toggleable overlays:

- player names;
- role abbreviations;
- formation shape;
- possession indicator.

Recommended:

- defensive line;
- average shape;
- passing network in post-match view;
- heatmap in post-match view.

Do not clutter the default live display.

---

# 14. Match HUD

The 2D renderer sits inside a complete Match Centre.

Required HUD:

```text
Competition
Home club     score     Away club
Match clock / phase
```

Required controls:

- pause where session policy allows;
- 1x;
- 2x;
- 4x;
- next highlight where allowed;
- key / extended / full highlights;
- camera mode;
- zoom;
- commentary toggle;
- stats/tactics panel;
- replay control after supported highlights.

Required human-v-human states:

- opponent connected;
- opponent reconnecting;
- tactical break;
- waiting for opponent;
- halftime ready/not ready.

---

# 15. Commentary synchronization

Commentary text and animation must refer to the same authoritative event sequence.

Example:

```text
37:42 — Saka carries down the right.
37:46 — Saka finds Ødegaard.
37:49 — Ødegaard slips a through ball to the striker.
37:52 — Shot!
37:53 — Saved by the goalkeeper.
```

The visual timeline may interpolate between those events, but it may not reorder semantic events.

Commentary should highlight the current event.

Clicking a recent important commentary entry may replay that highlight when technically practical.

---

# 16. Highlight system

The simulation may emit far more events than the player watches.

Create presentation filters:

### Key
Show:

- goals;
- major chances;
- penalties;
- red cards;
- important saves;
- major tactical moments where available.

### Extended
Show:

- key;
- meaningful attacks;
- set pieces;
- selected defensive sequences.

### Full
Show the full event stream/presentation.

Filtering must not re-simulate the match.

The underlying match is identical across highlight modes.

---

# 17. Presentation timeline

Create a timeline compiler.

Input:

```text
ordered semantic MatchPresentationEvents
```

Output:

```text
ordered PresentationClips
```

Example:

```ts
type PresentationClip = {
  clipId: string;
  sourceEventIds: string[];
  startMatchSecond: number;
  endMatchSecond: number;
  importance: "normal" | "extended" | "key";
  commands: RenderCommand[];
};
```

Possible RenderCommands:

```text
MovePlayer
MoveBall
SetPossession
ShowShotCue
ShowSaveCue
ShowGoalCue
ShowCard
ShowInjury
ShowSubstitution
SetCameraFocus
UpdateScore
UpdateClock
ResetShape
```

The same semantic event set + presentation version + presentation seed must compile into equivalent presentation commands.

Persist:

```text
presentation_version
```

for compatibility/debugging.

---

# 18. Replay system

Implement goal replay at minimum.

Recommended v1:

- goals;
- penalties;
- major chances.

Replay must use stored presentation commands or recompile from stored semantic events.

Replay must never invoke the match engine again.

Controls:

- replay;
- close;
- 0.5x or 1x replay speed if simple to implement.

After replay, return to the current live state safely.

For online matches, replay may be local only after the authoritative match has moved beyond the highlight. It must not pause the other user unless the shared session policy explicitly pauses the match.

---

# 19. Human-v-human multiplayer

This requirement is mandatory.

Authoritative server owns:

- match state;
- match event sequence;
- accepted tactical commands;
- effective command time;
- score;
- substitutions;
- cards;
- injuries;
- current simulation phase.

Clients receive ordered events/state updates.

Renderer behavior:

```text
server event stream
      ↓
client presentation adapter
      ↓
local deterministic animation
```

Clients may render at different frame rates without desynchronizing football state.

Use:

- monotonically increasing server sequence;
- match revision/state version;
- reconnect snapshot;
- event replay from last acknowledged sequence if supported.

On reconnect:

1. stop unsafe local playback;
2. request/receive authoritative current state;
3. rebuild active players, score, clock and tactical state;
4. resume presentation from safe event boundary;
5. never guess missing football events.

Both clients must display the same:

- score;
- match clock state;
- semantic event order;
- substitutions;
- cards;
- final result.

Visual interpolation timing may vary slightly due to local rendering.

---

# 20. Tactical command integration

During live matches the human can change:

- formation;
- player role/duty;
- team mentality/instructions;
- substitutions;
- set-piece takers where supported.

The renderer sends nothing directly to the simulation.

Flow:

```text
UI command
   ↓
existing validated multiplayer/game command path
   ↓
authoritative server accepts/rejects
   ↓
engine applies at deterministic safe boundary
   ↓
authoritative event/state update
   ↓
renderer re-resolves team shape
```

Show a clear UI state for:

- pending;
- accepted;
- rejected;
- applied.

Do not visually apply a tactical change as if real until authoritative confirmation is received.

---

# 21. Match speed

Support:

```text
1x
2x
4x
```

Speed modifies presentation time, not simulated outcomes.

At faster speed:

- preserve critical-event readability;
- reduce nonessential animation delay;
- avoid skipping score/card/substitution updates.

For human-v-human:

- obey the session's shared simulation speed policy;
- local renderer must not silently advance simulation.

---

# 22. Performance budget

Target typical desktop hardware.

Desired:

- 60 FPS rendering on common modern Windows PCs;
- graceful operation at 30 FPS without gameplay effect;
- low CPU overhead relative to match simulation;
- no memory growth across repeated matches.

Avoid:

- React re-rendering at every animation frame;
- creating/destroying large numbers of objects each frame;
- thousands of DOM nodes;
- unnecessary texture allocations;
- per-frame database/network work.

Use renderer-owned state for high-frequency animation.

React owns surrounding UI and coarse match state.

---

# 23. Resize and display support

Required:

- windowed desktop;
- maximized;
- common 16:9 and 16:10 resolutions;
- usable at 1366×768;
- crisp on high-DPI screens.

The pitch must preserve its aspect ratio.

When space is limited:

- prioritize pitch;
- collapse secondary panels;
- preserve score/time;
- preserve tactical controls.

---

# 24. Audio

Audio is optional for the first implementation.

If implemented without delaying required work, support simple original/non-infringing effects:

- whistle;
- kick;
- crowd rise;
- goal reaction;
- card whistle.

Audio must be individually muteable.

Do not require audio for Definition of Done.

---

# 25. Accessibility

Required:

- reduced-motion mode;
- colorblind-safe team differentiation;
- readable contrast;
- scalable labels;
- keyboard-accessible match controls where practical;
- no information conveyed only by color.

Reduced-motion mode:

- simplify camera movement;
- shorten easing;
- avoid celebration zoom;
- retain all football information.

---

# 26. Settings

Add persisted match-view settings:

```text
match_view_mode
camera_mode
match_speed_default
highlight_mode
show_player_names
show_role_labels
show_commentary
reduced_motion
team_marker_style
```

These are local user preferences and must not alter match results.

---

# 27. Versioning

Version the presentation layer separately from the match engine.

Example:

```text
match_engine_version = "albion-engine-1"
presentation_version = "albion-2d-1"
```

A saved completed match should contain enough data to support:

- result/history;
- semantic event review;
- compatible replay where supported.

If an old presentation version is unavailable, degrade gracefully to commentary/stat review instead of corrupting save data.

---

# 28. Testing requirements

Do not treat renderer tests as optional.

## 28.1 Unit tests

Test:

- coordinate transforms;
- halftime side swap;
- zone-to-position mapping;
- formation slot placement;
- deterministic presentation generation;
- timeline ordering;
- speed conversion;
- highlight filtering;
- replay compilation.

## 28.2 Property/invariant tests

Required invariants:

- all active players remain within allowed visual bounds except explicit exit/substitution transition;
- exactly one active ball representation exists;
- event sequence never reverses;
- score displayed equals authoritative score;
- red-carded player is not shown as active after application;
- substituted-out player is not active after replacement;
- halftime transform preserves team identity;
- no presentation command changes engine state.

## 28.3 Integration tests

Cover:

- full match from kickoff to final whistle;
- substitutions;
- yellow/red cards;
- injury;
- penalty;
- goal replay;
- extra time;
- penalty shootout if supported by competition engine;
- tactical changes;
- reconnect during live match.

## 28.4 Multiplayer integration test

Run host + guest clients against one match server.

Verify both receive:

- identical ordered semantic event IDs;
- identical final score;
- identical cards/substitutions;
- compatible presentation timeline.

Force disconnect/reconnect of guest and verify successful state rebuild.

## 28.5 Long-run stability

Automate repeated rendered or headless-presentation compilation for many matches.

Check:

- memory;
- uncaught errors;
- sequence corruption;
- replay corruption.

---

# 29. Visual QA checklist

Manually verify at minimum:

- 4-4-2;
- 4-3-3;
- 4-2-3-1;
- narrow formation;
- wide formation;
- high press;
- low block;
- fast counter;
- red-card shape;
- 10v11;
- late substitution;
- goalkeeper save;
- corner;
- penalty;
- off-the-ball movement;
- halftime side swap;
- 1x/2x/4x;
- 1366×768;
- maximized 1080p+;
- host vs guest online.

The renderer should never look like 20 outfield markers collapsing into one ball location.

---

# 30. Error handling

If a presentation event is unknown:

- log structured diagnostic information;
- preserve event order;
- show safe generic movement/commentary fallback;
- do not crash the match.

If renderer initialization fails:

- automatically fall back to commentary/stat match centre;
- keep the match playable;
- show a recoverable UI error.

Renderer failure must never corrupt the save.

---

# 31. Telemetry/debug tools for development

Add developer-only diagnostics:

- current semantic event;
- server sequence;
- presentation clip ID;
- match second;
- presentation position coordinates;
- active player IDs;
- FPS;
- frame time;
- renderer entity count;
- presentation version.

Add optional visual debug overlay:

- pitch zones;
- target positions;
- role anchors;
- movement paths.

Disable debug overlay in normal user mode.

---

# 32. Implementation sequence

Execute in this order unless repository realities require a small adjustment.

## Phase A — contract and renderer shell

1. Audit existing match event structs.
2. Define presentation DTOs.
3. Create presentation adapter.
4. Create full pitch renderer.
5. Render 22 players + ball from static state.
6. Add resize handling.

**Exit condition:** a valid match state displays correctly on the 2D pitch.

## Phase B — movement and event timeline

1. Implement normalized coordinates.
2. Implement formation anchors.
3. Implement PositionResolver.
4. Implement MovementPlanner.
5. Implement PresentationTimeline.
6. Animate pass/carry/shot/basic possession changes.

**Exit condition:** an event sequence plays smoothly without teleports in normal play.

## Phase C — complete football event coverage

Implement:

- dribble;
- tackle;
- interception;
- cross;
- save;
- goal;
- foul;
- card;
- injury;
- substitutions;
- restarts;
- halftime side change.

**Exit condition:** standard 90-minute matches do not require placeholder animations for normal core events.

## Phase D — Match Centre UX

Implement:

- score/clock HUD;
- commentary;
- speed;
- highlight mode;
- camera;
- zoom;
- stats/tactics access;
- tactical-break states.

**Exit condition:** the user can manage a full match without leaving the live match experience.

## Phase E — replay

1. Store/reuse presentation clips.
2. Implement goal replay.
3. Ensure replay never re-simulates.
4. Return safely to live state.

**Exit condition:** every goal can be replayed in a completed/local-safe context.

## Phase F — multiplayer

1. Bind presentation to authoritative ordered server events.
2. Implement reconnect rebuild.
3. Validate tactical command confirmation.
4. Validate H2H match consistency.
5. Test different local frame rates.

**Exit condition:** two online users watch and manage the same match without football-state divergence.

## Phase G — polish/performance/accessibility

1. profile;
2. remove frame allocation hotspots;
3. add reduced motion;
4. add colorblind-safe markers;
5. complete responsive behavior;
6. complete debug tools;
7. complete QA.

**Exit condition:** satisfies all Definition of Done items.

---

# 33. Required code-quality rules

- TypeScript strict mode where project configuration permits.
- Do not use `any` as a shortcut for presentation contracts.
- Centralize renderer constants.
- Centralize timing/easing configuration.
- Centralize tactical shape coefficients.
- Do not scatter magic numbers.
- Keep rendering concerns out of match-engine crate.
- Keep networking concerns out of renderer.
- Keep simulation RNG out of renderer.
- Clean up renderer resources on unmount.
- Add comments for non-obvious transforms/algorithms, not trivial code.
- Preserve existing repository formatting/lint conventions.
- Add tests with each major subsystem rather than after everything is built.

---

# 34. Suggested configuration

Create a versioned renderer configuration.

Example:

```ts
export const MATCH_2D_CONFIG = {
  baseWidth: 105,
  baseHeight: 68,

  playerMarkerRadius: 0.012,
  ballRadius: 0.005,

  speeds: {
    normal: 1,
    fast: 2,
    veryFast: 4,
  },

  animation: {
    shortPassMs: 420,
    longPassMs: 650,
    shotMs: 380,
    tackleMs: 280,
    shapeTransitionMs: 700,
  },

  camera: {
    minZoom: 1,
    maxZoom: 2.2,
  },
} as const;
```

Treat values as tunable defaults, not sacred constants.

---

# 35. Required user experience

A normal match should feel like this:

```text
Pre-match
   ↓
Kickoff
   ↓
2D live play
   ↓
Key attack develops
   ↓
Camera follows
   ↓
Pass / movement / shot
   ↓
Goal or save
   ↓
Commentary + stats update
   ↓
Manager can change tactics/substitute
   ↓
Authoritative confirmation
   ↓
Team shape visibly changes
   ↓
Halftime
   ↓
Both human managers Ready in H2H
   ↓
Second half
   ↓
Final whistle
   ↓
Post-match stats / timeline / replay
```

The user should not need to read commentary to understand every basic action.

The user should be able to see **why** a tactic appears to be succeeding or failing.

---

# 36. Definition of Done

The feature is **not complete** until all items below are true.

## Renderer

- [ ] Full football pitch renders correctly.
- [ ] 22 active players and ball render correctly.
- [ ] Goalkeepers are visually distinct.
- [ ] Player markers include useful identification.
- [ ] Resize/high-DPI behavior is stable.
- [ ] Animation is smooth on target desktop hardware.

## Football visualization

- [ ] Passes animate.
- [ ] Long/through passes animate.
- [ ] Carries animate.
- [ ] Dribbles animate.
- [ ] Crosses animate.
- [ ] Shots animate.
- [ ] Saves animate.
- [ ] Tackles animate.
- [ ] Interceptions animate.
- [ ] Goals animate.
- [ ] Fouls/cards display.
- [ ] Injuries display.
- [ ] Substitutions display.
- [ ] Set-piece restarts display.
- [ ] Halftime side swap works.

## Tactical readability

- [ ] Formation affects visual shape.
- [ ] Width affects visual shape.
- [ ] defensive line affects visual shape.
- [ ] pressing visibly affects engagement.
- [ ] possession/non-possession shape differs.
- [ ] role anchors produce reasonable differences.
- [ ] red card changes team shape.

## Match Centre

- [ ] Score and clock are authoritative.
- [ ] Commentary is synchronized.
- [ ] 1x/2x/4x controls work.
- [ ] Key/Extended/Full highlight modes work.
- [ ] Camera controls work.
- [ ] Match tactical controls remain usable.
- [ ] Tactical changes wait for authoritative acceptance.

## Replay

- [ ] Goal replay works.
- [ ] Replay does not call match simulation.
- [ ] Replay returns safely to current/final state.

## Multiplayer

- [ ] Host and guest receive same semantic event order.
- [ ] Host and guest have identical football outcome.
- [ ] Reconnect rebuilds current match state safely.
- [ ] Client FPS differences cannot change match result.
- [ ] Client camera/zoom differences cannot change match result.
- [ ] H2H halftime/ready integration works.

## Reliability

- [ ] Renderer failure falls back without corrupting match/save.
- [ ] Unit tests pass.
- [ ] Integration tests pass.
- [ ] Multiplayer live-match test passes.
- [ ] No known memory leak across repeated matches.
- [ ] No critical console/runtime errors in a full 90-minute match.

## Accessibility

- [ ] Reduced-motion mode works.
- [ ] Team identification does not rely only on color.
- [ ] HUD is readable at common resolutions.

---

# 37. Final acceptance scenario

Before declaring this work complete, perform this exact end-to-end scenario:

1. Start Project Albion server.
2. Host creates/loads a two-human career.
3. Guest connects.
4. Both users reach a scheduled head-to-head fixture.
5. Open Match Centre.
6. Both users see the same starting XI, score and clock.
7. Kick off.
8. Observe at least:
   - pass;
   - carry;
   - tackle/interception;
   - shot;
   - goalkeeper action.
9. Host changes team instruction.
10. Guest performs a substitution.
11. Verify both changes are accepted through authoritative command flow.
12. Verify the visible team shapes update after server confirmation.
13. Trigger or use a test fixture containing a goal.
14. Verify score updates correctly on both clients.
15. Verify goal replay.
16. Disconnect guest during second half.
17. Reconnect guest.
18. Verify score, clock, active players, cards/substitutions and event state rebuild correctly.
19. Complete the match.
20. Verify identical final result and match record on server/host/guest.
21. Open post-match view and confirm timeline/stats/replay.
22. Run automated renderer, integration and multiplayer tests.
23. Fix all failures.
24. Update `IMPLEMENTATION_STATUS.md`.

Do not mark Project Albion 2D Enhanced complete unless this scenario succeeds.

---

# 38. Final instruction to the coding AI

Implement this feature completely.

Do not return only design commentary.

Do not stop at a static pitch.

Do not stop with moving dots that are disconnected from the real match event stream.

Do not create a second simulation inside the renderer.

Do not weaken deterministic multiplayer architecture to make animation easier.

Prefer a robust, readable 2D football presentation over unnecessary graphical effects.

When a minor visual detail is ambiguous, choose the solution that best supports:

1. tactical readability;
2. deterministic authoritative simulation;
3. smooth online two-player play;
4. performance;
5. maintainability.

After implementation, run the required tests and complete the final acceptance scenario before declaring the task finished.
