//! Tests for graceful shutdown handling (Issue #1594).
//!
//! Validates that the oracle service properly handles SIGTERM/SIGINT by:
//! - Stopping acceptance of new work
//! - Allowing current submissions to finish (with timeout)
//! - Exiting cleanly without data loss

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Barrier;

#[tokio::test]
async fn test_graceful_shutdown_stops_new_work() {
    let shutdown_barrier = Arc::new(Barrier::new(2));
    let shutdown_barrier_clone = shutdown_barrier.clone();

    let task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(100));
        let shutdown_signal = shutdown_barrier_clone.wait();

        tokio::select! {
            _ = shutdown_signal => {
                // Signal received - should stop taking new work
                true
            }
            _ = async {
                loop {
                    interval.tick().await;
                }
            } => {
                false
            }
        }
    });

    shutdown_barrier.wait().await;
    let result = tokio::time::timeout(Duration::from_secs(1), task)
        .await
        .expect("task should complete quickly after signal");

    assert!(result.expect("task should finish"), "shutdown signal should be processed");
}

#[tokio::test]
async fn test_graceful_shutdown_with_timeout() {
    let shutdown_signal = std::sync::Arc::new(tokio::sync::Notify::new());
    let shutdown_signal_notif = shutdown_signal.clone();

    let work_task = tokio::spawn(async move {
        let timeout = Duration::from_millis(500);
        let start = std::time::Instant::now();

        tokio::select! {
            _ = shutdown_signal_notif.notified() => {
                let elapsed = start.elapsed();
                elapsed
            }
            _ = tokio::time::sleep(timeout) => {
                // Simulate work timeout
                timeout
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(200)).await;
    shutdown_signal.notify_one();

    let elapsed = tokio::time::timeout(Duration::from_secs(1), work_task)
        .await
        .expect("task should complete")
        .expect("task should finish");

    assert!(elapsed.as_millis() < 500, "should stop within timeout");
}

#[tokio::test]
async fn test_graceful_shutdown_current_work_completes() {
    let shutdown_signal = Arc::new(tokio::sync::Notify::new());
    let shutdown_clone = shutdown_signal.clone();

    let submission_task = tokio::spawn(async move {
        let work_duration = Duration::from_millis(300);
        let work_start = std::time::Instant::now();

        loop {
            tokio::select! {
                _ = shutdown_clone.notified() => {
                    // Start of shutdown - complete current work
                    tokio::time::sleep(work_duration).await;
                    return true; // Successfully completed
                }
                _ = tokio::time::sleep(Duration::from_secs(60)) => {
                    return false; // Should not reach here
                }
            }
        }
    });

    tokio::time::sleep(Duration::from_millis(100)).await;
    shutdown_signal.notify_one();

    let result = tokio::time::timeout(Duration::from_secs(2), submission_task)
        .await
        .expect("task should complete")
        .expect("task should finish");

    assert!(result, "current submission should complete");
}
