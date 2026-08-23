# Project Albion — Gameplay Systems Specification

## Philosophy

Every system must operate through explicit state, services and domain events. Avoid hidden cross-table mutation. Human and AI managers obey the same core rules.

## Calendar and advancement

One authoritative career date/time exists on the server.

Advance only when:
- both managers are Ready; or
- an explicitly enabled force-advance policy applies.

Before advancing, detect human blockers:
- invalid lineup/registration;
- required deadline response;
- human fixture;
- other mandatory action.

Day processing order must be deterministic and tested:

```text
deadlines
→ scheduled payments
→ recovery
→ training
→ medical
→ scouting
→ morale/form
→ AI planning
→ registration/window logic
→ fixtures/matches
→ discipline/standings
→ finances
→ inbox/news
→ autosave
→ blocker detection
```

## Competition engine

Generic types:
- league;
- knockout cup;
- league/group stage + knockout;
- playoff.

Ruleset controls:
- participants;
- rounds;
- points;
- tie breakers;
- promotion/relegation;
- playoffs;
- substitution/registration;
- cup extra time/penalties;
- prize money;
- qualification.

Season rollover must be idempotent:
1. final tables;
2. winners/history;
3. prize money;
4. qualification;
5. promotion/relegation;
6. next memberships;
7. reputation update;
8. season-stat reset;
9. contract/offseason actions;
10. schedule;
11. youth intake/preseason.

Crash retry must never double-promote or double-pay.

## Squad

Player state:
- first team;
- development;
- youth;
- loaned out;
- loaned in;
- transfer listed;
- unavailable.

Starting XI validation:
- 11 distinct players;
- eligible;
- not suspended/injured unavailable;
- goalkeeper rule;
- competition registration.

Bench and substitution limits come from ruleset.

Auto-selection uses:
- position suitability;
- ability;
- condition;
- sharpness;
- tactical role;
- recent load.

Its goal is a legal sensible team, not perfect optimization.

## Tactics

Team profile:
- formation;
- mentality;
- tempo;
- width;
- passing directness;
- pressing;
- defensive line;
- line of engagement;
- counterpress;
- counterattack;
- build-up side/style;
- time wasting;
- crossing;
- goalkeeper distribution.

Player slot:
- position;
- role;
- duty;
- assigned player.

Role changes:
- passing risk;
- movement;
- press frequency;
- width;
- shot tendency;
- crossing;
- dribbling;
- defensive positioning.

Track formation/role familiarity with modest influence.

## Transfers

AI and human squad planning categorizes needs:
- critical;
- upgrade;
- depth;
- prospect;
- surplus.

Offer state machine:

```text
Draft
→ Submitted
→ UnderReview
→ Rejected | Countered | AcceptedClub
→ ContractNegotiation
→ Checks
→ Completed

or Withdrawn | Expired | Failed
```

Persist the state.

Fee components v1:
- upfront;
- installments;
- appearance add-on;
- team achievement add-on;
- sell-on percentage.

Loan:
- duration;
- fee;
- wage contribution;
- recall option;
- expected playing time;
- registration eligibility.

Valuation inputs:
- current ability;
- potential;
- age;
- reputation;
- league/club level;
- contract time;
- wage;
- form;
- demand;
- position scarcity.

Value is not a guaranteed selling price.

Club negotiation considers:
- replacement cost;
- player importance;
- financial need;
- demand;
- remaining contract;
- transfer policy.

Use seeded variation, not one visible threshold.

## Contracts

Fields:
- start/end;
- weekly wage;
- squad status;
- signing fee;
- appearance/goal/clean-sheet bonuses baseline;
- release clause framework;
- extension option;
- loan parent relation where relevant.

Player decision considers:
- wage;
- reputation;
- league;
- expected minutes;
- ambition;
- adaptability;
- relationship;
- competing offers.

Prevent spam exploit with finite rounds/cooldown.

## Finance

Every balance change creates a ledger transaction:
- date;
- club;
- category;
- amount in minor units;
- currency;
- reference;
- localized description key.

Track:
- cash;
- transfer budget;
- wage budget;
- wages;
- future transfer commitments;
- projected income/cost.

Revenue:
- matchday;
- commercial/sponsor abstraction;
- broadcast abstraction;
- prize;
- sales/loans.

Costs:
- wages;
- transfer payments;
- fees/bonuses;
- staff;
- facilities;
- youth/operations.

Financial regulations are versioned rule evaluators.

## Fitness

Track 0–100 concepts:
- condition;
- fatigue load;
- match sharpness.

Fatigue inputs:
- minutes;
- tactics;
- travel abstraction;
- natural fitness/stamina;
- rest;
- training.

## Injury

Risk:
- baseline;
- contact;
- fatigue;
- proneness;
- training load;
- recent return.

Record:
- type/category;
- severity;
- estimate range;
- recurrence;
- training availability.

Match engine emits incident; medical system finalizes persistent injury.

## Training

Weekly:
- recovery;
- physical;
- tactical;
- technical;
- attacking;
- defending;
- set pieces;
- match preparation.

Individual:
- position/role;
- attribute focus;
- intensity.

Effect constrained by:
- potential;
- age;
- coaching;
- professionalism;
- health;
- workload/minutes.

Overtraining raises fatigue/risk.

## Development/aging

Use position/attribute-specific age curves.

Principles:
- physical peaks/declines differently from mental/technical;
- growth is uncertain;
- minutes help but do not guarantee growth;
- older players can retain technique while losing pace;
- severe injuries can affect development;
- potential is a ceiling/range, not guaranteed destination.

Apply weekly/monthly development ticks, not huge post-match boosts.

## Morale and form

Separate:
- morale: medium-term satisfaction;
- form: recent performance;
- optional confidence: short-term match context.

Morale inputs:
- results;
- playing time vs promised role;
- contract;
- transfer interest;
- team cohesion;
- manager interactions.

Keep modifiers modest to avoid runaway streaks.

## Discipline

Cards/suspensions are competition-specific.
Ruleset controls:
- accumulation;
- reset;
- red-card bans;
- competition boundaries.

Match engine emits cards; competition layer applies suspension.

## Scouting

Per manager-player knowledge:
- level 0–100;
- last observed;
- known attribute ranges;
- estimated value/wage;
- personality certainty.

Assignments:
- region/league;
- position;
- age;
- role;
- budget.

Progress depends on:
- scout quality;
- scope;
- visibility;
- time;
- familiarity.

Search must not expose true hidden attributes.

## Youth

Annual intake based on:
- club youth recruitment;
- coaching;
- reputation;
- nation youth strength abstraction;
- deterministic randomness;
- light positional need.

Generated player:
- generated identity;
- age/DOB;
- positions;
- attributes;
- potential;
- personality;
- physical profile.

Persist generation seed/version.

## Staff

Minimum v1:
- assistant;
- coaches;
- scouts;
- physio/medical.

Staff affects efficiency/quality rather than hard-gating actions.

## AI managers

Persistent profile:
- formations;
- mentality;
- pressing;
- possession/directness;
- rotation;
- youth preference;
- transfer risk;
- age preference;
- adaptability.

Weekly planning:
- depth chart;
- first-choice/rotation/prospect/surplus;
- weak positions;
- contract risks.

Recruitment:
- target scoring;
- affordability;
- registration;
- age;
- style fit;
- squad need.

Guardrails:
- no impossible spending;
- no 8-goalkeeper bug;
- no selling all players at one position without replacement planning;
- legal lineup emergency fallback.

Match AI:
- pre-match selection/tactic;
- fatigue rotation;
- injury substitution;
- score/time reactions;
- yellow-card risk;
- late attacking/defensive changes.

No omniscient knowledge of unsubmitted human tactics.

## Manager jobs

v1 minimum:
- human stays with chosen club;
- resign can be optional;
- AI managers may be hired/fired with replacement pool.

Human job applications can wait for v1.1.

## Inbox/news

Deterministic template-based messages:
- transfer;
- contract;
- medical;
- scouting;
- registration;
- finance;
- board;
- match;
- competition.

Messages can contain actionable choices/deadlines.

News from domain events:
- transfer;
- major result;
- title/relegation;
- injury;
- manager change;
- youth breakthrough.

Optional LLM prose must never decide canonical state.

## Statistics/history

Player season:
- appearances/starts/minutes;
- goals/assists;
- cards;
- clean sheets where relevant;
- detailed match stats where engine provides;
- average rating.

Club:
- record;
- goals;
- trophies;
- transfer spend/income.

Persist career totals and competition history.

## Registration and work eligibility

Ruleset supplies:
- squad size;
- homegrown;
- age exemption;
- transfer/registration dates;
- competition eligibility.

Work-permit/eligibility uses configurable evaluator and reason codes. Avoid hardcoded legal text.

## Difficulty

No hidden cheating difficulty.

Optional settings can adjust:
- scouting uncertainty;
- AI planning depth;
- negotiation tolerance;
- assistant automation.

Default AI follows same economy/physics as users.

## Gameplay acceptance

A headless 10-season simulation must preserve:
- valid club counts;
- correct promotion/relegation;
- cup winners;
- contracts;
- transfers/windows;
- legal lineups;
- youth replenishment;
- numeric financial validity;
- history records;
- no impossible permanent duplicate club ownership for players.
