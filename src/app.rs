// Copyright (C) 2025 KaxaPage
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// See LICENSE file or https://www.gnu.org/licenses/agpl-3.0.txt
use axum::{extract::DefaultBodyLimit, routing::get, Router};
use governor::{DefaultKeyedRateLimiter, Quota, RateLimiter};
use sqlx::PgPool;
use std::net::IpAddr;
use std::num::NonZeroU32;
use std::sync::Arc;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use uuid::Uuid;

use crate::api;
use crate::routes::{admin, assets, public};

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub workspace_id: Uuid,
    /// Rate limiter for /admin/login — max 10 attempts per minute per IP.
    pub login_rl: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    /// Admin bearer token, read once at startup from ADMIN_TOKEN env var.
    pub admin_token: String,
    /// When true, the auth cookie is sent with the `Secure` attribute.
    /// Set COOKIE_SECURE=true in production (HTTPS only).
    pub cookie_secure: bool,
}

pub fn new_login_rate_limiter() -> Arc<DefaultKeyedRateLimiter<IpAddr>> {
    let quota = Quota::per_minute(NonZeroU32::new(10).unwrap());
    Arc::new(RateLimiter::keyed(quota))
}

/// Constant-time byte comparison. Use for secrets to prevent timing attacks.
pub fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Returns the worst service status from a slice of status strings.
/// Order: operational < maintenance < degraded < partial_outage < major_outage
pub fn overall_status_from_services(statuses: &[String]) -> String {
    fn score(s: &str) -> i32 {
        match s {
            "major_outage" => 4,
            "partial_outage" => 3,
            "degraded" => 2,
            "maintenance" => 1,
            _ => 0,
        }
    }
    let mut worst = "operational";
    let mut worst_score = -1;
    for s in statuses {
        let sc = score(s);
        if sc > worst_score {
            worst = s.as_str();
            worst_score = sc;
        }
    }
    worst.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── overall_status_from_services ─────────────────────────────────────────

    #[test]
    fn overall_empty() {
        assert_eq!(overall_status_from_services(&[]), "operational");
    }

    #[test]
    fn overall_all_operational() {
        let v = vec!["operational".to_string(), "operational".to_string()];
        assert_eq!(overall_status_from_services(&v), "operational");
    }

    #[test]
    fn overall_picks_worst() {
        let v = vec![
            "operational".to_string(),
            "degraded".to_string(),
            "maintenance".to_string(),
        ];
        assert_eq!(overall_status_from_services(&v), "degraded");
    }

    #[test]
    fn overall_major_outage_wins() {
        let v = vec![
            "partial_outage".to_string(),
            "major_outage".to_string(),
            "degraded".to_string(),
        ];
        assert_eq!(overall_status_from_services(&v), "major_outage");
    }

    #[test]
    fn overall_partial_outage() {
        let v = vec!["operational".to_string(), "partial_outage".to_string()];
        assert_eq!(overall_status_from_services(&v), "partial_outage");
    }

    #[test]
    fn overall_maintenance_above_operational() {
        let v = vec!["operational".to_string(), "maintenance".to_string()];
        assert_eq!(overall_status_from_services(&v), "maintenance");
    }

    #[test]
    fn overall_single_service() {
        assert_eq!(
            overall_status_from_services(&["major_outage".to_string()]),
            "major_outage"
        );
        assert_eq!(
            overall_status_from_services(&["maintenance".to_string()]),
            "maintenance"
        );
    }

    // ── constant_time_eq ─────────────────────────────────────────────────────

    #[test]
    fn ct_eq_equal() {
        assert!(constant_time_eq(b"secret", b"secret"));
        assert!(constant_time_eq(b"", b""));
        assert!(constant_time_eq(b"abc123", b"abc123"));
    }

    #[test]
    fn ct_eq_different_value() {
        assert!(!constant_time_eq(b"secret", b"Secret"));
        assert!(!constant_time_eq(b"aaa", b"aab"));
        assert!(!constant_time_eq(b"token", b"toke0"));
    }

    #[test]
    fn ct_eq_different_length() {
        assert!(!constant_time_eq(b"short", b"longer"));
        assert!(!constant_time_eq(b"", b"x"));
        assert!(!constant_time_eq(b"x", b""));
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/", get(public::page_html))
        .route("/rss.xml", get(public::page_rss))
        .route("/style.css", get(assets::style_css))
        .route("/healthz", get(public::healthz))
        // embedded admin SPA
        .route("/admin", get(admin::admin_handler))
        .route("/admin/{*path}", get(admin::admin_handler))
        // API
        .merge(api::router(state.clone()))
        .with_state(state)
        .layer(DefaultBodyLimit::max(1_048_576)) // 1 MB
        .layer(CompressionLayer::new())
        .layer(TraceLayer::new_for_http())
}
