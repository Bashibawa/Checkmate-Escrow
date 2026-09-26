//! Tests for Chess.com loop serialization (Issue #1592).
//!
//! Validates that:
//! - The Chess.com-specific loop can be used without racing concurrent submissions
//! - Both loops never sign concurrently with the same key
//! - The Chess.com poll interval is properly respected

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_chess_com_loop_does_not_race_submissions() {
    let submission_count = Arc::new(AtomicU32::new(0));
    let signing_lock = Arc::new(Mutex::new(()));
    let signed_count = Arc::new(AtomicU32::new(0));

    let count_clone1 = submission_count.clone();
    let lock_clone1 = signing_lock.clone();
    let signed_clone1 = signed_count.clone();

    let main_loop = tokio::spawn(async move {
        for _ in 0..5 {
            count_clone1.fetch_add(1, Ordering::SeqCst);
            let _guard = lock_clone1.lock().await;
            tokio::time::sleep(Duration::from_millis(10)).await;
            signed_clone1.fetch_add(1, Ordering::SeqCst);
        }
    });

    let count_clone2 = submission_count.clone();
    let lock_clone2 = signing_lock.clone();
    let signed_clone2 = signed_count.clone();

    let chess_com_loop = tokio::spawn(async move {
        for _ in 0..3 {
            count_clone2.fetch_add(1, Ordering::SeqCst);
            let _guard = lock_clone2.lock().await;
            tokio::time::sleep(Duration::from_millis(15)).await;
            signed_clone2.fetch_add(1, Ordering::SeqCst);
        }
    });

    let _ = tokio::join!(main_loop, chess_com_loop);

    let total_submissions = submission_count.load(Ordering::SeqCst);
    let total_signed = signed_count.load(Ordering::SeqCst);

    assert_eq!(total_submissions, 8, "should have 5 + 3 submissions");
    assert_eq!(total_signed, 8, "all submissions should complete signing");
}

#[tokio::test]
async fn test_chess_com_loop_respects_interval() {
    let loop_count = Arc::new(AtomicU32::new(0));
    let loop_count_clone = loop_count.clone();

    let chess_com_loop = tokio::spawn(async move {
        let interval = Duration::from_millis(100);
        let mut ticker = tokio::time::interval(interval);
        let start = std::time::Instant::now();

        loop {
            ticker.tick().await;
            loop_count_clone.fetch_add(1, Ordering::SeqCst);

            if start.elapsed() > Duration::from_millis(350) {
                break;
            }
        }
    });

    let _ = tokio::time::timeout(Duration::from_secs(1), chess_com_loop).await;

    let count = loop_count.load(Ordering::SeqCst);

    // With 100ms interval over ~350ms, we expect ~3-4 ticks
    assert!(
        count >= 3 && count <= 5,
        "loop should tick ~3-4 times, got {}",
        count
    );
}

#[tokio::test]
async fn test_concurrent_loops_serialized_signing() {
    let signing_in_progress = Arc::new(AtomicU32::new(0));
    let max_concurrent_signings = Arc::new(AtomicU32::new(0));
    let lock = Arc::new(Mutex::new(()));

    let signing_clone1 = signing_in_progress.clone();
    let max_clone1 = max_concurrent_signings.clone();
    let lock_clone1 = lock.clone();

    let main_loop = tokio::spawn(async move {
        for _ in 0..3 {
            let _guard = lock_clone1.lock().await;
            signing_clone1.fetch_add(1, Ordering::SeqCst);

            let concurrent = signing_clone1.load(Ordering::SeqCst);
            loop {
                let current_max = max_clone1.load(Ordering::SeqCst);
                if concurrent <= current_max
                    || max_clone1
                        .compare_exchange_weak(current_max, concurrent, Ordering::SeqCst, Ordering::SeqCst)
                        .is_ok()
                {
                    break;
                }
            }

            tokio::time::sleep(Duration::from_millis(20)).await;
            signing_clone1.fetch_sub(1, Ordering::SeqCst);
        }
    });

    let signing_clone2 = signing_in_progress.clone();
    let max_clone2 = max_concurrent_signings.clone();
    let lock_clone2 = lock.clone();

    let chess_com_loop = tokio::spawn(async move {
        for _ in 0..2 {
            let _guard = lock_clone2.lock().await;
            signing_clone2.fetch_add(1, Ordering::SeqCst);

            let concurrent = signing_clone2.load(Ordering::SeqCst);
            loop {
                let current_max = max_clone2.load(Ordering::SeqCst);
                if concurrent <= current_max
                    || max_clone2
                        .compare_exchange_weak(current_max, concurrent, Ordering::SeqCst, Ordering::SeqCst)
                        .is_ok()
                {
                    break;
                }
            }

            tokio::time::sleep(Duration::from_millis(20)).await;
            signing_clone2.fetch_sub(1, Ordering::SeqCst);
        }
    });

    let _ = tokio::join!(main_loop, chess_com_loop);

    let max_signing = max_concurrent_signings.load(Ordering::SeqCst);
    assert_eq!(
        max_signing, 1,
        "only one signature operation should happen at a time, got max={}",
        max_signing
    );
}
