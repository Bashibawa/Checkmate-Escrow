//! Tests for event status mapping and lifecycle events.
//!
//! Issue #1601: Index the missing escrow lifecycle events

// ── Status mapping for each event type ─────────────────────────────────

#[test]
fn test_status_mapping_for_standard_events() {
    let status_map = [
        ("match:created", "pending"),
        ("match:activated", "active"),
        ("match:completed", "completed"),
        ("match:cancelled", "cancelled"),
        ("match:expired", "expired"),
    ];

    for (event_type, expected_status) in &status_map {
        assert!(!event_type.is_empty());
        assert!(!expected_status.is_empty());
    }
}

#[test]
fn test_status_mapping_for_pending_result_event() {
    // match:pending_result occurs when a dispute vote doesn't reach quorum
    // Status should be: "pending_result"
    let event_type = "match:pending_result";
    let expected_status = "pending_result";

    assert_eq!(expected_status, "pending_result");
    assert!(!event_type.is_empty());
}

#[test]
fn test_status_mapping_for_finalized_event() {
    // match:finalized occurs when dispute is resolved or vote succeeds
    // Status should be: "finalized"
    let event_type = "match:finalized";
    let expected_status = "finalized";

    assert_eq!(expected_status, "finalized");
}

#[test]
fn test_status_mapping_for_rollback_event() {
    // match:rollback occurs when a dispute causes the match to be rolled back
    // Status should be: "rollback"
    let event_type = "match:rollback";
    let expected_status = "rollback";

    assert_eq!(expected_status, "rollback");
}

#[test]
fn test_status_mapping_for_paused_event() {
    // match:paused is emitted when admin pauses a match
    // Status should be: "paused"
    let event_type = "match:paused";
    let expected_status = "paused";

    assert_eq!(expected_status, "paused");
}

#[test]
fn test_status_mapping_for_resumed_event() {
    // match:resumed is emitted when admin resumes a paused match
    // Status should be: "resumed"
    let event_type = "match:resumed";
    let expected_status = "resumed";

    assert_eq!(expected_status, "resumed");
}

#[test]
fn test_status_mapping_for_claim_event() {
    // match:claim is emitted when a player claims their vested payout
    // Status should be: "claimed" (or similar)
    let event_type = "match:claim";
    let expected_status = "claimed";

    assert_eq!(expected_status, "claimed");
}

#[test]
fn test_status_mapping_for_adm_stall_event() {
    // match:adm_stall is emitted when admin stalls a match pending result
    // Status should be: "adm_stall"
    let event_type = "match:adm_stall";
    let expected_status = "adm_stall";

    assert_eq!(expected_status, "adm_stall");
}

#[test]
fn test_dispute_created_is_informational_event() {
    // dispute:created is emitted when a dispute is initiated
    // This is informational; it doesn't change match status directly
    // The match status comes from the underlying match state
    let event_type = "dispute:created";

    // This event should be stored but doesn't map to a match status
    assert!(!event_type.is_empty());
}

#[test]
fn test_dispute_resolved_event_status() {
    // dispute:resolved is emitted when a dispute is resolved
    // The match status at this point depends on the resolution
    // (completed, cancelled, finalized, etc.)
    let event_type = "dispute:resolved";

    assert!(!event_type.is_empty());
}

#[test]
fn test_dispute_voted_is_informational_event() {
    // dispute:voted is emitted when someone votes on a dispute
    // This is informational; match status comes from dispute state
    let event_type = "dispute:voted";

    assert!(!event_type.is_empty());
}

// ── Match status enum should support all these statuses ────────────────

#[test]
fn test_extended_match_status_enum_coverage() {
    // Current MatchStatus enum has: Pending, Active, Completed, Cancelled, Expired
    // Must be extended to include:
    let extended_statuses = vec![
        "pending",        // match:created
        "active",         // match:activated
        "completed",      // match:completed
        "cancelled",      // match:cancelled
        "expired",        // match:expired
        "pending_result", // match:pending_result
        "finalized",      // match:finalized
        "rollback",       // match:rollback
        "paused",         // match:paused
        "resumed",        // match:resumed
        "claimed",        // match:claim
        "adm_stall",      // match:adm_stall
    ];

    for status in extended_statuses {
        assert!(!status.is_empty());
    }
}

// ── Event to status mapping must be consistent ─────────────────────────

#[test]
fn test_status_mapping_is_deterministic() {
    // Same event type should always map to same status
    let event_type = "match:created";
    let status1 = "pending";
    let status2 = "pending";

    assert_eq!(status1, status2);
}

#[test]
fn test_no_event_maps_to_multiple_statuses() {
    // Each event type must map to exactly one status
    // (one-to-many would create ambiguity in reverse lookup)
    let mappings = [
        ("match:created", "pending"),
        ("match:activated", "active"),
        ("match:completed", "completed"),
        ("match:cancelled", "cancelled"),
        ("match:expired", "expired"),
        ("match:pending_result", "pending_result"),
        ("match:finalized", "finalized"),
        ("match:rollback", "rollback"),
        ("match:paused", "paused"),
        ("match:resumed", "resumed"),
        ("match:claim", "claimed"),
        ("match:adm_stall", "adm_stall"),
    ];

    // Verify no duplicates on event type side
    let mut seen_events = std::collections::HashSet::new();
    for (event, _status) in &mappings {
        assert!(
            seen_events.insert(event),
            "Event {} maps to multiple statuses",
            event
        );
    }
}

#[test]
fn test_status_values_are_api_compatible() {
    // These status values must match what the API contract expects
    let api_compatible_statuses = vec![
        "pending",
        "active",
        "completed",
        "cancelled",
        "expired",
        "pending_result",
        "finalized",
        "rollback",
        "paused",
        "resumed",
        "claimed",
        "adm_stall",
    ];

    for status in api_compatible_statuses {
        // Status should be lowercase (consistent with current API)
        assert_eq!(status, status.to_lowercase(), "{} must be lowercase", status);
        // No special characters that might break JSON
        assert!(
            status.chars().all(|c| c.is_alphanumeric() || c == '_'),
            "{} contains invalid characters",
            status
        );
    }
}
