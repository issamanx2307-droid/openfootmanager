# Project Albion — Match Engine and Balance Specification

## Goals

The match engine must be:
- deterministic for identical input + seed + engine version;
- fast enough for whole-world simulation;
- interactive for live tactical changes;
- statistically plausible;
- explainable through events;
- independent of UI/network/database;
- original Project Albion logic.

It is not a full physics simulator and does not need 3D rendering.

## Migration from upstream

OpenFootManager already isolates match simulation and supports instant/live paths. Preserve this boundary and evolve behind a stable interface rather than deleting everything at once.

Target contracts:

```rust
pub trait MatchSimulator {
    fn simulate(&self, input: MatchInput, seed: MatchSeed) -> Result<MatchReport>;
}

pub trait LiveMatchSimulator {
    fn start(&self, input: MatchInput, seed: MatchSeed) -> Result<LiveMatchState>;
    fn step(
        &self,
        state: &mut LiveMatchState,
        commands: &[MatchCommand]
    ) -> Result<Vec<MatchEvent>>;
}
```

Persist `match_engine_version`.

## Determinism

All match influences must be explicit:
- lineup;
- player attributes;
- fitness/fatigue;
- formation/roles/tactics;
- morale/context if used;
- home advantage;
- competition rules;
- seed;
- engine config version.

Forbidden:
- OS random source during simulation;
- wall-clock time;
- nondeterministic collection iteration affecting choices;
- client arrival time used directly as a football roll.

Completed match stores:
- engine version;
- seed;
- input hash;
- report hash.

Debug replay of stored input/seed/version must reproduce exact report.

## Simulation model

Use a hybrid possession/event chain.

### Match clock
Represent internal time in seconds.

Do not necessarily tick each second. After an action, advance by a context-dependent duration (for example 2–12 simulated seconds). This keeps chains believable and fast.

### Logical zones

Recommended:
- defensive box;
- defensive left/centre/right;
- midfield left/centre/right;
- attacking left/centre/right;
- attacking box.

Exact enum may be 9–11 zones.

Ball state:
- possession team;
- actor;
- zone;
- pressure;
- phase: open play / transition / set piece.

### Actions

Core actions:
- short pass;
- progressive pass;
- long pass;
- through ball;
- cross;
- carry;
- dribble;
- recycle;
- tackle;
- interception;
- clearance;
- shot;
- header;
- save/parry/claim;
- foul;
- throw-in;
- corner;
- free kick;
- penalty;
- kickoff.

Selection depends on:
- role;
- team tactic;
- zone;
- score/time;
- player attributes;
- pressure/support.

## Action resolution

Typical chain:

```text
choose actor
→ choose action
→ choose target/context
→ calculate contest
→ resolve success/failure
→ move zone/change possession/create restart
→ update player/team stats
→ update fatigue
→ advance clock
```

Do not resolve full match from one team-strength score.

## Attribute mapping

Passing factors:
- passing;
- technique;
- vision;
- decisions;
- composure;
- pressure;
- distance;
- receiver movement.

Dribble:
- dribbling;
- technique;
- agility;
- acceleration;
- defender tackling/positioning;
- pressure.

Shot:
- finishing;
- technique;
- composure;
- location/angle;
- defensive pressure;
- goalkeeper positioning/reflexes;
- preferred-foot/body context if modeled.

Aerial:
- heading;
- jumping;
- strength;
- positioning.

Defense:
- tackling;
- marking;
- positioning;
- anticipation;
- concentration;
- aggression as risk modifier.

Centralize weights/config. Never scatter magic numbers through files.

## Tactical effects

### Mentality
Changes risk, forward support and defensive exposure.

### Tempo
Changes action frequency/risk and fatigue.

### Width
Changes zone/movement preference.

### Directness
Changes passing distance and transition speed.

### Pressing
Changes contest frequency and fatigue.

### Defensive line
Trades compactness against space behind.

### Counterpress
Changes immediate behavior after losing ball.

### Counterattack
Changes transition behavior after winning ball.

Tactics change behavior/probabilities; avoid simple +rating multipliers.

## Player roles

Implement behavior profiles.

Examples:
- winger: wide occupation, carry/cross;
- inside forward: wide start, half-space/shot tendency;
- deep playmaker: receives, progressive passing;
- ball winner: more defensive contests with positional risk;
- target forward: aerial receptions/layoffs;
- pressing forward: aggressive press;
- overlapping fullback: forward runs/cross;
- inverted fullback: central build-up movement.

Start with a coherent role set; keep data-driven extensibility.

## Fitness

Effective ability degrades gradually with fatigue/condition.

Avoid cliff effects.

Fatigue depends on:
- minutes;
- tempo;
- pressing;
- role activity;
- stamina/natural fitness;
- starting condition.

Substitutes enter with actual pre-match condition.

## Match-state behavior

The engine/AI can respond to:
- leading/trailing;
- time remaining;
- red card;
- injury;
- tactical instruction.

No scripted comeback bonus.

## Home advantage

Configurable and modest.
Calibrate statistically.
Never guarantee outcomes.

## Set pieces

v1:
- corners;
- direct/indirect free-kick abstraction;
- penalties;
- throw-in restart;
- taker priorities.

Detailed routines are later enhancement.

## Fouls/cards

Foul chance depends on:
- contest;
- aggression;
- tackling;
- pressure;
- zone;
- referee profile only if modeled.

Card logic:
- severity;
- previous card;
- rules/config.

Red card changes available players immediately.

## Injuries

Engine emits an injury event/severity signal.
Game medical system converts it into persistent injury type/duration.

## Goalkeeper model

Shot pipeline:
1. attempt;
2. block?
3. on target?
4. goalkeeper action?
5. goal/save/parry?

Keeper attributes:
- positioning;
- reflexes;
- handling;
- aerial command;
- one-on-one;
- distribution.

## Statistical invariants

After every match:
- score equals goal events;
- player goals reconcile with team goals, allowing explicit own-goal handling;
- shots on target <= shots;
- completed passes <= attempted;
- possession totals ~100%;
- minutes align with substitutions;
- no player has minutes before entry;
- cards reconcile with events;
- lineup remains legal.

Assert in tests.

## Live simulation

Phases:
- PreKickOff;
- FirstHalf;
- HalfTime;
- SecondHalf;
- ExtraTime where allowed;
- PenaltyShootout where allowed;
- Finished.

Live commands:
- substitute;
- formation;
- role/duty;
- team instruction;
- set-piece taker;
- halftime readiness.

Validate phase and rule limits.

## Human-v-human command timing

Server applies tactical commands at deterministic safe boundaries between action resolution.

Accepted command records:
- server sequence;
- effective match second.

If both commands land in one boundary, deterministic server ordering applies. Client clock does not decide football outcomes.

## Match speed

UI modes:
- 1x
- 2x
- 4x
- next highlight when allowed.

Human-v-human:
- shared speed policy;
- halftime waits for both Ready;
- pause policy is session-level.

Separate human-v-AI matches may present different display speeds while shared world waits for both matches to complete.

## Highlights

Internal simulation can generate more events than UI displays.

Highlight filters:
- key;
- extended;
- full text.

Filter never changes simulation.

## Match AI

Substitution/tactical decision inputs:
- injury;
- condition;
- cards;
- score;
- time;
- performance;
- tactical need.

Respect competition substitution count/windows.

## Calibration tool

Create headless runner, conceptually:

```bash
cargo run -p engine --bin calibrate --   --matches 100000   --profile data/calibration/premier_reference.json
```

Collect:
- goals/match;
- home/draw/away;
- scorelines;
- shots;
- shots on target;
- pass accuracy;
- possession;
- cards;
- penalties;
- injuries;
- stronger-team win rate;
- player-stat distributions.

Reference profile comes from operator-approved statistics and is versioned.

## Balance philosophy

Use broad statistical ranges, not exact imitation of one season.

Required behaviors:
- equal teams roughly symmetric absent home edge;
- stronger teams win more often;
- upsets remain possible;
- strong pressing has fatigue cost;
- ultra-defensive lowers both concession and attacking output;
- all-out attack increases chance creation and vulnerability;
- no single attribute dominates;
- no single tactic dominates all matchups.

## Synthetic strength tests

Create elite/strong/average/weak teams.

Large-N expectations:
- elite beats weak clearly more than 50%;
- equal pairings close to symmetric after home edge removal;
- swapping home/away shifts results modestly;
- tactical extremes expose trade-offs.

## Performance

Benchmark:
- instant match;
- live step;
- 100-match batch;
- report serialization.

Keep DB/network out of hot path.
Optimize only after correctness/profiling.

## Match report

Must include:
- score;
- extra time/penalties;
- goals/assists;
- cards;
- injuries;
- substitutions;
- event timeline;
- team stats;
- player stats;
- player ratings;
- tactical-change summary.

Player rating derives from contribution and role expectations, not decorative randomness.

## Storage policy

Full details:
- human matches;
- fully simulated English matches if storage permits;
- important cup/continental matches.

Background distant matches:
- summary can suffice.

Never discard trophy/table/player career history.

## Engine version policy

Historical results never change.

For future matches after app update:
- either pin career to current engine version;
- or migrate at documented boundary, preferably season rollover.

Store engine version per match.

## Definition of Done

- deterministic replay exact;
- instant/live no-intervention paths reconcile;
- live tactics/substitutions affect subsequent play;
- human-v-human uses one server state;
- 100k calibration runner works;
- stat invariants pass;
- anti-exploit tests pass;
- performance reasonable;
- 10-season soak contains plausible broad distributions and no engine crash.
