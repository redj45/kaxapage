//! Integration tests for the kaxapage API.
//!
//! Requires a running PostgreSQL instance. Set DATABASE_URL before running:
//!   DATABASE_URL=postgres://kaxapage:kaxapage@localhost:5432/kaxapage cargo test
//!
//! Each test boots a fresh router with a real DB connection and runs migrations.
//! Tests are designed to be independent — each creates its own workspace via bootstrap.

use axum::http::HeaderValue;
use axum_test::TestServer;
use serde_json::{json, Value};
use std::net::SocketAddr;

// ── helpers ──────────────────────────────────────────────────────────────────

async fn make_pool() -> sqlx::PgPool {
    dotenvy::dotenv().ok();
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL must be set to run integration tests");
    let pool = kaxapage::db::connect(&database_url)
        .await
        .expect("failed to connect to database");
    kaxapage::db::migrate(&pool)
        .await
        .expect("failed to run migrations");
    pool
}

fn make_test_server(state: kaxapage::app::AppState) -> TestServer {
    use axum::extract::connect_info::MockConnectInfo;
    let router =
        kaxapage::app::router(state).layer(MockConnectInfo(SocketAddr::from(([127, 0, 0, 1], 0))));
    TestServer::new(router).unwrap()
}

async fn make_server() -> TestServer {
    let pool = make_pool().await;
    let workspace_id = sqlx::query_scalar::<_, uuid::Uuid>(
        "SELECT id FROM workspaces ORDER BY created_at DESC LIMIT 1",
    )
    .fetch_optional(&pool)
    .await
    .unwrap()
    .unwrap_or_else(uuid::Uuid::new_v4);

    let state = kaxapage::app::AppState {
        db: pool,
        workspace_id,
        login_rl: kaxapage::app::new_login_rate_limiter(),
        admin_token: "test-token".to_string(),
        cookie_secure: false,
    };

    make_test_server(state)
}

/// Bootstrap a fresh workspace and return (server, page_id, page_slug).
/// Uses a unique slug per call to avoid conflicts between parallel tests.
async fn bootstrap_fresh() -> (TestServer, String, String) {
    let pool = make_pool().await;

    // Insert workspace + page directly so each test is truly isolated.
    let ws = sqlx::query!(
        "INSERT INTO workspaces (name) VALUES ($1) RETURNING id",
        "Test Workspace"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let slug = format!(
        "test-{}",
        uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
    );

    let page = sqlx::query!(
        "INSERT INTO status_pages (workspace_id, slug, title, published) VALUES ($1, $2, $3, true) RETURNING id, slug",
        ws.id,
        slug,
        "Test Status Page"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let state = kaxapage::app::AppState {
        db: pool,
        workspace_id: ws.id,
        login_rl: kaxapage::app::new_login_rate_limiter(),
        admin_token: "test-token".to_string(),
        cookie_secure: false,
    };

    let server = make_test_server(state);
    (server, page.id.to_string(), page.slug)
}

fn admin_cookie() -> HeaderValue {
    // Must match the admin_token set in make_server() / bootstrap_fresh().
    HeaderValue::from_str("kp_admin=test-token").unwrap()
}

// ── /healthz ─────────────────────────────────────────────────────────────────

#[tokio::test]
async fn healthz_returns_ok() {
    let server = make_server().await;
    let res = server.get("/healthz").await;
    res.assert_status_ok();
    res.assert_text("ok");
}

// ── /api/v1/admin/login ───────────────────────────────────────────────────────

#[tokio::test]
async fn login_valid_token() {
    let server = make_server().await;

    let res = server
        .post("/api/v1/admin/login")
        .json(&json!({ "token": "test-token" }))
        .await;

    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["ok"], true);
}

#[tokio::test]
async fn login_invalid_token_returns_401() {
    let server = make_server().await;

    let res = server
        .post("/api/v1/admin/login")
        .json(&json!({ "token": "wrong-token" }))
        .await;

    res.assert_status_unauthorized();
    let body: Value = res.json();
    assert_eq!(body["ok"], false);
    assert_eq!(body["error"]["code"], "unauthorized");
}

#[tokio::test]
async fn login_empty_token_returns_401() {
    let server = make_server().await;

    let res = server
        .post("/api/v1/admin/login")
        .json(&json!({ "token": "" }))
        .await;

    res.assert_status_unauthorized();
}

// ── /api/v1/admin/logout ──────────────────────────────────────────────────────

#[tokio::test]
async fn logout_returns_ok() {
    let server = make_server().await;
    let res = server.post("/api/v1/admin/logout").await;
    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["ok"], true);
}

// ── /api/v1/admin/pages ───────────────────────────────────────────────────────

#[tokio::test]
async fn admin_pages_requires_auth() {
    let server = make_server().await;
    let res = server.get("/api/v1/admin/pages").await;
    res.assert_status_unauthorized();
}

#[tokio::test]
async fn admin_pages_returns_list() {
    let (server, _page_id, _slug) = bootstrap_fresh().await;

    let res = server
        .get("/api/v1/admin/pages")
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .await;

    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["ok"], true);
    assert!(body["data"].is_array());
    assert!(!body["data"].as_array().unwrap().is_empty());
}

// ── /api/v1/admin/services ────────────────────────────────────────────────────

#[tokio::test]
async fn services_crud() {
    let (server, _page_id, _slug) = bootstrap_fresh().await;

    // Create
    let res = server
        .post("/api/v1/admin/services")
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .json(&json!({
            "name": "API Gateway",
            "description": "Main API",
            "status": "operational"
        }))
        .await;

    res.assert_status(axum::http::StatusCode::CREATED);
    let body: Value = res.json();
    assert_eq!(body["ok"], true);
    let service_id = body["data"]["id"].as_str().unwrap().to_string();
    assert_eq!(body["data"]["name"], "API Gateway");
    assert_eq!(body["data"]["status"], "operational");

    // List
    let res = server
        .get("/api/v1/admin/services")
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .await;

    res.assert_status_ok();
    let body: Value = res.json();
    assert!(body["data"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["id"] == service_id));

    // Patch
    let res = server
        .patch(&format!("/api/v1/admin/services/{service_id}"))
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .json(&json!({ "status": "degraded" }))
        .await;

    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["data"]["status"], "degraded");

    // Delete
    let res = server
        .delete(&format!("/api/v1/admin/services/{service_id}"))
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .await;

    res.assert_status(axum::http::StatusCode::NO_CONTENT);

    // Confirm deleted
    let res = server
        .delete(&format!("/api/v1/admin/services/{service_id}"))
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .await;

    res.assert_status_not_found();
}

#[tokio::test]
async fn create_service_invalid_status() {
    let (server, _page_id, _slug) = bootstrap_fresh().await;

    let res = server
        .post("/api/v1/admin/services")
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .json(&json!({
            "name": "My Service",
            "status": "broken"
        }))
        .await;

    res.assert_status_bad_request();
    let body: Value = res.json();
    assert_eq!(body["ok"], false);
    assert_eq!(body["error"]["code"], "bad_request");
}

#[tokio::test]
async fn create_service_empty_name() {
    let (server, _page_id, _slug) = bootstrap_fresh().await;

    let res = server
        .post("/api/v1/admin/services")
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .json(&json!({ "name": "   " }))
        .await;

    res.assert_status_bad_request();
}

#[tokio::test]
async fn patch_service_not_found() {
    let (server, _page_id, _slug) = bootstrap_fresh().await;
    let fake_id = uuid::Uuid::new_v4();

    let res = server
        .patch(&format!("/api/v1/admin/services/{fake_id}"))
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .json(&json!({ "status": "degraded" }))
        .await;

    res.assert_status_not_found();
}

// ── /api/v1/admin/incidents ───────────────────────────────────────────────────

async fn create_test_incident(server: &TestServer, page_id: &str) -> String {
    let res = server
        .post("/api/v1/admin/incidents")
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .json(&json!({
            "status_page_id": page_id,
            "title": "Test Incident",
            "message": "Something went wrong",
            "status": "investigating",
            "impact": "minor"
        }))
        .await;

    let body: Value = res.json();
    assert_eq!(body["ok"], true, "create_test_incident failed: {body}");
    body["data"]["incident_id"].as_str().unwrap().to_string()
}

#[tokio::test]
async fn incident_full_lifecycle() {
    let (server, page_id, _slug) = bootstrap_fresh().await;

    // Create incident
    let incident_id = create_test_incident(&server, &page_id).await;

    // Get incident
    let res = server
        .get(&format!("/api/v1/admin/incidents/{incident_id}"))
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .await;

    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["data"]["incident"]["status"], "investigating");
    assert_eq!(body["data"]["incident"]["title"], "Test Incident");
    assert_eq!(body["data"]["updates"].as_array().unwrap().len(), 1);

    // Add update
    let res = server
        .post(&format!("/api/v1/admin/incidents/{incident_id}/updates"))
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .json(&json!({
            "message": "We identified the root cause",
            "status": "identified"
        }))
        .await;

    res.assert_status(axum::http::StatusCode::CREATED);
    let body: Value = res.json();
    assert_eq!(body["data"]["message"], "We identified the root cause");

    // Confirm status changed
    let res = server
        .get(&format!("/api/v1/admin/incidents/{incident_id}"))
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .await;

    let body: Value = res.json();
    assert_eq!(body["data"]["incident"]["status"], "identified");
    assert_eq!(body["data"]["updates"].as_array().unwrap().len(), 2);

    // Resolve
    let res = server
        .post(&format!("/api/v1/admin/incidents/{incident_id}/resolve"))
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .json(&json!({ "message": "All systems normal." }))
        .await;

    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["ok"], true);

    // Confirm resolved
    let res = server
        .get(&format!("/api/v1/admin/incidents/{incident_id}"))
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .await;

    let body: Value = res.json();
    assert_eq!(body["data"]["incident"]["status"], "resolved");
    assert!(body["data"]["incident"]["resolved_at"].is_string());
}

#[tokio::test]
async fn resolve_incident_twice_returns_409() {
    let (server, page_id, _slug) = bootstrap_fresh().await;
    let incident_id = create_test_incident(&server, &page_id).await;

    // First resolve
    server
        .post(&format!("/api/v1/admin/incidents/{incident_id}/resolve"))
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .json(&json!({ "message": "Fixed." }))
        .await
        .assert_status_ok();

    // Second resolve — must fail with 409
    let res = server
        .post(&format!("/api/v1/admin/incidents/{incident_id}/resolve"))
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .json(&json!({ "message": "Trying again." }))
        .await;

    res.assert_status(axum::http::StatusCode::CONFLICT);
    let body: Value = res.json();
    assert_eq!(body["error"]["code"], "conflict");
}

#[tokio::test]
async fn add_update_to_resolved_incident_returns_409() {
    let (server, page_id, _slug) = bootstrap_fresh().await;
    let incident_id = create_test_incident(&server, &page_id).await;

    // Resolve it
    server
        .post(&format!("/api/v1/admin/incidents/{incident_id}/resolve"))
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .json(&json!({ "message": "Fixed." }))
        .await
        .assert_status_ok();

    // Try to add update — must fail with 409
    let res = server
        .post(&format!("/api/v1/admin/incidents/{incident_id}/updates"))
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .json(&json!({ "message": "Update after resolve" }))
        .await;

    res.assert_status(axum::http::StatusCode::CONFLICT);
    let body: Value = res.json();
    assert_eq!(body["error"]["code"], "conflict");
}

#[tokio::test]
async fn create_incident_empty_title_returns_400() {
    let (server, page_id, _slug) = bootstrap_fresh().await;

    let res = server
        .post("/api/v1/admin/incidents")
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .json(&json!({
            "status_page_id": page_id,
            "title": "",
            "message": "Something broke"
        }))
        .await;

    res.assert_status_bad_request();
}

#[tokio::test]
async fn create_incident_empty_message_returns_400() {
    let (server, page_id, _slug) = bootstrap_fresh().await;

    let res = server
        .post("/api/v1/admin/incidents")
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .json(&json!({
            "status_page_id": page_id,
            "title": "My Incident",
            "message": ""
        }))
        .await;

    res.assert_status_bad_request();
}

#[tokio::test]
async fn create_incident_invalid_status_returns_400() {
    let (server, page_id, _slug) = bootstrap_fresh().await;

    let res = server
        .post("/api/v1/admin/incidents")
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .json(&json!({
            "status_page_id": page_id,
            "title": "My Incident",
            "message": "Something broke",
            "status": "not-a-status"
        }))
        .await;

    res.assert_status_bad_request();
}

#[tokio::test]
async fn create_incident_invalid_impact_returns_400() {
    let (server, page_id, _slug) = bootstrap_fresh().await;

    let res = server
        .post("/api/v1/admin/incidents")
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .json(&json!({
            "status_page_id": page_id,
            "title": "My Incident",
            "message": "Something broke",
            "impact": "super-critical"
        }))
        .await;

    res.assert_status_bad_request();
}

#[tokio::test]
async fn get_incident_not_found() {
    let (server, _page_id, _slug) = bootstrap_fresh().await;
    let fake_id = uuid::Uuid::new_v4();

    let res = server
        .get(&format!("/api/v1/admin/incidents/{fake_id}"))
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .await;

    res.assert_status_not_found();
}

#[tokio::test]
async fn list_incidents_pagination() {
    let (server, page_id, _slug) = bootstrap_fresh().await;

    // Create 3 incidents
    for i in 1..=3 {
        server
            .post("/api/v1/admin/incidents")
            .add_header(axum::http::header::COOKIE, admin_cookie())
            .json(&json!({
                "status_page_id": page_id,
                "title": format!("Incident {i}"),
                "message": "msg"
            }))
            .await
            .assert_status(axum::http::StatusCode::CREATED);
    }

    // List with limit=2
    let res = server
        .get(&format!(
            "/api/v1/admin/incidents?status_page_id={page_id}&limit=2"
        ))
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .await;

    res.assert_status_ok();
    let body: Value = res.json();
    let items = body["data"]["items"].as_array().unwrap();
    assert_eq!(items.len(), 2);
    // next_cursor must be present
    assert!(!body["data"]["next_cursor"].is_null());
}

// ── resolve with empty message returns 400 ────────────────────────────────────

#[tokio::test]
async fn resolve_empty_message_returns_400() {
    let (server, page_id, _slug) = bootstrap_fresh().await;
    let incident_id = create_test_incident(&server, &page_id).await;

    let res = server
        .post(&format!("/api/v1/admin/incidents/{incident_id}/resolve"))
        .add_header(axum::http::header::COOKIE, admin_cookie())
        .json(&json!({ "message": "" }))
        .await;

    res.assert_status_bad_request();
}

// ── /api/v1/public/pages/:slug ────────────────────────────────────────────────

#[tokio::test]
async fn public_page_api_returns_data() {
    let (server, _page_id, slug) = bootstrap_fresh().await;

    let res = server.get(&format!("/api/v1/public/pages/{slug}")).await;

    res.assert_status_ok();
    let body: Value = res.json();
    assert_eq!(body["ok"], true);
    assert!(body["data"]["page"].is_object());
    assert!(body["data"]["services"].is_array());
    assert!(body["data"]["active_incidents"].is_array());
    assert!(body["data"]["recent_incidents"].is_array());
    assert!(body["data"]["overall_status"].is_string());
}

#[tokio::test]
async fn public_page_api_unknown_slug_returns_404() {
    let server = make_server().await;

    let res = server
        .get("/api/v1/public/pages/totally-unknown-slug-xyz")
        .await;

    res.assert_status_not_found();
}

// ── GET / (public HTML page) ──────────────────────────────────────────────────

#[tokio::test]
async fn public_html_page_returns_200() {
    let (server, _page_id, _slug) = bootstrap_fresh().await;

    let res = server.get("/").await;
    res.assert_status_ok();

    let text = res.text();
    assert!(text.contains("<!doctype html>") || text.contains("<!DOCTYPE html>"));
}

// ── GET /rss.xml ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn rss_feed_returns_xml() {
    let (server, _page_id, _slug) = bootstrap_fresh().await;

    let res = server.get("/rss.xml").await;
    res.assert_status_ok();

    let text = res.text();
    assert!(text.contains(r#"<?xml version="1.0""#));
    assert!(text.contains("<rss"));
}

// ── admin auth middleware ─────────────────────────────────────────────────────

#[tokio::test]
async fn admin_endpoints_reject_missing_cookie() {
    let (server, page_id, _slug) = bootstrap_fresh().await;

    // services
    server
        .get("/api/v1/admin/services")
        .await
        .assert_status_unauthorized();

    // incidents
    server
        .get(&format!("/api/v1/admin/incidents?status_page_id={page_id}"))
        .await
        .assert_status_unauthorized();

    // pages
    server
        .get("/api/v1/admin/pages")
        .await
        .assert_status_unauthorized();
}
