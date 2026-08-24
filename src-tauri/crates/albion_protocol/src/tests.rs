use crate::command::*;
use crate::envelope::{Envelope, EnvelopePayload, MessageKind};
use crate::event::*;
use crate::{CURRENT_VERSIONS, ErrorCode, ProtocolError, RulesetVersion};
use uuid::Uuid;

fn roundtrip_command(command: Command) {
    let json = serde_json::to_string(&command).unwrap();
    let parsed: Command = serde_json::from_str(&json).unwrap();
    // Commands don't implement PartialEq (some bodies wrap serde_json::Value
    // via nested types), so compare via re-serialization instead.
    assert_eq!(json, serde_json::to_string(&parsed).unwrap());
}

fn roundtrip_event(event: ServerEvent) {
    let json = serde_json::to_string(&event).unwrap();
    let parsed: ServerEvent = serde_json::from_str(&json).unwrap();
    assert_eq!(json, serde_json::to_string(&parsed).unwrap());
}

#[test]
fn every_command_variant_round_trips() {
    roundtrip_command(Command::SetStartingXi(SetStartingXiBody {
        fixture_id: "fpl-fixture-2026-08-01-ars-che".into(),
        player_ids: (1..=11).map(|id| format!("fpl-{id}")).collect(),
        formation: "4-4-2".into(),
    }));
    roundtrip_command(Command::SetTactics(SetTacticsBody {
        formation: "4-3-3".into(),
        mentality: "balanced".into(),
    }));
    roundtrip_command(Command::SubmitTransferBid(SubmitTransferBidBody {
        player_id: "fpl-99".into(),
        upfront_minor: 2_500_000_00,
        installments_minor: vec![500_000_00, 500_000_00],
    }));
    roundtrip_command(Command::RespondTransferOffer(RespondTransferOfferBody {
        offer_id: "offer-99".into(),
        response: TransferOfferResponse::Counter,
        counter_upfront_minor: Some(3_000_000_00),
    }));
    roundtrip_command(Command::SubmitContractOffer(SubmitContractOfferBody {
        player_id: "fpl-99".into(),
        weekly_wage_minor: 125_000_00,
        contract_end_year: 2030,
        contract_end_month: 6,
    }));
    roundtrip_command(Command::SetTrainingPlan(SetTrainingPlanBody {
        weekly_intensity: 7,
        team_focus: "attacking_movement".into(),
    }));
    roundtrip_command(Command::MarkReady(MarkReadyBody {}));
    roundtrip_command(Command::ApplyLiveMatchCommand(ApplyLiveMatchCommandBody {
        match_id: Uuid::new_v4(),
        command: LiveMatchCommandKind::Substitute {
            player_out_id: "fpl-10".into(),
            player_in_id: "fpl-11".into(),
        },
    }));
    roundtrip_command(Command::ApplyLiveMatchCommand(ApplyLiveMatchCommandBody {
        match_id: Uuid::new_v4(),
        command: LiveMatchCommandKind::ChangeFormation { formation: "4-2-3-1".into() },
    }));
    roundtrip_command(Command::ApplyLiveMatchCommand(ApplyLiveMatchCommandBody {
        match_id: Uuid::new_v4(),
        command: LiveMatchCommandKind::ChangeRoleDuty {
            player_id: "fpl-7".into(),
            role: "advanced_playmaker".into(),
            duty: "attack".into(),
        },
    }));
    roundtrip_command(Command::ApplyLiveMatchCommand(ApplyLiveMatchCommandBody {
        match_id: Uuid::new_v4(),
        command: LiveMatchCommandKind::SetTeamInstruction {
            key: "tempo".into(),
            value: "higher".into(),
        },
    }));
}

#[test]
fn command_wire_shape_uses_stable_string_ids_not_uuids() {
    // Guards the deliberate design choice in command.rs: fixture/player ids
    // must accept non-UUID stable ids like FPL-imported records.
    let command: Command = serde_json::from_value(serde_json::json!({
        "type": "SetStartingXi",
        "body": {
            "fixture_id": "fpl-fixture-2026-08-01-ars-che",
            "player_ids": ["fpl-1", "fpl-2"],
            "formation": "4-4-2"
        }
    }))
    .unwrap();
    let Command::SetStartingXi(body) = command else {
        panic!("expected SetStartingXi command");
    };
    assert_eq!(body.fixture_id, "fpl-fixture-2026-08-01-ars-che");
    assert_eq!(body.player_ids, ["fpl-1", "fpl-2"]);
}

#[test]
fn every_event_variant_round_trips() {
    let career_id = Uuid::new_v4();
    let manager_id = Uuid::new_v4();
    let match_id = Uuid::new_v4();

    roundtrip_event(ServerEvent::Hello(HelloBody {
        career_id,
        server_versions: CURRENT_VERSIONS.to_owned_set(),
        current_revision: 12,
    }));
    roundtrip_event(ServerEvent::CommandAck(CommandAckBody {
        command_id: Uuid::new_v4(),
        applied_revision: 13,
    }));
    roundtrip_event(ServerEvent::CommandRejected(CommandRejectedBody {
        command_id: Uuid::new_v4(),
        current_revision: 13,
        error: ProtocolError::new(ErrorCode::StaleRevision).with_param("current_revision", "13"),
    }));
    roundtrip_event(ServerEvent::StateDelta(StateDeltaBody {
        from_revision: 12,
        to_revision: 13,
        changes: serde_json::json!({ "club": { "balance_minor": 500 } }),
    }));
    roundtrip_event(ServerEvent::ViewSnapshot(ViewSnapshotBody {
        revision: 13,
        view_name: "dashboard".into(),
        payload: serde_json::json!({ "club_name": "Albion" }),
    }));
    roundtrip_event(ServerEvent::GameTimeChanged(GameTimeChangedBody {
        career_year: 2026,
        career_month: 8,
        career_day: 24,
        revision: 13,
    }));
    roundtrip_event(ServerEvent::ReadyStateChanged(ReadyStateChangedBody {
        manager_id,
        is_ready: true,
        other_manager_ready: false,
    }));

    roundtrip_event(ServerEvent::MatchOpened(MatchOpenedBody {
        match_id,
        fixture_id: "fpl-fixture-2026-08-24-che-ars".into(),
        home_club_id: "fpl-club-che".into(),
        away_club_id: "fpl-club-ars".into(),
    }));
    roundtrip_event(ServerEvent::MatchEventBatch(MatchEventBatchBody {
        match_id,
        from_seq: 0,
        to_seq: 3,
        events: vec![serde_json::json!({"kind": "goal", "minute": 12})],
    }));
    roundtrip_event(ServerEvent::MatchState(MatchStateBody {
        match_id,
        phase: "second_half".into(),
        match_second: 3105,
        home_score: 1,
        away_score: 0,
    }));
    roundtrip_event(ServerEvent::MatchFinished(MatchFinishedBody {
        match_id,
        home_score: 2,
        away_score: 1,
        report: serde_json::json!({"possession": {"home": 55, "away": 45}}),
    }));
    roundtrip_event(ServerEvent::ServerNotice(ServerNoticeBody {
        severity: ServerNoticeSeverity::Warning,
        message_key: "server.maintenance_soon".into(),
    }));
    roundtrip_event(ServerEvent::Ping);
    roundtrip_event(ServerEvent::Pong);
}

#[test]
fn command_envelope_round_trips_and_preserves_expected_revision() {
    let envelope = Envelope {
        protocol_version: CURRENT_VERSIONS.protocol_version,
        message_id: Uuid::new_v4(),
        kind: MessageKind::Command,
        career_id: Uuid::new_v4(),
        manager_id: Uuid::new_v4(),
        expected_revision: Some(42),
        payload: EnvelopePayload::Command(Command::MarkReady(MarkReadyBody {})),
    };
    let json = serde_json::to_string(&envelope).unwrap();
    let parsed: Envelope = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.expected_revision, Some(42));
    assert_eq!(parsed.command_id(), envelope.message_id);
    assert!(matches!(parsed.kind, MessageKind::Command));
}

#[test]
fn version_sets_compare_by_full_equality() {
    let a = CURRENT_VERSIONS.to_owned_set();
    let mut b = a.clone();
    assert!(a.is_compatible_with(&b));
    b.protocol_version += 1;
    assert!(!a.is_compatible_with(&b));
}

#[test]
fn ruleset_version_participates_in_compatibility() {
    let base = CURRENT_VERSIONS.to_owned_set().with_ruleset(RulesetVersion {
        ruleset_id: "england-2026-27-v1".into(),
        ruleset_version: 1,
    });
    let bumped_ruleset = CURRENT_VERSIONS.to_owned_set().with_ruleset(RulesetVersion {
        ruleset_id: "england-2026-27-v1".into(),
        ruleset_version: 2,
    });

    assert!(base.is_compatible_with(&base.clone()));
    assert!(!base.is_compatible_with(&bumped_ruleset));
}

#[test]
fn version_set_without_a_ruleset_still_round_trips() {
    let versions = CURRENT_VERSIONS.to_owned_set();
    assert_eq!(versions.ruleset, None);
    let json = serde_json::to_string(&versions).unwrap();
    let parsed: crate::VersionSet = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, versions);
}

#[test]
fn pre_ruleset_version_set_json_still_deserializes() {
    // Guards forward-compatibility: a VersionSet serialized before the
    // `ruleset` field existed (no field present at all) must still parse,
    // defaulting `ruleset` to None rather than failing.
    let legacy = serde_json::json!({
        "app_version": "0.1.0",
        "protocol_version": 1,
        "save_schema_version": 1,
        "snapshot_schema_version": 1,
        "match_engine_version": "albion-engine-v1",
        "rating_model_version": "albion-rating-v1"
    });
    let parsed: crate::VersionSet = serde_json::from_value(legacy).unwrap();
    assert_eq!(parsed.ruleset, None);
}
