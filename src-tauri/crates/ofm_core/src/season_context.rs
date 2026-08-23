use crate::end_of_season::is_league_complete;
use crate::game::{Game, TransferWindowRule};
use chrono::{Datelike, Duration, NaiveDate};
use domain::league::League;
use domain::season::{SeasonContext, SeasonPhase, TransferWindowContext, TransferWindowStatus};

const TRANSFER_WINDOW_PRESEASON_DAYS: i64 = 30;
const TRANSFER_WINDOW_POST_START_DAYS: i64 = 30;

pub fn refresh_game_context(game: &mut Game) {
    game.season_context = derive_season_context(game);
}

pub fn derive_season_context(game: &Game) -> SeasonContext {
    let Some(league) = &game.league else {
        return SeasonContext::default();
    };

    let season_start = league_boundary_date(league, Boundary::Start);
    let season_end = league_boundary_date(league, Boundary::End);
    let current_date = game.clock.current_date.date_naive();

    let phase = if is_league_complete(league) {
        SeasonPhase::PostSeason
    } else if league_has_started(league) {
        SeasonPhase::InSeason
    } else {
        SeasonPhase::Preseason
    };

    let days_until_season_start = season_start.and_then(|start| {
        let days = (start - current_date).num_days();
        (days >= 0).then_some(days)
    });

    let transfer_window = if game.transfer_windows.is_empty() {
        derive_transfer_window_context(current_date, season_start)
    } else {
        derive_configured_transfer_window_context(current_date, &game.transfer_windows)
    };

    SeasonContext {
        phase,
        season_start: season_start.map(format_date),
        season_end: season_end.map(format_date),
        days_until_season_start,
        transfer_window,
    }
}

#[derive(Copy, Clone)]
enum Boundary {
    Start,
    End,
}

fn league_boundary_date(league: &League, boundary: Boundary) -> Option<NaiveDate> {
    league
        .fixtures
        .iter()
        .filter(|fixture| fixture.counts_for_league_standings())
        .filter_map(|fixture| NaiveDate::parse_from_str(&fixture.date, "%Y-%m-%d").ok())
        .reduce(|left, right| match boundary {
            Boundary::Start => left.min(right),
            Boundary::End => left.max(right),
        })
}

fn league_has_started(league: &League) -> bool {
    league.standings.iter().any(|entry| entry.played > 0)
        || league.fixtures.iter().any(|fixture| {
            fixture.counts_for_league_standings()
                && fixture.status == domain::league::FixtureStatus::Completed
        })
}

fn derive_transfer_window_context(
    current_date: NaiveDate,
    season_start: Option<NaiveDate>,
) -> TransferWindowContext {
    let Some(mut window_season_start) = season_start else {
        return TransferWindowContext::default();
    };

    let mut opens_on = transfer_window_opens_on(window_season_start);
    let mut closes_on = transfer_window_closes_on(window_season_start);

    while current_date > closes_on {
        window_season_start = add_year_clamped(window_season_start);
        opens_on = transfer_window_opens_on(window_season_start);
        closes_on = transfer_window_closes_on(window_season_start);
    }

    let (status, days_until_opens, days_remaining) = if current_date < opens_on {
        (
            TransferWindowStatus::Closed,
            Some((opens_on - current_date).num_days()),
            None,
        )
    } else if current_date > closes_on {
        (TransferWindowStatus::Closed, None, None)
    } else {
        let remaining = (closes_on - current_date).num_days();
        let status = if remaining == 0 {
            TransferWindowStatus::DeadlineDay
        } else {
            TransferWindowStatus::Open
        };
        (status, None, Some(remaining))
    };

    TransferWindowContext {
        status,
        opens_on: Some(format_date(opens_on)),
        closes_on: Some(format_date(closes_on)),
        days_until_opens,
        days_remaining,
    }
}

fn derive_configured_transfer_window_context(
    current_date: NaiveDate,
    windows: &[TransferWindowRule],
) -> TransferWindowContext {
    let mut active = None;
    let mut next = None;
    for year in [current_date.year() - 1, current_date.year(), current_date.year() + 1] {
        for window in windows {
            let Some(opens_on) = NaiveDate::from_ymd_opt(year, window.start_month.into(), window.start_day.into()) else { continue; };
            let end_year = if (window.end_month, window.end_day) < (window.start_month, window.start_day) { year + 1 } else { year };
            let Some(closes_on) = NaiveDate::from_ymd_opt(end_year, window.end_month.into(), window.end_day.into()) else { continue; };
            if current_date >= opens_on && current_date <= closes_on {
                active = Some((opens_on, closes_on));
            } else if opens_on > current_date && next.is_none_or(|(candidate, _)| opens_on < candidate) {
                next = Some((opens_on, closes_on));
            }
        }
    }
    let (opens_on, closes_on, status) = if let Some((opens_on, closes_on)) = active {
        (opens_on, closes_on, if current_date == closes_on { TransferWindowStatus::DeadlineDay } else { TransferWindowStatus::Open })
    } else if let Some((opens_on, closes_on)) = next {
        (opens_on, closes_on, TransferWindowStatus::Closed)
    } else {
        return TransferWindowContext::default();
    };
    TransferWindowContext {
        status,
        opens_on: Some(format_date(opens_on)),
        closes_on: Some(format_date(closes_on)),
        days_until_opens: (current_date < opens_on).then_some((opens_on - current_date).num_days()),
        days_remaining: (current_date >= opens_on && current_date <= closes_on).then_some((closes_on - current_date).num_days()),
    }
}

fn format_date(date: NaiveDate) -> String {
    date.format("%Y-%m-%d").to_string()
}

fn transfer_window_opens_on(season_start: NaiveDate) -> NaiveDate {
    season_start - Duration::days(TRANSFER_WINDOW_PRESEASON_DAYS)
}

fn transfer_window_closes_on(season_start: NaiveDate) -> NaiveDate {
    season_start + Duration::days(TRANSFER_WINDOW_POST_START_DAYS)
}

fn add_year_clamped(date: NaiveDate) -> NaiveDate {
    let target_year = date.year() + 1;
    date.with_year(target_year).unwrap_or_else(|| {
        NaiveDate::from_ymd_opt(target_year, date.month(), 1)
            .and_then(|first_of_month| {
                first_of_month
                    .checked_add_months(chrono::Months::new(1))
                    .and_then(|next_month| next_month.checked_sub_signed(Duration::days(1)))
            })
            .expect("target year and month should produce a valid date")
    })
}

#[cfg(test)]
mod tests {
    use super::derive_season_context;
    use crate::clock::GameClock;
    use crate::game::{Game, TransferWindowRule};
    use chrono::{TimeZone, Utc};
    use domain::league::{
        Fixture, FixtureCompetition, FixtureStatus, League, MatchResult, StandingEntry,
    };
    use domain::manager::Manager;
    use domain::season::{SeasonPhase, TransferWindowStatus};
    use domain::team::Team;

    fn make_team(id: &str, name: &str) -> Team {
        Team::new(
            id.to_string(),
            name.to_string(),
            name.to_string(),
            "England".to_string(),
            "Test City".to_string(),
            format!("{} Ground", name),
            20_000,
        )
    }

    fn make_fixture(id: &str, date: &str, status: FixtureStatus, matchday: u32) -> Fixture {
        Fixture {
            id: id.to_string(),
            matchday,
            date: date.to_string(),
            home_team_id: "team1".to_string(),
            away_team_id: "team2".to_string(),
            competition: FixtureCompetition::League,
            status: status.clone(),
            result: (status == FixtureStatus::Completed).then_some(MatchResult {
                home_goals: 1,
                away_goals: 0,
                home_scorers: vec![],
                away_scorers: vec![],
                report: None,
                home_penalties: None,
                away_penalties: None,
            }),
            ..Default::default()
        }
    }

    fn make_game(current_date: (i32, u32, u32), league: Option<League>) -> Game {
        let clock = GameClock::new(
            Utc.with_ymd_and_hms(current_date.0, current_date.1, current_date.2, 12, 0, 0)
                .unwrap(),
        );
        let manager = Manager::new(
            "mgr1".to_string(),
            "Alex".to_string(),
            "Manager".to_string(),
            "1980-01-01".to_string(),
            "England".to_string(),
        );
        let mut game = Game::new(
            clock,
            manager,
            vec![
                make_team("team1", "Alpha FC"),
                make_team("team2", "Beta FC"),
            ],
            vec![],
            vec![],
            vec![],
        );
        game.league = league;
        game
    }

    #[test]
    fn derives_preseason_context_before_first_fixture() {
        let league = League {
            id: "league1".to_string(),
            name: "Premier Division".to_string(),
            season: 2026,
            fixtures: vec![make_fixture(
                "fx1",
                "2026-08-01",
                FixtureStatus::Scheduled,
                1,
            )],
            standings: vec![
                StandingEntry::new("team1".to_string()),
                StandingEntry::new("team2".to_string()),
            ],
            transfer_log: vec![],
            transfer_rumours: vec![],
            ..Default::default()
        };
        let game = make_game((2026, 7, 10), Some(league));

        let context = derive_season_context(&game);

        assert_eq!(context.phase, SeasonPhase::Preseason);
        assert_eq!(context.season_start.as_deref(), Some("2026-08-01"));
        assert_eq!(context.days_until_season_start, Some(22));
        assert_eq!(context.transfer_window.status, TransferWindowStatus::Open);
        assert_eq!(context.transfer_window.days_remaining, Some(52));
    }

    #[test]
    fn derives_deadline_day_window_status() {
        let league = League {
            id: "league1".to_string(),
            name: "Premier Division".to_string(),
            season: 2026,
            fixtures: vec![make_fixture(
                "fx1",
                "2026-08-01",
                FixtureStatus::Scheduled,
                1,
            )],
            standings: vec![
                StandingEntry::new("team1".to_string()),
                StandingEntry::new("team2".to_string()),
            ],
            transfer_log: vec![],
            transfer_rumours: vec![],
            ..Default::default()
        };
        let game = make_game((2026, 8, 31), Some(league));

        let context = derive_season_context(&game);

        assert_eq!(
            context.transfer_window.status,
            TransferWindowStatus::DeadlineDay
        );
        assert_eq!(context.transfer_window.days_remaining, Some(0));
    }

    #[test]
    fn pinned_transfer_calendar_overrides_legacy_season_relative_window() {
        let mut game = make_game((2026, 12, 20), Some(League::default()));
        game.transfer_windows = vec![
            TransferWindowRule {
                name: "summer".to_string(),
                start_month: 6,
                start_day: 10,
                end_month: 9,
                end_day: 1,
            },
            TransferWindowRule {
                name: "winter".to_string(),
                start_month: 1,
                start_day: 1,
                end_month: 2,
                end_day: 3,
            },
        ];

        let context = derive_season_context(&game);

        assert_eq!(context.transfer_window.status, TransferWindowStatus::Closed);
        assert_eq!(context.transfer_window.opens_on.as_deref(), Some("2027-01-01"));
        assert_eq!(context.transfer_window.closes_on.as_deref(), Some("2027-02-03"));
        assert_eq!(context.transfer_window.days_until_opens, Some(12));
    }

    #[test]
    fn derives_next_window_after_current_window_has_closed() {
        let league = League {
            id: "league1".to_string(),
            name: "Premier Division".to_string(),
            season: 2026,
            fixtures: vec![make_fixture(
                "fx1",
                "2026-08-01",
                FixtureStatus::Completed,
                1,
            )],
            standings: vec![
                StandingEntry::new("team1".to_string()),
                StandingEntry::new("team2".to_string()),
            ],
            transfer_log: vec![],
            transfer_rumours: vec![],
            ..Default::default()
        };
        let game = make_game((2026, 9, 15), Some(league));

        let context = derive_season_context(&game);

        assert_eq!(context.transfer_window.status, TransferWindowStatus::Closed);
        assert_eq!(
            context.transfer_window.opens_on.as_deref(),
            Some("2027-07-02")
        );
        assert_eq!(
            context.transfer_window.closes_on.as_deref(),
            Some("2027-08-31")
        );
        assert_eq!(context.transfer_window.days_until_opens, Some(290));
        assert_eq!(context.transfer_window.days_remaining, None);
    }

    #[test]
    fn derives_in_season_context_after_matches_begin() {
        let mut alpha = StandingEntry::new("team1".to_string());
        alpha.record_result(2, 1);
        let mut beta = StandingEntry::new("team2".to_string());
        beta.record_result(1, 2);
        let league = League {
            id: "league1".to_string(),
            name: "Premier Division".to_string(),
            season: 2026,
            fixtures: vec![make_fixture(
                "fx1",
                "2026-08-01",
                FixtureStatus::Completed,
                1,
            )],
            standings: vec![alpha, beta],
            transfer_log: vec![],
            transfer_rumours: vec![],
            ..Default::default()
        };
        let game = make_game((2026, 8, 5), Some(league));

        let context = derive_season_context(&game);

        assert_eq!(context.phase, SeasonPhase::InSeason);
        assert_eq!(context.days_until_season_start, None);
        assert_eq!(context.transfer_window.status, TransferWindowStatus::Open);
    }

    #[test]
    fn derives_postseason_context_once_league_is_complete() {
        let mut alpha = StandingEntry::new("team1".to_string());
        alpha.record_result(2, 1);
        let mut beta = StandingEntry::new("team2".to_string());
        beta.record_result(1, 2);
        let league = League {
            id: "league1".to_string(),
            name: "Premier Division".to_string(),
            season: 2026,
            fixtures: vec![
                make_fixture("fx1", "2026-08-01", FixtureStatus::Completed, 1),
                make_fixture("fx2", "2026-08-08", FixtureStatus::Completed, 2),
            ],
            standings: vec![alpha, beta],
            transfer_log: vec![],
            transfer_rumours: vec![],
            ..Default::default()
        };
        let game = make_game((2026, 8, 9), Some(league));

        let context = derive_season_context(&game);

        assert_eq!(context.phase, SeasonPhase::PostSeason);
        assert_eq!(context.season_end.as_deref(), Some("2026-08-08"));
    }
}
