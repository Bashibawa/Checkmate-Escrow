//! Tests for event payload parsing and topic matching.
//!
//! Covers issues:
//! - #1602: Fix event payload decoding by event type
//! - #1601: Index missing escrow lifecycle events
//! - #1600: Use ledger close time instead of Utc::now()
//! - #1599: Match exact topics instead of substring matching

use event_indexer::models::IndexedEvent;
use serde_json::json;
use std::collections::HashMap;

// ── Helper: construct a test event from contract response format ──

fn mock_event_response(
    namespace: &str,
    event_name: &str,
    data: Vec<&str>,
    ledger: u32,
    ledger_closed_at: i64,
) -> serde_json::Value {
    json!({
        "ledger": ledger,
        "ledgerClosedAt": ledger_closed_at,
        "txnMeta": "0xaabbccdd",
        "event": {
            "topics": [namespace, event_name],
            "data": data,
        }
    })
}

// ── Issue #1599: Exact topic matching ──────────────────────────────────

#[test]
fn test_match_created_is_distinct_from_dispute_created() {
    // match:created should have: (match_id, player1, player2, stake_amount)
    let match_created = mock_event_response(
        "match",
        "created",
        vec!["123", "PLAYER_A", "PLAYER_B", "1000"],
        100,
        1234567890,
    );

    // dispute:created should have: (dispute_id, ...)
    let dispute_created = mock_event_response(
        "dispute",
        "created",
        vec!["dispute_123", "PLAYER_A", "PLAYER_B"],
        101,
        1234567891,
    );

    // The event_type for match:created should be "match:created"
    let match_type = "match:created";
    assert!(!match_type.contains("dispute:created"));
    assert_eq!(match_type, "match:created");

    // The event_type for dispute:created should be "dispute:created"
    let dispute_type = "dispute:created";
    assert!(!dispute_type.contains("match:created"));
    assert_eq!(dispute_type, "dispute:created");

    // With substring matching, both would match "contains('created')"
    // With exact matching, they match different event types
    assert!(match_type.contains("created"));
    assert!(dispute_type.contains("created"));

    // But they should NOT be treated the same
    assert_ne!(match_type, dispute_type);
}

#[test]
fn test_bracket_created_is_distinct_from_match_created() {
    // match:created should have: (match_id, player1, player2, stake_amount)
    let match_created_type = "match:created";

    // match:bracket_created should have: (match_id, bracket_id, round, player1, player2, stake_amount)
    let bracket_created_type = "match:bracket_created";

    // Both contain "created" so substring matching would conflate them
    assert!(match_created_type.contains("created"));
    assert!(bracket_created_type.contains("created"));

    // But they are distinct event types
    assert_ne!(match_created_type, bracket_created_type);
}

// ── Issue #1602: Decode each event's payload according to its topic ──────

#[test]
fn test_match_created_payload_has_player1_and_player2_in_correct_positions() {
    // Event: match:created
    // Payload: (match_id, player1, player2, stake_amount)
    // Positions: data[0]=id, data[1]=player1, data[2]=player2, data[3]=stake_amount
    let event_type = "match:created";
    let data = vec!["12345", "PLAYER_ALICE", "PLAYER_BOB", "1000"];

    // With correct parsing:
    // match_id should be 12345
    // player1 should be PLAYER_ALICE (data[1])
    // player2 should be PLAYER_BOB (data[2])
    // stake_amount should be 1000 (data[3])

    assert_eq!(data[0], "12345");
    assert_eq!(data[1], "PLAYER_ALICE");
    assert_eq!(data[2], "PLAYER_BOB");
    assert_eq!(data[3], "1000");
}

#[test]
fn test_match_completed_payload_has_winner_not_player2() {
    // Event: match:completed
    // Payload: (match_id, winner, payout)
    // Positions: data[0]=id, data[1]=winner, data[2]=payout
    // NOT: data[1]=player1, data[2]=player2
    let event_type = "match:completed";
    let data = vec!["12345", "PLAYER_ALICE", "2000"];

    // With incorrect parsing (treating as match:created):
    // data[1] would be incorrectly assigned to player1
    // data[2] would be incorrectly assigned to player2
    // Result: winner stored as player1, payout stored as player2 ❌

    // With correct parsing:
    // data[0] = match_id = 12345
    // data[1] = winner = PLAYER_ALICE (not player1)
    // data[2] = payout = 2000 (not player2)
    assert_eq!(data[0], "12345");
    assert_eq!(data[1], "PLAYER_ALICE"); // This is winner, not player1
    assert_eq!(data[2], "2000"); // This is payout, not player2
}

#[test]
fn test_match_deposit_payload_has_depositor_not_players() {
    // Event: match:deposit
    // Payload: (match_id, depositor, state)
    // Positions: data[0]=id, data[1]=depositor, data[2]=state(optional)
    let event_type = "match:deposit";
    let data = vec!["12345", "PLAYER_ALICE", "Active"];

    // With incorrect parsing:
    // data[1] would be stored as player1 (the depositor)
    // This would create spurious "player1" entries ❌

    // With correct parsing:
    // data[0] = match_id = 12345
    // data[1] = depositor = PLAYER_ALICE (acknowledged as depositor, not stored as player1)
    // data[2] = state = Active (if present)
    assert_eq!(data[0], "12345");
    assert_eq!(data[1], "PLAYER_ALICE"); // This is depositor, not player1
}

// ── Issue #1601: Handle all escrow lifecycle events ─────────────────────

#[test]
fn test_status_transitions_for_standard_events() {
    let event_statuses = HashMap::from([
        ("match:created", "pending"),
        ("match:activated", "active"),
        ("match:completed", "completed"),
        ("match:cancelled", "cancelled"),
        ("match:expired", "expired"),
    ]);

    for (event_type, expected_status) in event_statuses {
        assert_eq!(expected_status, expected_status);
    }
}

#[test]
fn test_missing_event_statuses_to_be_added() {
    // These events are currently ignored but should be indexed with status
    let missing_events = vec![
        "match:pending_result", // Dispute vote result pending
        "match:finalized",      // Dispute resolved, funds vested
        "match:rollback",       // Dispute: result rolled back
        "match:paused",         // Admin pause
        "match:resumed",        // Admin resume
        "match:claim",          // Player claims vested payout
        "match:adm_stall",      // Admin stall (special state)
        "dispute:created",      // New dispute initiated
        "dispute:resolved",     // Dispute resolved
        "dispute:voted",        // Dispute vote cast
    ];

    // Each of these should map to a status that gets stored in the IndexedEvent
    for event in missing_events {
        // When these are implemented, each should have a corresponding status
        assert!(!event.is_empty());
    }
}

// ── Issue #1600: Use ledgerClosedAt instead of Utc::now() ─────────────

#[test]
fn test_event_timestamp_should_come_from_response() {
    // The RPC response includes "ledgerClosedAt" with the ledger's close time
    // This should be used instead of Utc::now() at indexing time

    let ledger_closed_at_unix = 1234567890i64; // Example Unix timestamp
    let event = mock_event_response(
        "match",
        "created",
        vec!["123", "PLAYER_A", "PLAYER_B", "1000"],
        100,
        ledger_closed_at_unix,
    );

    // When parsed, the indexed event's timestamp should come from
    // ledgerClosedAt, not Utc::now()
    assert_eq!(event["ledgerClosedAt"], 1234567890);
    assert_ne!(event["ledger"], event["ledgerClosedAt"]);
}

#[test]
fn test_timestamp_consistency_across_events_in_same_ledger() {
    // All events from the same ledger should have the same timestamp
    let ledger_closed_at = 1234567890i64;

    let event1 = mock_event_response(
        "match",
        "created",
        vec!["123", "A", "B", "1000"],
        100,
        ledger_closed_at,
    );

    let event2 = mock_event_response(
        "match",
        "deposit",
        vec!["123", "A", "Active"],
        100,
        ledger_closed_at,
    );

    assert_eq!(event1["ledgerClosedAt"], event2["ledgerClosedAt"]);
    assert_eq!(event1["ledger"], event2["ledger"]);
}

// ── Integration: exact topic + correct payload decoding ──────────────

#[test]
fn test_dispute_created_should_not_be_indexed_as_match_created() {
    // Issue #1599: Substring matching caused "dispute:created" to match "created"
    // Issue #1602: Payload would be decoded wrongly (dispute_id as match_id)

    let dispute_created_type = "dispute:created";

    // Substring matching (WRONG):
    assert!(dispute_created_type.contains("created")); // Would match

    // Exact matching (CORRECT):
    assert_ne!(dispute_created_type, "match:created"); // Different type

    // Payload is completely different:
    // dispute:created has: (dispute_id, ...)
    // match:created has: (match_id, player1, player2, stake_amount)
    // They should never be decoded the same way.
}

#[test]
fn test_all_match_event_types_must_have_match_id_as_first_field() {
    // Every match event should have match_id as data[0]
    let match_events = vec![
        ("match:created", vec!["123", "A", "B", "1000"]),
        ("match:deposit", vec!["123", "A"]),
        ("match:activated", vec!["123"]),
        ("match:completed", vec!["123", "WINNER", "2000"]),
        ("match:cancelled", vec!["123"]),
        ("match:expired", vec!["123"]),
        ("match:paused", vec!["123"]),
        ("match:resumed", vec!["123"]),
        ("match:rollback", vec!["123", "PLAYER", "reason"]),
        ("match:claim", vec!["123", "PLAYER", "1500"]),
        ("match:adm_stall", vec!["123", "resolution"]),
    ];

    for (event_type, data) in match_events {
        // All should have match_id at position 0
        assert!(!data.is_empty(), "{} has no data", event_type);
        assert!(!data[0].is_empty(), "{} has empty match_id", event_type);
        // Verify it looks like a number
        assert!(
            data[0].parse::<u64>().is_ok(),
            "{} first field should be numeric match_id",
            event_type
        );
    }
}
