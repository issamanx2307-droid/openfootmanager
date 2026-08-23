//! Stable, versioned simulation contracts for Project Albion.
//!
//! The low-level `simulate_with_rng` and `LiveMatchState` APIs remain useful
//! internally, but callers that need replayability should use this module. It
//! owns the seeded RNG for a live game so commands cannot accidentally source
//! entropy from the UI or wall clock.

use rand::SeedableRng;
use rand::rngs::StdRng;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::event::MatchEvent;
use crate::live_match::{LiveMatchState, MatchCommand, SubstitutionRules};
use crate::report::MatchReport;
use crate::types::{MatchConfig, TeamData};

/// The version assigned to every result produced by this implementation.
pub const MATCH_ENGINE_VERSION: &str = "albion-engine-v1";

/// Explicit source of all match randomness.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct MatchSeed(pub u64);

/// Complete football input for instant or live simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchInput {
    pub home: TeamData,
    pub away: TeamData,
    #[serde(default)]
    pub config: MatchConfig,
    #[serde(default)]
    pub home_bench: Vec<crate::types::PlayerData>,
    #[serde(default)]
    pub away_bench: Vec<crate::types::PlayerData>,
    #[serde(default)]
    pub allows_extra_time: bool,
    #[serde(skip)]
    pub substitution_rules: SubstitutionRules,
}

impl MatchInput {
    /// Stable FNV-1a fingerprint of the serialized input. Persist it alongside
    /// the seed and engine version to make a stored match replayable/debuggable.
    pub fn fingerprint(&self) -> Result<String, String> {
        let payload = serde_json::to_vec(self).map_err(|error| error.to_string())?;
        Ok(format!("{:016x}", fnv1a64(&payload)))
    }
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    bytes.iter().fold(OFFSET, |hash, byte| (hash ^ u64::from(*byte)).wrapping_mul(PRIME))
}

/// Stable one-shot simulator contract.
pub trait MatchSimulator {
    fn engine_version(&self) -> &'static str;
    fn simulate(&self, input: MatchInput, seed: MatchSeed) -> Result<MatchReport, String>;
}

/// Stable interactive simulator contract.
pub trait LiveMatchSimulator {
    fn engine_version(&self) -> &'static str;
    fn start(&self, input: MatchInput, seed: MatchSeed) -> Result<SeededLiveMatch, String>;
    fn step(
        &self,
        state: &mut SeededLiveMatch,
        commands: &[MatchCommand],
    ) -> Result<Vec<MatchEvent>, String>;
}

/// Albion engine v1 implementation of both stable contracts.
#[derive(Debug, Default, Clone, Copy)]
pub struct AlbionV1Simulator;

impl MatchSimulator for AlbionV1Simulator {
    fn engine_version(&self) -> &'static str {
        MATCH_ENGINE_VERSION
    }

    fn simulate(&self, input: MatchInput, seed: MatchSeed) -> Result<MatchReport, String> {
        let mut live = LiveMatchSimulator::start(self, input, seed)?;
        while !live.is_finished() {
            LiveMatchSimulator::step(self, &mut live, &[])?;
        }
        Ok(live.into_report())
    }
}

impl LiveMatchSimulator for AlbionV1Simulator {
    fn engine_version(&self) -> &'static str {
        MATCH_ENGINE_VERSION
    }

    fn start(&self, input: MatchInput, seed: MatchSeed) -> Result<SeededLiveMatch, String> {
        Ok(SeededLiveMatch {
            state: LiveMatchState::new_with_substitution_rules(
                input.home,
                input.away,
                input.config,
                input.home_bench,
                input.away_bench,
                input.allows_extra_time,
                input.substitution_rules,
            ),
            seed,
            rng: StdRng::seed_from_u64(seed.0),
        })
    }

    fn step(
        &self,
        state: &mut SeededLiveMatch,
        commands: &[MatchCommand],
    ) -> Result<Vec<MatchEvent>, String> {
        for command in commands {
            state.state.apply_command(command.clone())?;
        }
        Ok(state.state.step_minute(&mut state.rng).events)
    }
}

/// A live state paired with the original seed and its private deterministic RNG.
pub struct SeededLiveMatch {
    state: LiveMatchState,
    rng: StdRng,
    seed: MatchSeed,
}

impl SeededLiveMatch {
    pub fn seed(&self) -> MatchSeed {
        self.seed
    }

    pub fn state(&self) -> &LiveMatchState {
        &self.state
    }

    pub fn is_finished(&self) -> bool {
        self.state.is_finished()
    }

    pub fn into_report(self) -> MatchReport {
        self.state.into_report()
    }
}

/// Stable FNV-1a fingerprint for the completed report used by replay checks.
pub fn report_fingerprint(report: &MatchReport) -> Result<String, String> {
    #[derive(Serialize)]
    struct CanonicalReport<'a> {
        home_goals: u8,
        away_goals: u8,
        home_stats: &'a crate::report::TeamStats,
        away_stats: &'a crate::report::TeamStats,
        events: &'a [MatchEvent],
        goals: &'a [crate::report::GoalDetail],
        player_stats: BTreeMap<&'a String, &'a crate::report::PlayerMatchStats>,
        home_possession: f64,
        total_minutes: u8,
        home_penalties: Option<u8>,
        away_penalties: Option<u8>,
    }

    let player_stats = report.player_stats.iter().collect();
    let canonical = CanonicalReport {
        home_goals: report.home_goals,
        away_goals: report.away_goals,
        home_stats: &report.home_stats,
        away_stats: &report.away_stats,
        events: &report.events,
        goals: &report.goals,
        player_stats,
        home_possession: report.home_possession,
        total_minutes: report.total_minutes,
        home_penalties: report.home_penalties,
        away_penalties: report.away_penalties,
    };
    let payload = serde_json::to_vec(&canonical).map_err(|error| error.to_string())?;
    Ok(format!("{:016x}", fnv1a64(&payload)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{PlayStyle, PlayerData, Position, TacticsConfig};

    fn player(id: &str, position: Position) -> PlayerData {
        PlayerData {
            id: id.to_string(),
            name: id.to_string(),
            position,
            ovr: 70,
            condition: 100,
            fitness: 75,
            pace: 70,
            stamina: 70,
            strength: 70,
            agility: 70,
            passing: 70,
            shooting: 70,
            tackling: 70,
            dribbling: 70,
            defending: 70,
            positioning: 70,
            vision: 70,
            decisions: 70,
            composure: 70,
            aggression: 50,
            teamwork: 70,
            leadership: 70,
            handling: 70,
            reflexes: 70,
            aerial: 70,
            traits: Vec::new(),
            role: Default::default(),
        }
    }

    fn team(id: &str) -> TeamData {
        let positions = [
            Position::Goalkeeper,
            Position::Defender,
            Position::Defender,
            Position::Defender,
            Position::Defender,
            Position::Midfielder,
            Position::Midfielder,
            Position::Midfielder,
            Position::Midfielder,
            Position::Forward,
            Position::Forward,
        ];
        TeamData {
            id: id.to_string(),
            name: id.to_string(),
            formation: "4-4-2".to_string(),
            play_style: PlayStyle::Balanced,
            players: positions
                .into_iter()
                .enumerate()
                .map(|(index, position)| player(&format!("{id}-{index}"), position))
                .collect(),
            tactics: TacticsConfig::default(),
        }
    }

    fn input() -> MatchInput {
        MatchInput {
            home: team("home"),
            away: team("away"),
            config: MatchConfig::default(),
            home_bench: Vec::new(),
            away_bench: Vec::new(),
            allows_extra_time: false,
            substitution_rules: SubstitutionRules::default(),
        }
    }

    #[test]
    fn versioned_instant_simulator_replays_an_exact_report() {
        let simulator = AlbionV1Simulator;
        let match_input = input();
        let seed = MatchSeed(4_242);

        let first = MatchSimulator::simulate(&simulator, match_input.clone(), seed).unwrap();
        let replay = MatchSimulator::simulate(&simulator, match_input.clone(), seed).unwrap();

        assert_eq!(
            MatchSimulator::engine_version(&simulator),
            MATCH_ENGINE_VERSION
        );
        assert_eq!(report_fingerprint(&first).unwrap(), report_fingerprint(&replay).unwrap());
        assert_eq!(
            report_fingerprint(&first).unwrap(),
            report_fingerprint(&replay).unwrap(),
            "same input, seed and engine version must produce an exact report"
        );
        assert_eq!(match_input.fingerprint().unwrap(), input().fingerprint().unwrap());
    }

    #[test]
    fn seeded_live_simulator_replays_without_external_rng() {
        let simulator = AlbionV1Simulator;
        let run = |simulator: AlbionV1Simulator| {
            let mut state = LiveMatchSimulator::start(&simulator, input(), MatchSeed(7)).unwrap();
            while !state.is_finished() {
                LiveMatchSimulator::step(&simulator, &mut state, &[]).unwrap();
            }
            state.into_report()
        };

        let first = run(simulator);
        let replay = run(simulator);
        assert_eq!(report_fingerprint(&first).unwrap(), report_fingerprint(&replay).unwrap());
    }

    #[test]
    fn instant_and_live_contracts_are_consistent_without_commands() {
        let simulator = AlbionV1Simulator;
        let seed = MatchSeed(99);
        let instant = MatchSimulator::simulate(&simulator, input(), seed).unwrap();
        let mut live = LiveMatchSimulator::start(&simulator, input(), seed).unwrap();
        while !live.is_finished() {
            LiveMatchSimulator::step(&simulator, &mut live, &[]).unwrap();
        }

        assert_eq!(
            report_fingerprint(&instant).unwrap(),
            report_fingerprint(&live.into_report()).unwrap(),
            "instant and untouched live simulation must replay the same match"
        );
    }

    #[test]
    fn input_fingerprint_changes_with_a_football_input_change() {
        let baseline = input();
        let mut changed = baseline.clone();
        changed.home.tactics.pressing_intensity = crate::types::PressingIntensity::Aggressive;

        assert_ne!(baseline.fingerprint().unwrap(), changed.fingerprint().unwrap());
    }
}
