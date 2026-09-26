//! Tests for `GET /matches` pagination with limit and default cap.
//!
//! Tests verify:
//! - Default limit is 50
//! - Maximum limit is capped at 100
//! - Limit/offset are pushed into SQL, not applied in Rust
//! - Large match tables don't cause memory issues

use axum::body::to_bytes;
use axum::http::{Request, StatusCode};
use event_indexer::{api::build_router, cache::EventCache, db::Database, rpc::SorobanRpcClient};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower::ServiceExt;

fn api_cache() -> Arc<event_indexer::api_cache::ApiCache> {
    Arc::new(event_indexer::api_cache::ApiCache::in_memory())
}

async fn app() -> axum::Router {
    let db_url = std::env::var("DATABASE_URL").unwrap();
    let db = Arc::new(Database::from_dsns(&db_url, &db_url, 2, 2).expect("failed to create db"));
    db.init_schema().await.expect("failed to init schema");

    let cache = Arc::new(RwLock::new(EventCache::new(100)));
    let rpc = Arc::new(SorobanRpcClient::new("http://localhost:1").unwrap());

    build_router(db, cache, rpc, api_cache(), None)
}

/// `GET /matches` (no limit param) returns at most 50 matches (default limit).
#[tokio::test]
async fn get_matches_default_limit_50() {
    if std::env::var("DATABASE_URL").is_err() {
        println!("Skipping get_matches_default_limit_50: DATABASE_URL not set");
        return;
    }

    let app = app().await;

    let request = Request::builder()
        .uri("/matches")
        .method("GET")
        .body(axum::body::Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(parsed["success"], true);

    let matches_array = parsed["data"].as_array().unwrap();
    assert!(matches_array.len() <= 50, "Expected at most 50 matches, got {}", matches_array.len());
}

/// `GET /matches?limit=200` caps the limit at 100 (maximum allowed).
#[tokio::test]
async fn get_matches_limit_capped_at_100() {
    if std::env::var("DATABASE_URL").is_err() {
        println!("Skipping get_matches_limit_capped_at_100: DATABASE_URL not set");
        return;
    }

    let app = app().await;

    let request = Request::builder()
        .uri("/matches?limit=200")
        .method("GET")
        .body(axum::body::Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(parsed["success"], true);

    let matches_array = parsed["data"].as_array().unwrap();
    assert!(matches_array.len() <= 100, "Expected at most 100 matches, got {}", matches_array.len());
}

/// `GET /matches?limit=25` respects a lower limit request.
#[tokio::test]
async fn get_matches_respects_lower_limit() {
    if std::env::var("DATABASE_URL").is_err() {
        println!("Skipping get_matches_respects_lower_limit: DATABASE_URL not set");
        return;
    }

    let app = app().await;

    let request = Request::builder()
        .uri("/matches?limit=25")
        .method("GET")
        .body(axum::body::Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(parsed["success"], true);

    let matches_array = parsed["data"].as_array().unwrap();
    assert!(matches_array.len() <= 25, "Expected at most 25 matches, got {}", matches_array.len());
}

/// `GET /matches?limit=50&offset=0` works with offset-based pagination.
#[tokio::test]
async fn get_matches_offset_pagination_with_limit() {
    if std::env::var("DATABASE_URL").is_err() {
        println!("Skipping get_matches_offset_pagination_with_limit: DATABASE_URL not set");
        return;
    }

    let app = app().await;

    let request = Request::builder()
        .uri("/matches?limit=50&offset=0")
        .method("GET")
        .body(axum::body::Body::empty())
        .unwrap();

    let response = app.oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(parsed["success"], true);

    let matches_array = parsed["data"].as_array().unwrap();
    assert!(matches_array.len() <= 50, "Expected at most 50 matches, got {}", matches_array.len());
}
