# Project Albion — Product Vision and Scope

## Product statement

Project Albion is an original desktop football-management simulation for two friends sharing one private online career focused on English club football.

It uses real club/player identity and roster facts to establish the starting world, then deliberately diverges into its own history. The game is about tactical judgment, squad building, recruitment, player development, finances and long-term stories rather than reflexes.

Do not copy another manager game's proprietary text, database ratings, assets or UI trade dress.

## Product pillars

### One shared world
Every transfer, injury, fixture, contract, youth player, league table and trophy exists once on the authoritative server.

### Believable causality
Outcomes should usually be understandable through:
- player quality and fit;
- tactics;
- fatigue/fitness;
- injuries;
- morale/form;
- squad depth;
- transfer/financial decisions;
- opponent decisions;
- controlled randomness.

Randomness creates uncertainty, not arbitrary scripting.

### Long-term stories
The simulation should naturally create:
- title races;
- promotions/relegations;
- playoff drama;
- cup runs;
- aging stars;
- breakout youth;
- failed/successful transfers;
- managerial changes;
- financial cycles;
- rivalries between the two human clubs.

### Fast workflow
Depth must not mean busywork. Support:
- global search;
- dense sortable tables;
- shortlists;
- player compare;
- filters;
- assistant auto-selection;
- one-click Ready;
- automatic routine simulation.

### Private-first ownership
No cloud account is required. The host owns the career save and can back it up. Online play is a private connection to the host/server.

## Users

v1 has exactly two human managers:
- Host Manager
- Guest Manager

They can control any two different clubs in the four fully playable English divisions.

All other clubs are AI controlled.

## Fully playable English world

| Division | Simulation depth |
|---|---|
| Premier League | Full |
| Championship | Full |
| League One | Full |
| League Two | Full |

Required feeder/background support:
- National League and lower English clubs sufficient to produce relegation/promotion candidates and cup entrants.
- Foreign clubs/leagues sufficient for transfers, loans, scouting and continental opponents.

## Required competitions

Domestic:
- four leagues above;
- FA Cup;
- EFL Cup;
- Community Shield when applicable.

Continental for qualifying English clubs:
- Champions League-style;
- Europa League-style;
- Conference League-style.

Competition rules are season-versioned configuration. Never assume current real rules will remain valid forever.

## Canonical new-career mode: Season Reset

This is a deliberate alternate-timeline mode.

Example:

```text
Real-world snapshot freeze: after transfer window closes
Club/player rosters: real as of freeze
League points: 0
Played matches: 0
Real results: ignored
Season player stats: 0
Fixtures: generated/imported but all unplayed
Start: opening state
```

Question answered by the game:
> What if the squads at the end of the transfer window had started the season from day one?

This behavior is required even if the snapshot freeze occurs after real matches have already been played.

Existing careers never receive later real transfer updates.

## Player identity data

Potential real-world factual fields:
- full name / known name;
- date of birth;
- nationality;
- football nation;
- height;
- preferred foot if known;
- positions;
- current club;
- shirt number if known;
- contract dates if known;
- loan relationship if known.

Unknown facts stay unknown.

## Project Albion player attributes

Use a game-owned 1–100 system.

Technical:
- first touch
- passing
- technique
- dribbling
- finishing
- crossing
- long shots
- heading
- tackling
- marking
- set pieces

Mental:
- anticipation
- decisions
- vision
- composure
- positioning
- off-ball
- teamwork
- work rate
- aggression
- bravery
- leadership
- concentration

Physical:
- acceleration
- pace
- agility
- balance
- strength
- stamina
- jumping
- natural fitness

Goalkeeping:
- reflexes
- handling
- aerial command
- one-on-one
- positioning
- distribution
- kicking
- throwing

Hidden/game-only:
- consistency
- professionalism
- ambition
- adaptability
- injury proneness
- pressure handling
- loyalty
- big-match tendency

Every generated attribute set records its rating-model version.

## Tactical scope

At minimum support data-driven shapes:
- 4-4-2
- 4-2-3-1
- 4-3-3
- 4-1-4-1
- 4-3-2-1
- 3-4-2-1
- 3-4-3
- 3-5-2
- 5-3-2

Team instructions:
- mentality
- tempo
- width
- passing directness
- build-up preference
- pressing intensity
- defensive line
- line of engagement
- counterpress
- counterattack
- time wasting
- crossing approach
- overlap/underlap tendency

Player roles modify behavior, not simply a hidden overall bonus.

## Squad and registration

Required:
- first team / development / youth designations;
- starting XI and bench;
- injuries and suspensions;
- competition eligibility;
- squad registration;
- homegrown/age exemptions through ruleset;
- auto-select legal lineup;
- squad-role expectations.

UI must explain ineligibility.

## Transfers

Required stateful workflow:
- listing;
- asking price;
- bid;
- counteroffer;
- accept/reject;
- installments;
- basic add-ons;
- sell-on framework;
- loans;
- wage contribution;
- contract negotiation;
- medical/registration hook;
- completion;
- deadline/expiry;
- competing AI bids.

Transfer value is dynamic and depends on ability, age, potential, reputation, contract length, wage, demand and club context.

## Contracts

Required:
- start/end;
- weekly wage;
- squad status;
- signing fee baseline;
- appearance/goal/clean-sheet bonuses baseline;
- release-clause framework;
- extension option framework;
- negotiation rounds/cooldown;
- expiry/free agency.

Player acceptance considers money, club level, playing time, ambition, relationship and competing interest.

## Finances

Track:
- cash/balance;
- transfer budget;
- wage budget;
- wage commitments;
- transfer installments;
- matchday income abstraction;
- broadcast/commercial/sponsorship abstraction;
- prize money;
- staff/facility/youth costs;
- projected result.

Every money change goes through a ledger.

English financial regulations are versioned rules, not hardcoded forever.

## Scouting and uncertainty

A human manager should not know every hidden attribute of every player.

Per-manager scouting knowledge includes:
- confidence level;
- observed ability range;
- position/role suitability;
- strengths/weaknesses;
- estimated fee/wage;
- personality hints;
- last observation.

Knowledge improves with time, scout quality, visibility and familiarity.

The two human managers can have different scouting knowledge.

## Training and development

Required:
- weekly intensity;
- team focus;
- individual focus;
- recovery around matches;
- age curves;
- minutes played;
- coaching quality;
- professionalism;
- potential ceiling;
- position familiarity.

Growth must be gradual and uncertain.

Older players can lose physical attributes while retaining/improving mental/technical strengths.

## Fitness and injury

Track:
- condition;
- fatigue;
- match sharpness;
- injury;
- return estimate range;
- recurrence risk.

Risk depends on workload, fatigue, contact, injury proneness and recent recovery.

## AI manager standard

AI must:
- field legal teams;
- rotate for congestion;
- respond to injuries/cards;
- use a tactical identity;
- adjust to match state;
- identify squad weaknesses;
- buy/sell/loan rationally;
- renew important contracts;
- keep finances within real game rules;
- avoid obviously broken squad composition.

AI uses the same football/economic rules as humans; no hidden money or attribute bonuses.

## Multiplayer experience

Lobby:
1. host creates/loads career;
2. server starts;
3. host claims club;
4. guest joins by private address + join code;
5. guest claims a different club;
6. compatibility is checked;
7. both enter shared world.

Game time:
- either player can browse/manage concurrently;
- each marks Ready;
- server advances only when the barrier is satisfied;
- advancement stops for meaningful human blockers.

Human-v-human:
- one live match state;
- two independent tactical controls;
- synchronized score/event feed;
- server resolves all actions.

Separate human-v-AI fixtures:
- can run concurrently as two live sessions;
- shared world waits until both resolve.

## Save philosophy

Career save is valuable user data.

Required:
- one relational SQLite DB per career;
- migrations;
- atomic transactions;
- autosave;
- rotating backups;
- integrity check;
- safe restore;
- persistent historical records.

History should allow past lookup of:
- league tables;
- champions;
- cups;
- player career stats;
- transfers;
- managers;
- club achievements.

## UI philosophy

Desktop-first:
- information dense;
- quick navigation;
- sortable/filterable tables;
- clear budget/fitness/eligibility consequences;
- original visual identity;
- keyboard usable;
- accessible;
- English + Thai.

## Simulation levels

Level A — full:
- human clubs and fully playable English divisions;
- detailed match engine and statistics.

Level B — background:
- English feeders and major foreign leagues/clubs;
- simplified match/finance cycle;
- full engine when human-relevant.

Level C — distant:
- coarse seasonal strength/player/transfer lifecycle.

The system may temporarily promote a background match to full engine when needed.

## Explicit non-goals for v1

Do not delay v1 for:
- 3D match rendering;
- mobile/console;
- public matchmaking;
- cloud accounts;
- spectator servers;
- voice chat;
- user content marketplace;
- national team management;
- women's competitions;
- historical eras;
- managing below League Two;
- required generative AI;
- full human job-hopping system.

Architecture should not prevent later additions.

## Success criteria

Technical:
- deterministic replay for same match input/seed/version;
- reconnect converges to canonical revision;
- no duplicate player identity from snapshot imports;
- no money duplication from command retry;
- 10-season soak completes with restarts;
- snapshot update never changes active save.

Gameplay:
- stronger teams have an advantage but upsets remain possible;
- AI maintains legal squads;
- league/cup/promotion structures remain valid;
- transfer economy does not collapse;
- youth replenishes long careers;
- no one tactic dominates all contexts.

Usability:
- two non-developer users can reach and play first fixture;
- Thai UI remains readable;
- disconnect shows recoverable status;
- ordinary management actions do not require console or DB editing.
