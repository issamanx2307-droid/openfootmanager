# Project Albion — Data Snapshot and Database Specification

## Core rule

Real-world data and career state are separate products.

A real-world snapshot is an immutable starting dataset.
A career database is a mutable alternate football history.

Updating snapshots never modifies existing careers.

## Pipeline

```text
source/API/manual file
→ raw capture
→ provider parser
→ normalized staging
→ identity resolution
→ validation
→ Albion rating generation
→ human-readable diff
→ publish immutable snapshot
→ New Game import
→ career diverges forever
```

## Snapshot states

- draft
- validated
- published
- deprecated

Published snapshots are immutable.

Example ID:
`england-2026-27-summer-close-v1`

Manifest:

```yaml
snapshot_id: england-2026-27-summer-close-v1
snapshot_schema_version: 1
season: "2026/27"
freeze_date: "YYYY-MM-DD"
published_at: "ISO-8601"
rating_model_version: albion-rating-v1
ruleset_version: england-2026-27-v1
content_hash: "sha256:..."
sources:
  - provider: example
    retrieved_at: "ISO-8601"
    note: "operator-recorded provenance"
```

Do not invent a freeze date. Operator intentionally publishes the snapshot after the desired transfer window.

## Season Reset import

Copy:
- club identities;
- player identities;
- roster membership as of freeze;
- known contracts;
- positions/nationalities/DOB;
- competition membership;
- generated Albion attributes;
- configured reputation/financial seeds.

Reset:
- tables;
- points;
- W/D/L/GF/GA;
- player season stats;
- real match results;
- real transfer rumors;
- real future transfers;
- real season form.

Injuries/suspensions:
- default reset for Season Reset;
- allow snapshot setting later if operator wants them.

Fixtures:
- generate from rules or import fixture pairing/dates;
- mark all unplayed.

## Provider abstraction

Never couple the updater to one website.

Required provider/adapters:
- manual CSV;
- manual JSON;
- deterministic test fixture;
- at least one structured provider adapter when practical.

Contract concept:

```text
fetch/read input
→ RawRecord[]
→ normalize
→ NormalizedBatch
```

If a source blocks automation, use manual downloaded data rather than bypassing technical controls.

## Provenance

Track source and generated origin.

Conceptual tables:
- data_source
- source_record
- entity_external_id
- field_provenance

For each imported fact record:
- entity_id;
- field;
- source;
- source key;
- retrieved time;
- confidence.

Generated game data:
- `origin = albion_rating_v1`
- `origin = generated`
- `origin = operator_override`

Do not present generated estimates as verified facts.

## Stable identity

Names are not IDs.

Use UUID/stable project IDs.

External mapping:

```text
entity_external_id(
  entity_type,
  entity_id,
  provider,
  external_key,
  first_seen_at,
  last_seen_at,
  UNIQUE(provider, entity_type, external_key)
)
```

Resolution order:
1. exact provider stable ID;
2. trusted cross-provider mapping;
3. exact normalized name + DOB + nationality;
4. high-confidence fuzzy candidate;
5. manual review.

Never auto-merge ambiguous people.

## Name handling

Store:
- canonical display name;
- known name;
- sort name;
- normalized search name;
- aliases.

Preserve Unicode/original spelling.
Diacritic stripping is search-only.

## Snapshot data model

Recommended tables:
- snapshot_meta
- countries
- cities
- stadiums
- clubs
- club_aliases
- competitions
- competition_memberships
- players
- player_positions
- player_attributes
- player_traits
- contracts
- loans
- staff
- club_financial_seeds
- source_entities
- field_provenance

Player minimum:

```text
id
full_name
known_name
date_of_birth nullable
nationality_code nullable
football_nation_code nullable
height_cm nullable
preferred_foot nullable
primary_position
current_club_id nullable
status
```

## Career database model

One SQLite file per career.

### Metadata
- career_meta
- schema_migrations
- career_versions
- human_managers
- manager_sessions

### Football entities
- clubs
- players
- player_positions
- player_attributes
- player_traits
- staff
- contracts
- loans
- squad_memberships
- registrations

### Competition
- seasons
- competitions
- competition_editions
- competition_entries
- fixtures
- match_results
- standings
- discipline
- cup_brackets
- qualification_slots

### Matches
- matches
- match_lineups
- match_bench
- match_tactics
- match_events
- match_team_stats
- match_player_stats
- match_seed

Background matches may store summary only.

### Transfer/contract
- transfer_listings
- transfer_offers
- transfer_offer_clauses
- transfer_negotiations
- transfer_history
- contract_offers
- contract_history

### Fitness/development
- fitness_state
- injuries
- medical_history
- suspensions
- training_plans
- player_development_log

### Finance
- club_finance_accounts
- finance_transactions
- budgets
- transfer_installments
- wage_commitments
- financial_rule_periods

All money uses integer minor units or exact decimal, never float.

### Scouting
- scouting_assignments
- manager_player_knowledge
- scouting_reports
- shortlists

### AI
- ai_manager_profiles
- ai_squad_plans
- ai_recruitment_plans

### History/news
- domain_events
- inbox_messages
- news_items
- club_history
- player_career_stats
- manager_history
- competition_history
- awards

## Constraints

Enforce useful DB constraints:
- fixture home != away;
- non-negative fees/wages;
- unique result per fixture;
- one competition entry per club/edition;
- one human manager control per club;
- valid FK references;
- no impossible money range;
- canonical entity IDs unique.

Domain services enforce higher-order rules.

## Indexes

At least:
- player normalized name;
- player club;
- contract club/end date;
- contract player/status;
- fixture date/competition;
- fixture home/away;
- transfer status/deadline;
- inbox manager/read state;
- standings edition/rank;
- domain event date/type.

Profile before adding FTS.

## Snapshot diff

Before publish output machine JSON and human Markdown/HTML.

Summary:
- clubs added/removed/changed;
- players added/removed/changed;
- roster movement;
- contract changes;
- identity conflicts;
- unknown data;
- rating-model changes.

Per club:

```text
Club X
IN:
 + Player A
OUT:
 - Player B
CHANGED:
 * Player C contract date
```

Critical validation failures block publication.

## Validation

Critical:
- duplicate canonical ID;
- broken FK;
- duplicate stable external key;
- one player permanently assigned to multiple clubs;
- incompatible league membership;
- impossible date;
- invalid contract interval;
- missing required competition;
- manifest hash mismatch.

Warnings:
- missing height;
- unknown preferred foot;
- missing contract end;
- low-confidence candidate;
- missing shirt number.

Warnings do not block unless operator policy says so.

## Rating model

Real facts/statistics can feed an original Albion model.

Possible inputs:
- age;
- position;
- minutes/starts;
- goals/assists;
- shots;
- passing/progression;
- defensive/aerial actions;
- goalkeeper metrics;
- competition strength;
- team role.

Missing metrics degrade gracefully.

Outputs:
- 1–100 attributes;
- current ability summary;
- potential range;
- confidence;
- explanation factors;
- model version.

Potential should reflect:
- age;
- current level;
- minutes;
- league strength;
- uncertainty.

Any random uncertainty is deterministic from stable player ID + rating model version.

## Manual overrides

Support audited override:
- field/attribute;
- new value;
- reason;
- timestamp;
- operator;
- pinned/unpinned.

Do not overwrite source facts.
Pinned overrides survive re-import until intentionally removed.

## Club financial seeds

Exact public finances may be incomplete.

Allow:
- approved known values;
- generated league/reputation estimates.

Generated estimates are labeled as generated.

## Schedule handling

Support:
1. generated schedule;
2. imported schedule with all results removed.

Validate:
- correct round/pairing count;
- no club double-booking;
- cup/continental collision handling;
- configured minimum rest;
- all canonical Season Reset matches unplayed.

## Snapshot update workflow

Example:

```text
albion-data import --season 2026-27 --stage
albion-data validate
albion-data diff --against <previous>
albion-data publish --id england-2026-27-winter-close-v1
```

Existing career remains unchanged.
New career may select the new snapshot.

## Reproducibility

Preserve:
- raw input hashes;
- pipeline version/git hash;
- rating model;
- ruleset;
- deterministic config;
- output content hash.

Same inputs/config should create same normalized content, excluding non-content publication timestamps.

## Save safety

Career DB:
- transactions;
- autosave checkpoints;
- 3–5 rotating backups;
- integrity checks after suspicious shutdown;
- never overwrite last known-good backup with corrupt content.

Snapshot:
- verify content hash before career creation.

## Migrations

Every career schema change:
- numbered migration;
- tested from previous supported schema;
- transactional where possible;
- backup before risky migration;
- version increments only after success.

## Acceptance criteria

Data subsystem is complete when:
- synthetic fixture dataset publishes;
- manual CSV/JSON works;
- identity survives transfer across snapshots;
- diff reports movement;
- ambiguous identity requires review;
- published snapshot hashes correctly;
- new career seeds correctly;
- updating snapshot does not alter old career;
- save closes/reopens correctly;
- migration tests pass;
- 10-season historical data remains queryable.
