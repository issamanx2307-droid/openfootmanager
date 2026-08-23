# Project Albion — UI/UX and Localization Specification

## Goal

Create an original desktop football-management interface optimized for information density, quick comparison, obvious shared-game status, keyboard/mouse use and first-class English/Thai localization.

Do not copy Football Manager visual assets/layout verbatim.

## Main frame

```text
┌───────────────────────────────────────────────────────────────┐
│ Career | Date | Next fixture | Ready | Other manager | Server│
├──────────────┬────────────────────────────────────────────────┤
│ Dashboard    │                                                │
│ Inbox        │                 Current route                  │
│ Squad        │                                                │
│ Tactics      │                                                │
│ Schedule     │                                                │
│ Competitions │                                                │
│ Transfers    │                                                │
│ Scouting     │                                                │
│ Training     │                                                │
│ Medical      │                                                │
│ Finances     │                                                │
│ Staff/Club   │                                                │
├──────────────┴────────────────────────────────────────────────┤
│ autosave | connection | diagnostics/notices                  │
└───────────────────────────────────────────────────────────────┘
```

Ready status must remain easily visible on most routes.

## Navigation

Primary:
- Dashboard
- Inbox
- Squad
- Tactics
- Schedule
- Competitions
- Transfers
- Scouting
- Training
- Medical
- Finances
- Staff
- Club

Global:
- search;
- multiplayer/session status;
- settings;
- host/save controls.

Use breadcrumbs for deep pages.

## Dashboard

Show:
- club;
- table position;
- recent/next fixtures;
- form;
- urgent injuries/suspensions;
- transfer/wage budgets;
- urgent inbox;
- squad concerns;
- other human manager connection/Ready state;
- Ready action.

Prioritize actionable info over decoration.

## Inbox

Columns:
- date;
- category;
- source;
- subject;
- deadline;
- action required.

Message body:
- concise explanation;
- contextual data;
- action buttons;
- links to relevant player/club/offer.

Unread/urgent must not depend on color alone.

## Squad

Configurable table:
- player;
- age;
- position;
- role;
- known ability;
- condition;
- sharpness;
- morale;
- appearances/goals/assists;
- wage;
- contract end;
- status.

Required:
- sorting/filtering;
- position grouping;
- quick details;
- multi-select where useful;
- compare up to 3 players.

## Player profile

Tabs:
- Overview
- Attributes
- Statistics
- Contract
- Development
- Medical
- History
- Scouting for non-owned

Scouted players display ranges/confidence rather than hidden true values.

## Tactics

Areas:
- pitch/formation;
- squad list;
- team instructions;
- role/duty;
- set pieces;
- validation.

Drag/drop optional, but click/keyboard alternative mandatory.

Show:
- condition;
- position suitability;
- availability;
- tactical familiarity.

Validation:
- exactly 11;
- GK;
- eligibility;
- bench;
- registration.

Support tactic presets.

## Transfers

Tabs:
- Search
- Shortlist
- Offers In
- Offers Out
- Negotiations
- Completed

Search filters:
- position;
- age;
- club/league;
- nationality;
- contract expiry;
- estimated value/wage;
- scouting knowledge;
- role fit.

Offer builder:
- upfront;
- installments;
- add-ons;
- sell-on;
- total guaranteed commitment;
- future commitment;
- budget after deal.

Never hide financial consequences.

## Contract negotiation

Show:
- demand/status;
- wage impact;
- role;
- term;
- bonuses;
- clauses;
- negotiation rounds/cooldown.

Rejection can explain high-level reason without revealing exact AI threshold.

## Scouting

Views:
- assignments;
- reports;
- shortlist;
- discovery.

Report:
- knowledge confidence;
- ability range;
- strengths/weaknesses;
- role fit;
- estimated fee/wage;
- recommendation.

## Training

Weekly calendar:
- match days;
- recovery;
- sessions;
- intensity.

Panels:
- team focus;
- units;
- individual;
- workload alerts.

Show qualitative trade-off among development, fatigue and injury risk.

## Medical

List:
- injury;
- estimated return range;
- recurrence indicator;
- condition;
- training status.

## Finances

Summary:
- balance;
- transfer budget;
- wage budget/spend;
- future commitments;
- projected season outcome;
- financial-rule status.

Ledger:
- date;
- category;
- amount;
- reference.

Locale-aware formatting.

## Competition pages

League:
- table;
- fixtures/results;
- statistics;
- rules;
- history.

Cup:
- rounds/bracket;
- fixtures;
- rules;
- prize path.

Make promotion/relegation/playoff zones accessible and clear.

## Schedule

Calendar/list views.

Show:
- date;
- competition;
- opponent;
- H/A;
- result;
- rest days;
- congestion warning.

## Match preview

Show:
- opponent;
- known tactical profile;
- form;
- injuries;
- competition rules;
- lineup validity;
- optional modeled weather/referee only if actually implemented.

Block kickoff for invalid XI unless explicit auto-select.

## Live match centre

Required:
- score/time;
- event feed;
- stats;
- player ratings;
- tactics;
- substitutes;
- speed;
- connection/opponent status.

Optional compact zone visualization.

Do not make animation block tactical action.

Human-v-human displays:
- opponent connected;
- tactical break;
- halftime Ready;
- paused/reconnecting.

## Post-match

Show:
- result;
- timeline;
- goals/cards/injuries;
- team stats;
- player ratings;
- tactical summary;
- table effect;
- next fixture.

## Multiplayer lobby

Host:
- career/snapshot;
- server status;
- private address;
- join code;
- host club;
- guest slot.

Guest:
- address;
- code;
- compatibility;
- available clubs.

Both see:
- app/protocol status;
- snapshot;
- ruleset;
- session fairness settings.

## Shared multiplayer indicator

Persistent:
- other manager name/club;
- connected/disconnected;
- Ready;
- in match;
- server saving;
- career date.

No in-game chat required for v1.

## Ready UX

Button: `Ready to Continue`.

After click:
- show `You are ready`;
- allow cancel until advancement locks;
- show other manager state.

During advancement:
- current date/progress;
- stop reason when barrier ends.

Never freeze without feedback.

## Errors

Specific states:
- connecting;
- reconnecting;
- server unavailable;
- protocol mismatch;
- save unavailable/corrupt;
- stale action requiring refresh.

Never only say "Something went wrong".

Show stable error code and optional copyable diagnostic ID.

## Visual design system

Create centralized tokens:
- typography;
- spacing;
- radius;
- surfaces;
- semantic colors;
- table density;
- states.

Avoid scattered color literals.

Support dark and light mode if practical/upstream already does.

## Resolution

Primary:
- 1440×900 recommended;
- 1920×1080 strong;
- 1280×720 minimum usable.

Not mobile-first.

Allow table horizontal scroll rather than removing critical data.

## Accessibility

Required:
- semantic HTML;
- keyboard navigation;
- visible focus;
- screen reader labels;
- non-color state indicators;
- reduced motion;
- sufficient contrast;
- form error association.

Preserve upstream accessibility improvements.

## Localization

Locales:
- English;
- Thai `th-TH`.

Suggested namespaces:
- common
- navigation
- dashboard
- inbox
- squad
- tactics
- match
- transfers
- contracts
- scouting
- training
- medical
- finance
- competitions
- multiplayer
- errors

All visible strings use i18n.

Do not construct sentences by concatenating English fragments.

Use interpolation keys with natural Thai translation order.

## Date/number/money

Store canonical values.
Display through locale-aware formatter:
- dates;
- numbers;
- percentages;
- currency.

Money in DB stays integer minor units.

## Thai layout

Test:
- Thai + Latin names;
- long translations;
- line wrapping;
- buttons;
- tables;
- currency/numbers.

Use redistributable/system-safe font stack. Do not depend on private font files.

## Frontend state

Separate:
1. server canonical cache;
2. local UI/forms;
3. user settings.

Canonical store changes after server ack/delta.

## Optimistic UI

Okay:
- panel state;
- dirty form;
- harmless local preference;
- safe idempotent message-read pending state.

Not final truth:
- completed transfer;
- signed contract;
- money change;
- substitution;
- game advancement.

Show pending until ack.

## Confirmations

Confirm genuinely destructive actions:
- delete career;
- release player;
- irreversible withdrawal if relevant;
- stop server with unsaved critical state.

Do not over-confirm routine actions.

## Frontend tests

Component:
- table sort/filter;
- lineup validation;
- transfer commitment;
- Ready state;
- localized errors;
- Thai overflow.

E2E:
- host;
- guest joins;
- club selection;
- Ready barrier;
- transfer action;
- human-v-human match;
- disconnect/reconnect;
- save/reload.

## Definition of Done

- no hardcoded major-route English strings;
- English + Thai core UI complete;
- 1280×720 usable;
- keyboard core flow works;
- scouting hidden values not leaked;
- financial commitments clear;
- live match tactical controls work;
- Ready/connection state understandable;
- no copied proprietary visual assets.
