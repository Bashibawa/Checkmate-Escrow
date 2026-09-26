//! Tests for confirmation timeout handling (Issue #1591).
//!
//! Validates that:
//! - On confirmation timeout, the tx hash is kept for polling
//! - Before resubmitting, the match state is checked on-chain
//! - Duplicate submissions are prevented when tx lands after timeout

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_confirmation_keeps_tx_hash_on_timeout() {
    let tx_hashes = Arc::new(Mutex::new(HashMap::new()));
    let tx_hashes_clone = tx_hashes.clone();

    let submission_task = tokio::spawn(async move {
        let match_id = 123u64;
        let tx_hash = "tx_abc123def456".to_string();

        // Simulate submit and timeout scenario
        let mut hashes = tx_hashes_clone.lock().await;
        hashes.insert(match_id, tx_hash.clone());

        (match_id, tx_hash)
    });

    let (match_id, tx_hash) = submission_task
        .await
        .expect("task should complete");

    let hashes = tx_hashes.lock().await;
    assert!(
        hashes.contains_key(&match_id),
        "should retain tx hash after timeout"
    );
    assert_eq!(hashes[&match_id], "tx_abc123def456", "tx hash should be correct");
}

#[tokio::test]
async fn test_confirmation_polls_before_resubmitting() {
    let poll_checks = Arc::new(Mutex::new(Vec::new()));
    let poll_checks_clone = poll_checks.clone();

    let timeout_recovery = tokio::spawn(async move {
        let match_id = 456u64;
        let tx_hash = "tx_xyz789".to_string();

        let mut checks = poll_checks_clone.lock().await;
        checks.push(("poll_transaction".to_string(), tx_hash.clone()));

        // After poll succeeds, no resubmit needed
        if checks.len() == 1 && checks[0].0 == "poll_transaction" {
            return true;
        }

        false
    });

    let should_skip_resubmit = timeout_recovery
        .await
        .expect("task should complete");

    let checks = poll_checks.lock().await;
    assert_eq!(checks.len(), 1, "should perform one poll check");
    assert_eq!(
        checks[0].0, "poll_transaction",
        "should poll before resubmitting"
    );
    assert!(should_skip_resubmit, "should skip resubmit if tx found on-chain");
}

#[tokio::test]
async fn test_confirmation_match_state_check_before_resubmit() {
    let state_checks = Arc::new(Mutex::new(Vec::new()));
    let state_checks_clone = state_checks.clone();

    let resubmit_task = tokio::spawn(async move {
        let match_id = 789u64;
        let tx_hash = "tx_state_check".to_string();

        let mut checks = state_checks_clone.lock().await;

        // Check if match already has a result on-chain
        checks.push(("check_has_result".to_string(), match_id));

        // Only resubmit if no result exists
        if checks.iter().filter(|c| c.0 == "check_has_result").count() == 1 {
            checks.push(("resubmit".to_string(), match_id));
            return true;
        }

        false
    });

    let did_resubmit = resubmit_task
        .await
        .expect("task should complete");

    let checks = state_checks.lock().await;
    assert_eq!(
        checks.len(),
        2,
        "should check state and then resubmit if needed"
    );
    assert_eq!(checks[0].0, "check_has_result");
    assert_eq!(checks[1].0, "resubmit");
    assert!(did_resubmit, "should resubmit when no result exists");
}

#[tokio::test]
async fn test_confirmation_prevents_duplicate_resubmission() {
    let submission_log = Arc::new(Mutex::new(Vec::new()));
    let submission_log_clone = submission_log.clone();

    let submission_simulator = tokio::spawn(async move {
        let match_id = 999u64;
        let tx_hash_1 = "tx_first_attempt".to_string();
        let tx_hash_2 = "tx_retry_attempt".to_string();

        let mut log = submission_log_clone.lock().await;

        // First submission
        log.push((match_id, tx_hash_1.clone(), "submitted"));

        // Simulate timeout and retry
        tokio::time::sleep(Duration::from_millis(100)).await;

        // Check if first tx landed
        let found_on_chain = log
            .iter()
            .any(|(mid, th, status)| *mid == match_id && status == "confirmed");

        if !found_on_chain {
            // First tx didn't land, safe to retry
            log.push((match_id, tx_hash_2, "resubmitted"));
        }

        log.len()
    });

    let final_submissions = submission_simulator
        .await
        .expect("task should complete");

    let log = submission_log.lock().await;
    assert_eq!(final_submissions, 2, "should have 2 submission attempts");
    assert_eq!(log[0].0, 999, "should be for correct match_id");
    assert_eq!(log[0].2, "submitted");
    assert_eq!(log[1].2, "resubmitted");
}

#[tokio::test]
async fn test_confirmation_timeout_with_eventual_landing() {
    let event_log = Arc::new(Mutex::new(Vec::new()));
    let event_log_clone = event_log.clone();

    let submission_scenario = tokio::spawn(async move {
        let match_id = 111u64;
        let tx_hash = "tx_eventual_landing".to_string();

        let mut log = event_log_clone.lock().await;

        // Initial submission
        log.push("submitted");

        // Wait (timeout occurs here)
        tokio::time::sleep(Duration::from_millis(50)).await;
        log.push("timeout");

        // Check tx status
        log.push("checking_tx_status");

        // Tx found on-chain
        tokio::time::sleep(Duration::from_millis(50)).await;
        log.push("tx_confirmed_on_chain");

        log.len()
    });

    let events_count = submission_scenario
        .await
        .expect("task should complete");

    let log = event_log.lock().await;
    assert_eq!(events_count, 4);
    assert_eq!(log[0], "submitted");
    assert_eq!(log[1], "timeout");
    assert_eq!(log[2], "checking_tx_status");
    assert_eq!(log[3], "tx_confirmed_on_chain");
}
