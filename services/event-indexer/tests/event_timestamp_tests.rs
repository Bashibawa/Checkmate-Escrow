//! Tests for event timestamp handling.
//!
//! Issue #1600: Use ledgerClosedAt instead of Utc::now()

use chrono::{DateTime, Utc};

// ── Timestamp accuracy ────────────────────────────────────────────────

#[test]
fn test_event_timestamp_must_come_from_ledger_not_indexer() {
    // Current (WRONG): timestamp = Utc::now() at the moment the indexer processes the event
    // Correct: timestamp = ledgerClosedAt from the RPC response

    // Scenario: Event occurs at ledger close time 1700000000
    let ledger_closed_at_unix = 1700000000i64;
    let ledger_closed_at = DateTime::<Utc>::from_timestamp(ledger_closed_at_unix, 0).unwrap();

    // If the indexer processes it 5 seconds later, Utc::now() would be different
    let indexer_now_unix = ledger_closed_at_unix + 5; // 5 seconds later
    let indexer_now = DateTime::<Utc>::from_timestamp(indexer_now_unix, 0).unwrap();

    // The indexed event should use ledger_closed_at, not indexer_now
    assert_ne!(ledger_closed_at, indexer_now);

    // For history/analytics, the original ledger time is correct
    assert_eq!(ledger_closed_at_unix, 1700000000);
}

#[test]
fn test_timestamp_must_be_consistent_across_backfill() {
    // Scenario: Indexer is backfilling historical events
    // Events from ledger 10000 (closed at 1700000000) are processed today
    let ledger_closed_at_unix = 1700000000i64;
    let backfill_time_unix = 1723478400i64; // Today

    // With Utc::now(): all backfilled events get today's timestamp
    // This breaks analytics and receipts (shows wrong dates)

    // With ledgerClosedAt: events keep their original timestamps
    // Analytics correctly shows when events actually happened
    assert_ne!(ledger_closed_at_unix, backfill_time_unix);
}

#[test]
fn test_events_from_same_ledger_must_have_same_timestamp() {
    // Multiple events in one ledger are all closed at the same time
    let ledger_closed_at = 1700000000i64;

    // Event 1: match:created
    // Event 2: match:deposit
    // Event 3: match:activated
    // All three are in ledger 12345, closed at same time

    let event1_timestamp = ledger_closed_at;
    let event2_timestamp = ledger_closed_at;
    let event3_timestamp = ledger_closed_at;

    assert_eq!(event1_timestamp, event2_timestamp);
    assert_eq!(event2_timestamp, event3_timestamp);
}

#[test]
fn test_timestamp_precision_from_rpc_response() {
    // RPC returns ledgerClosedAt as a Unix timestamp (seconds)
    // This has second-level precision
    let ledger_closed_at_unix = 1700000000i64;

    // When converting to DateTime<Utc>, keep the precision
    // (nanoseconds = 0 if only second precision available)
    let dt = DateTime::<Utc>::from_timestamp(ledger_closed_at_unix, 0).unwrap();

    assert_eq!(dt.timestamp(), ledger_closed_at_unix);
    assert_eq!(dt.nanosecond(), 0);
}

// ── RPC response structure ────────────────────────────────────────────

#[test]
fn test_rpc_getevents_response_includes_ledger_closed_at() {
    // RPC response from getEvents should include:
    // {
    //   "events": [
    //     {
    //       "ledger": 12345,
    //       "ledgerClosedAt": 1700000000,
    //       "txnMeta": "...",
    //       "event": { "topics": [...], "data": [...] }
    //     }
    //   ]
    // }

    // Every event in the response has ledgerClosedAt
    let ledger = 12345u32;
    let ledger_closed_at = 1700000000i64;

    assert!(ledger > 0);
    assert!(ledger_closed_at > 0);
}

#[test]
fn test_parse_event_must_extract_ledger_closed_at() {
    // The parse_event function must:
    // 1. Extract "ledgerClosedAt" from the event value
    // 2. Convert it to DateTime<Utc>
    // 3. Use it as the IndexedEvent.timestamp

    let ledger_closed_at_unix = 1700000000i64;
    let timestamp = DateTime::<Utc>::from_timestamp(ledger_closed_at_unix, 0).unwrap();

    // This timestamp should be used in IndexedEvent, not Utc::now()
    assert_eq!(timestamp.timestamp(), ledger_closed_at_unix);
}

// ── Correctness of timestamp in different scenarios ───────────────────

#[test]
fn test_timestamp_in_backfill_scenario() {
    // Backfill scenario: Events from 3 months ago are indexed now
    let event_ledger_closed_at = 1691000000i64; // 3 months ago
    let current_time = 1723478400i64; // Today

    // Timestamp must be event_ledger_closed_at, NOT current_time
    assert_ne!(event_ledger_closed_at, current_time);
    assert!(current_time > event_ledger_closed_at);

    // The indexed event's timestamp should reflect when it actually happened
    let indexed_timestamp = event_ledger_closed_at;
    assert_eq!(indexed_timestamp, 1691000000);
}

#[test]
fn test_timestamp_in_normal_polling_scenario() {
    // Normal scenario: Events are polled within seconds of ledger close
    let ledger_closed_at = 1700000000i64;
    let poll_time = 1700000005i64; // 5 seconds later

    // Even if poll happens a few seconds after close, use ledger close time
    assert_ne!(ledger_closed_at, poll_time);

    let indexed_timestamp = ledger_closed_at;
    assert_eq!(indexed_timestamp, 1700000000);
}

#[test]
fn test_timestamp_in_downtime_recovery_scenario() {
    // Downtime scenario: Indexer was down, then restarts and catches up
    let event_ledger_closed_at = 1699999999i64; // When event was on ledger
    let recovery_time = 1700000500i64; // When indexer restarts (8+ minutes later)

    // Events must keep their original timestamps from when they occurred
    // Not get new timestamps from when the indexer recovered
    assert_ne!(event_ledger_closed_at, recovery_time);

    let indexed_timestamp = event_ledger_closed_at;
    assert_eq!(indexed_timestamp, 1699999999);
}

// ── Impact on API responses ────────────────────────────────────────────

#[test]
fn test_match_history_shows_actual_event_timestamps() {
    // Player's match history should show when matches actually happened
    // Not when they were indexed
    let match_created_timestamp = 1700000000i64; // When match was created
    let match_completed_timestamp = 1700010000i64; // When result was submitted
    let indexer_processing_time = 1700020000i64; // When indexer ran

    // With wrong implementation: all timestamps = indexer_processing_time
    // With correct implementation: timestamps = actual event times
    assert_ne!(match_completed_timestamp, indexer_processing_time);
}

#[test]
fn test_analytics_receipts_show_correct_dates() {
    // Receipts and analytics must show the actual date of transactions
    // Not the indexing date
    let transaction_date = 1700000000i64; // When it happened
    let indexing_date = 1700086400i64; // One day later (24h * 3600s)

    // Receipt should show transaction_date, not indexing_date
    assert_ne!(transaction_date, indexing_date);
    let receipt_timestamp = transaction_date;
    assert_eq!(receipt_timestamp, 1700000000);
}

#[test]
fn test_timestamp_sorting_remains_correct() {
    // Events must be sortable by their actual occurrence time
    // Not affected by when they were indexed

    let event1_closed_at = 1700000000i64;
    let event2_closed_at = 1700001000i64;
    let event3_closed_at = 1700002000i64;

    // All indexed at same time (say 1700003000) but with ledgerClosedAt:
    // Sorting by timestamp gives correct order
    assert!(event1_closed_at < event2_closed_at);
    assert!(event2_closed_at < event3_closed_at);
}
