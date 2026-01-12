// Copyright (C) 2025 KaxaPage
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// See LICENSE file or https://www.gnu.org/licenses/agpl-3.0.txt
mod api;
pub mod app;
pub mod db;

mod middleware {
    pub mod admin_auth;
}

mod routes {
    pub mod admin;
    pub mod assets;
    pub mod public;
}

use dotenvy::dotenv;
use std::net::SocketAddr;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let database_url = std::env::var("DATABASE_URL")?;
    let pool = db::connect(&database_url).await?;

    // Migrations run automatically at startup (acceptable for self-hosted; for
    // zero-downtime deployments consider running migrations as a separate step).
    db::migrate(&pool).await?;

    let admin_token = std::env::var("ADMIN_TOKEN").unwrap_or_default();
    if admin_token.is_empty() {
        tracing::warn!("ADMIN_TOKEN is not set; admin endpoints will be inaccessible");
    }

    let cookie_secure = std::env::var("COOKIE_SECURE")
        .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"))
        .unwrap_or(false);

    // If WORKSPACE_ID is not set, fall back to the first workspace in the DB.
    let workspace_id = if let Ok(v) = std::env::var("WORKSPACE_ID") {
        v.parse()?
    } else {
        // fallback: first workspace
        sqlx::query_scalar::<_, uuid::Uuid>(
            "SELECT id FROM workspaces ORDER BY created_at ASC LIMIT 1",
        )
        .fetch_optional(&pool)
        .await?
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No workspace found in the database. \
                 Run the bootstrap endpoint first, or set WORKSPACE_ID env var."
            )
        })?
    };

    let state = app::AppState {
        db: pool,
        workspace_id,
        login_rl: app::new_login_rate_limiter(),
        admin_token,
        cookie_secure,
    };

    let r = app::router(state);

    let addr = std::env::var("LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        r.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;
    Ok(())
}
