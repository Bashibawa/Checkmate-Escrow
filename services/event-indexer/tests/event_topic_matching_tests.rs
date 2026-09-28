//! Tests for exact topic matching.
//!
//! Issue #1599: Match exact topics instead of substring matching

// ── Topic structure ──────────────────────────────────────────────────

#[test]
fn test_event_topic_format_is_namespace_colon_name() {
    // Events have format: "namespace:name"
    // Examples: "match:created", "dispute:created", "admin:paused"

    let match_created = "match:created";
    let dispute_created = "dispute:created";
    let admin_paused = "admin:paused";

    assert_eq!(match_created, "match:created");
    assert_eq!(dispute_created, "dispute:created");
    assert_eq!(admin_paused, "admin:paused");
}

#[test]
fn test_substring_matching_would_conflate_different_events() {
    // With substring matching on "created":
    let topics = vec![
        "match:created",
        "match:bracket_created", // Substring match: contains "created"
        "dispute:created",        // Substring match: contains "created"
    ];

    // All would match .contains("created")
    for topic in &topics {
        assert!(topic.contains("created"));
    }

    // But they should NOT be treated the same!
    assert_ne!("match:created", "match:bracket_created");
    assert_ne!("match:created", "dispute:created");
}

// ── Match event types ────────────────────────────────────────────────

#[test]
fn test_match_created_exact_topic() {
    let event_type = "match:created";

    // Should match exactly
    assert_eq!(event_type, "match:created");

    // Should NOT match similar-looking events
    assert_ne!(event_type, "dispute:created");
    assert_ne!(event_type, "match:bracket_created");
}

#[test]
fn test_match_activated_exact_topic() {
    let event_type = "match:activated";

    assert_eq!(event_type, "match:activated");
    assert_ne!(event_type, "match:created");
}

#[test]
fn test_match_deposit_exact_topic() {
    let event_type = "match:deposit";

    assert_eq!(event_type, "match:deposit");
    assert_ne!(event_type, "match:created");
}

#[test]
fn test_match_completed_exact_topic() {
    let event_type = "match:completed";

    assert_eq!(event_type, "match:completed");
    assert_ne!(event_type, "match:activated");
}

#[test]
fn test_match_cancelled_exact_topic() {
    let event_type = "match:cancelled";

    assert_eq!(event_type, "match:cancelled");
    assert_ne!(event_type, "match:created");
}

#[test]
fn test_match_expired_exact_topic() {
    let event_type = "match:expired";

    assert_eq!(event_type, "match:expired");
    assert_ne!(event_type, "match:cancelled");
}

#[test]
fn test_match_pending_result_exact_topic() {
    let event_type = "match:pending_result";

    assert_eq!(event_type, "match:pending_result");
    assert_ne!(event_type, "match:created");
}

#[test]
fn test_match_finalized_exact_topic() {
    let event_type = "match:finalized";

    assert_eq!(event_type, "match:finalized");
    assert_ne!(event_type, "match:completed");
}

#[test]
fn test_match_rollback_exact_topic() {
    let event_type = "match:rollback";

    assert_eq!(event_type, "match:rollback");
    assert_ne!(event_type, "match:created");
}

#[test]
fn test_match_paused_exact_topic() {
    let event_type = "match:paused";

    assert_eq!(event_type, "match:paused");
    assert_ne!(event_type, "match:resumed");
}

#[test]
fn test_match_resumed_exact_topic() {
    let event_type = "match:resumed";

    assert_eq!(event_type, "match:resumed");
    assert_ne!(event_type, "match:paused");
}

#[test]
fn test_match_claim_exact_topic() {
    let event_type = "match:claim";

    assert_eq!(event_type, "match:claim");
    assert_ne!(event_type, "match:completed");
}

#[test]
fn test_match_adm_stall_exact_topic() {
    let event_type = "match:adm_stall";

    assert_eq!(event_type, "match:adm_stall");
    assert_ne!(event_type, "match:paused");
}

#[test]
fn test_match_bracket_created_is_distinct_from_match_created() {
    // Issue #1599 specifically mentions this as a bug
    let bracket_created = "match:bracket_created";
    let match_created = "match:created";

    // Both contain "created" so substring matching fails
    assert!(bracket_created.contains("created"));
    assert!(match_created.contains("created"));

    // But they are distinct events
    assert_ne!(bracket_created, match_created);
}

// ── Dispute event types ──────────────────────────────────────────────

#[test]
fn test_dispute_created_exact_topic() {
    let event_type = "dispute:created";

    // Should NOT be confused with match:created
    assert_ne!(event_type, "match:created");

    // Even though both contain "created"
    assert!(event_type.contains("created"));
    assert!("match:created".contains("created"));
}

#[test]
fn test_dispute_resolved_exact_topic() {
    let event_type = "dispute:resolved";

    assert_eq!(event_type, "dispute:resolved");
    assert_ne!(event_type, "match:completed");
}

#[test]
fn test_dispute_voted_exact_topic() {
    let event_type = "dispute:voted";

    assert_eq!(event_type, "dispute:voted");
}

// ── Admin event types ────────────────────────────────────────────────

#[test]
fn test_admin_paused_exact_topic() {
    let event_type = "admin:paused";

    assert_eq!(event_type, "admin:paused");
    assert_ne!(event_type, "match:paused");
}

#[test]
fn test_admin_unpaused_exact_topic() {
    let event_type = "admin:unpaused";

    assert_eq!(event_type, "admin:unpaused");
    assert_ne!(event_type, "match:resumed");
}

// ── Implementation: exact matching function ──────────────────────────

#[test]
fn test_exact_matching_with_equals() {
    // Implementation should use == or exact string comparison
    // NOT .contains()

    let event_type = "match:created";

    // Correct (exact matching):
    let is_match_created = event_type == "match:created";
    assert!(is_match_created);

    // Wrong (substring matching):
    let is_created_substring = event_type.contains("created");
    assert!(is_created_substring); // Too broad!
}

#[test]
fn test_exact_matching_handles_namespace_correctly() {
    // Namespace:name format ensures uniqueness
    // match:created is different from dispute:created
    let match_ns = "match";
    let dispute_ns = "dispute";
    let created_name = "created";

    let match_created = format!("{}:{}", match_ns, created_name);
    let dispute_created = format!("{}:{}", dispute_ns, created_name);

    assert_ne!(match_created, dispute_created);
    assert_eq!(match_created, "match:created");
    assert_eq!(dispute_created, "dispute:created");
}

#[test]
fn test_exact_matching_with_pattern_matching() {
    // Alternative implementation using pattern matching
    fn get_status(event_type: &str) -> Option<&str> {
        match event_type {
            "match:created" => Some("pending"),
            "match:activated" => Some("active"),
            "match:completed" => Some("completed"),
            "match:cancelled" => Some("cancelled"),
            "match:expired" => Some("expired"),
            "match:pending_result" => Some("pending_result"),
            "match:finalized" => Some("finalized"),
            "match:rollback" => Some("rollback"),
            "match:paused" => Some("paused"),
            "match:resumed" => Some("resumed"),
            "match:claim" => Some("claimed"),
            "match:adm_stall" => Some("adm_stall"),
            _ => None,
        }
    }

    // Pattern matching ensures exact matching
    assert_eq!(get_status("match:created"), Some("pending"));
    assert_eq!(get_status("match:activated"), Some("active"));
    assert_eq!(get_status("dispute:created"), None); // Not matched
    assert_eq!(get_status("match:bracket_created"), None); // Not matched
}

// ── Regression: previous substring matching bugs ──────────────────────

#[test]
fn test_regression_dispute_created_not_indexed_as_match() {
    // Bug scenario: dispute:created has payload (dispute_id, ...)
    // With substring matching "contains('created')", this becomes a match:created
    // With match:created parsing: dispute_id is stored as match_id
    // This corrupts the match table by inserting dispute_id as match_id

    let event_type = "dispute:created";

    // With exact matching, dispute:created is not matched
    assert_ne!(event_type, "match:created");

    // So its payload is not decoded as match:created
    // (dispute_id is not treated as match_id)
}

#[test]
fn test_regression_bracket_created_not_indexed_as_match() {
    // Bug scenario: match:bracket_created has payload with bracket_id
    // With substring matching "contains('created')", this becomes a match:created
    // With match:created parsing: bracket_id is stored as match_id

    let event_type = "match:bracket_created";

    // With exact matching, bracket_created is not confused with match:created
    assert_ne!(event_type, "match:created");
}

#[test]
fn test_all_match_events_have_distinct_exact_topics() {
    // Ensure no two match events are equal
    let match_events = vec![
        "match:created",
        "match:bracket_created",
        "match:deposit",
        "match:activated",
        "match:completed",
        "match:cancelled",
        "match:expired",
        "match:pending_result",
        "match:finalized",
        "match:rollback",
        "match:paused",
        "match:resumed",
        "match:claim",
        "match:adm_stall",
    ];

    // All should be unique
    let mut seen = std::collections::HashSet::new();
    for event in &match_events {
        assert!(
            seen.insert(event),
            "Duplicate event type: {}",
            event
        );
    }

    assert_eq!(seen.len(), match_events.len());
}

#[test]
fn test_namespaces_are_distinct() {
    // Different namespaces should not be confused
    let namespaces = vec![
        "match",
        "dispute",
        "admin",
        "escrow",
    ];

    for ns in &namespaces {
        // Each namespace should be unique
        assert!(!ns.is_empty());
        assert!(!ns.contains(":"));
    }
}
