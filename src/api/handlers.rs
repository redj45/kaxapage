// Copyright (C) 2025 KaxaPage
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// See LICENSE file or https://www.gnu.org/licenses/agpl-3.0.txt
use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{header, header::SET_COOKIE, HeaderMap, StatusCode},
    middleware,
    response::IntoResponse,
    routing::{get, patch, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::net::SocketAddr;
use uuid::Uuid;

use crate::api::error::ApiError;
use crate::api::extractors::AppJson;
use crate::api::types;
use crate::app::{overall_status_from_services, AppState};
use crate::middleware::admin_auth::admin_auth;

fn extract_client_ip(headers: &HeaderMap, fallback: std::net::IpAddr) -> std::net::IpAddr {
    if let Some(val) = headers.get("X-Real-IP") {
        if let Ok(s) = val.to_str() {
            if let Ok(ip) = s.trim().parse() {
                return ip;
            }
        }
    }
    if let Some(val) = headers.get("X-Forwarded-For") {
        if let Ok(s) = val.to_str() {
            if let Some(first) = s.split(',').next() {
                if let Ok(ip) = first.trim().parse() {
                    return ip;
                }
            }
        }
    }
    fallback
}

pub fn router(state: AppState) -> axum::Router<AppState> {
    let admin_routes = Router::new()
        .route("/api/v1/admin/pages", get(admin_pages))
        .route("/api/v1/admin/pages/{id}", patch(patch_page))
        .route(
            "/api/v1/admin/services",
            get(list_services).post(create_service),
        )
        .route(
            "/api/v1/admin/services/{id}",
            patch(patch_service).delete(delete_service),
        )
        .route(
            "/api/v1/admin/incidents",
            get(list_incidents).post(create_incident),
        )
        .route("/api/v1/admin/incidents/{id}", get(get_incident))
        .route("/api/v1/admin/incidents/{id}/updates", post(add_update))
        .route(
            "/api/v1/admin/incidents/{id}/resolve",
            post(resolve_incident),
        )
        .layer(middleware::from_fn_with_state(state, admin_auth));

    Router::new()
        .route("/api/v1/bootstrap", post(bootstrap))
        .route("/api/v1/admin/login", post(admin_login))
        .route("/api/v1/admin/logout", post(admin_logout))
        .route("/api/v1/public/pages/{slug}", get(public_page))
        .merge(admin_routes)
}

/* ---------------- Auth: Login / Logout ---------------- */

pub async fn admin_login(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    State(state): State<AppState>,
    AppJson(req): AppJson<types::LoginReq>,
) -> impl IntoResponse {
    let ip = extract_client_ip(&headers, addr.ip());
    if state.login_rl.check_key(&ip).is_err() {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(types::err(
                "rate_limited",
                "too many login attempts; try again later",
            )),
        )
            .into_response();
    }

    let ok = !state.admin_token.is_empty()
        && !req.token.is_empty()
        && crate::app::constant_time_eq(req.token.as_bytes(), state.admin_token.as_bytes());

    if !ok {
        return (
            StatusCode::UNAUTHORIZED,
            Json(types::err("unauthorized", "invalid token")),
        )
            .into_response();
    }

    // Reject tokens that contain characters not permitted in a cookie value (RFC 6265).
    if req
        .token
        .bytes()
        .any(|b| matches!(b, b';' | b',' | b' ' | b'"' | b'\\' | b'\n' | b'\r'))
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(types::err(
                "bad_request",
                "token contains characters not permitted in a cookie value",
            )),
        )
            .into_response();
    }

    let secure_flag = if state.cookie_secure { "; Secure" } else { "" };
    let cookie = format!(
        "kp_admin={}; HttpOnly; SameSite=Strict; Path=/; Max-Age=86400{}",
        req.token, secure_flag
    );
    let mut headers = HeaderMap::new();
    headers.insert(SET_COOKIE, cookie.parse().unwrap());
    (headers, Json(types::ok(serde_json::json!({})))).into_response()
}

pub async fn admin_logout() -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        SET_COOKIE,
        "kp_admin=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0"
            .parse()
            .unwrap(),
    );
    (headers, Json(types::ok(serde_json::json!({})))).into_response()
}

/* ---------------- Validation ---------------- */

fn validate_slug(s: &str) -> Result<(), ApiError> {
    if s.len() < 2 || s.len() > 60 {
        return Err(ApiError::BadRequest(
            "slug must be between 2 and 60 characters".into(),
        ));
    }
    if !s
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(ApiError::BadRequest(
            "slug may only contain lowercase letters, digits, and hyphens".into(),
        ));
    }
    if s.starts_with('-') || s.ends_with('-') {
        return Err(ApiError::BadRequest(
            "slug must not start or end with a hyphen".into(),
        ));
    }
    Ok(())
}

fn validate_str(value: &str, field: &str, max_len: usize) -> Result<(), ApiError> {
    if value.trim().is_empty() {
        return Err(ApiError::BadRequest(format!("{field} must not be empty")));
    }
    if value.len() > max_len {
        return Err(ApiError::BadRequest(format!(
            "{field} must not exceed {max_len} characters"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── validate_slug ────────────────────────────────────────────────────────

    #[test]
    fn slug_valid() {
        assert!(validate_slug("my-page").is_ok());
        assert!(validate_slug("status").is_ok());
        assert!(validate_slug("abc123").is_ok());
        assert!(validate_slug("a-b-c-1-2-3").is_ok());
        assert!(validate_slug("ab").is_ok()); // min len
                                              // 60 chars
        assert!(validate_slug("a".repeat(60).as_str()).is_ok());
    }

    #[test]
    fn slug_too_short() {
        assert!(validate_slug("a").is_err());
        assert!(validate_slug("").is_err());
    }

    #[test]
    fn slug_too_long() {
        assert!(validate_slug("a".repeat(61).as_str()).is_err());
    }

    #[test]
    fn slug_invalid_chars() {
        assert!(validate_slug("My-Page").is_err()); // uppercase
        assert!(validate_slug("my_page").is_err()); // underscore
        assert!(validate_slug("my page").is_err()); // space
        assert!(validate_slug("my.page").is_err()); // dot
    }

    #[test]
    fn slug_leading_trailing_hyphen() {
        assert!(validate_slug("-mypage").is_err());
        assert!(validate_slug("mypage-").is_err());
        assert!(validate_slug("-mypage-").is_err());
    }

    // ── validate_str ─────────────────────────────────────────────────────────

    #[test]
    fn str_valid() {
        assert!(validate_str("hello", "field", 200).is_ok());
        assert!(validate_str("  hello  ", "field", 200).is_ok());
    }

    #[test]
    fn str_empty() {
        assert!(validate_str("", "field", 200).is_err());
        assert!(validate_str("   ", "field", 200).is_err());
        assert!(validate_str("\t\n", "field", 200).is_err());
    }

    #[test]
    fn str_too_long() {
        assert!(validate_str("a".repeat(201).as_str(), "field", 200).is_err());
        assert!(validate_str("a".repeat(200).as_str(), "field", 200).is_ok());
    }
}

/* ---------------- Bootstrap ---------------- */

pub async fn bootstrap(
    State(state): State<AppState>,
    AppJson(req): AppJson<types::BootstrapReq>,
) -> Result<impl IntoResponse, ApiError> {
    validate_str(&req.workspace_name, "workspace_name", 200)?;
    validate_slug(&req.page.slug)?;
    validate_str(&req.page.title, "title", 200)?;
    for svc in &req.services {
        validate_str(&svc.name, "service name", 200)?;
    }

    let mut tx = state.db.begin().await.map_err(ApiError::Db)?;

    // Allow bootstrap only if no workspaces exist.
    let cnt = sqlx::query_scalar::<_, i64>(r#"SELECT COUNT(*) FROM workspaces"#)
        .fetch_one(&mut *tx)
        .await
        .map_err(ApiError::Db)?;

    if cnt > 0 {
        return Err(ApiError::Conflict("already bootstrapped"));
    }

    let ws = sqlx::query!(
        r#"INSERT INTO workspaces (name) VALUES ($1) RETURNING id"#,
        req.workspace_name
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::Db)?;

    let page = sqlx::query!(
        r#"
        INSERT INTO status_pages (workspace_id, slug, title, published)
        VALUES ($1, $2, $3, true)
        RETURNING id, slug
        "#,
        ws.id,
        req.page.slug,
        req.page.title
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::Db)?;

    for (i, s) in req.services.iter().enumerate() {
        sqlx::query!(
            r#"
            INSERT INTO services (workspace_id, name, description, position, status)
            VALUES ($1, $2, $3, $4, 'operational')
            "#,
            ws.id,
            s.name,
            s.description,
            i as i32
        )
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Db)?;
    }

    tx.commit().await.map_err(ApiError::Db)?;

    Ok(Json(types::ok(types::BootstrapResp {
        workspace_id: ws.id,
        status_page_id: page.id,
        page_slug: page.slug,
    })))
}

/* ---------------- Admin: Pages ---------------- */

pub async fn admin_pages(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let items = sqlx::query_as!(
        types::AdminPageItem,
        r#"
        SELECT id, slug, title, published
        FROM status_pages
        WHERE workspace_id = $1
        ORDER BY created_at ASC
        "#,
        state.workspace_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Db)?;

    Ok(Json(types::ok(items)))
}

pub async fn patch_page(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    AppJson(req): AppJson<types::PatchPageReq>,
) -> Result<impl IntoResponse, ApiError> {
    if let Some(ref t) = req.title {
        validate_str(t, "title", 200)?;
    }

    let row = sqlx::query!(
        r#"
        UPDATE status_pages
        SET title = COALESCE($1, title),
            published = COALESCE($2, published)
        WHERE id = $3 AND workspace_id = $4
        RETURNING id
        "#,
        req.title,
        req.published,
        id,
        state.workspace_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Db)?;

    if row.is_none() {
        return Err(ApiError::NotFound("page not found"));
    }

    Ok(Json(types::ok(serde_json::json!({ "id": id }))))
}

/* ---------------- Admin: Services ---------------- */

pub async fn list_services(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let items = sqlx::query_as!(
        types::ServiceItem,
        r#"
        SELECT id, name, description, position, status as "status!: String", updated_at
        FROM services
        WHERE workspace_id = $1
        ORDER BY position ASC, created_at ASC
        "#,
        state.workspace_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Db)?;

    Ok(Json(types::ok(items)))
}

pub async fn create_service(
    State(state): State<AppState>,
    AppJson(req): AppJson<types::CreateServiceReq>,
) -> Result<impl IntoResponse, ApiError> {
    validate_str(&req.name, "name", 200)?;
    if let Some(ref d) = req.description {
        validate_str(d, "description", 1000)?;
    }

    let position = req.position.unwrap_or(0);
    let status = req
        .status
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("operational");

    let row = sqlx::query_as!(
        types::ServiceItem,
        r#"
        INSERT INTO services (workspace_id, name, description, position, status)
        VALUES ($1, $2, $3, $4, ($5::text)::service_status)
        RETURNING id, name, description, position, status as "status!: String", updated_at
        "#,
        state.workspace_id,
        req.name,
        req.description,
        position,
        status
    )
    .fetch_one(&state.db)
    .await
    .map_err(ApiError::Db)?;

    Ok((StatusCode::CREATED, Json(types::ok(row))))
}

pub async fn patch_service(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    AppJson(req): AppJson<types::PatchServiceReq>,
) -> Result<impl IntoResponse, ApiError> {
    if let Some(ref n) = req.name {
        validate_str(n, "name", 200)?;
    }
    // Validate description only when setting a non-null value.
    if let Some(Some(ref d)) = req.description {
        validate_str(d, "description", 1000)?;
    }

    // Decompose the three-state description field:
    //   None          → field absent, no change
    //   Some(None)    → explicit null, clear the value
    //   Some(Some(v)) → set to new value
    let (update_desc, desc_value): (bool, Option<String>) = match req.description {
        None => (false, None),
        Some(v) => (true, v),
    };

    let row = sqlx::query_as!(
        types::ServiceItem,
        r#"
        UPDATE services
        SET name = COALESCE($1, name),
            description = CASE WHEN $2 THEN $3 ELSE description END,
            position = COALESCE($4, position),
            status = COALESCE(($5::text)::service_status, status),
            updated_at = now()
        WHERE id = $6 AND workspace_id = $7
        RETURNING id, name, description, position, status as "status!: String", updated_at
        "#,
        req.name,
        update_desc,
        desc_value,
        req.position,
        req.status.as_ref().map(|s| s.as_str()),
        id,
        state.workspace_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Db)?;

    match row {
        Some(svc) => Ok(Json(types::ok(svc))),
        None => Err(ApiError::NotFound("service not found")),
    }
}

pub async fn delete_service(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let res = sqlx::query!(
        r#"DELETE FROM services WHERE id = $1 AND workspace_id = $2"#,
        id,
        state.workspace_id
    )
    .execute(&state.db)
    .await
    .map_err(ApiError::Db)?;

    if res.rows_affected() == 0 {
        return Err(ApiError::NotFound("service not found"));
    }

    Ok(StatusCode::NO_CONTENT)
}

/* ---------------- Admin: Incidents ---------------- */

#[derive(Deserialize)]
pub struct IncidentsQuery {
    pub status_page_id: Uuid,
    pub limit: Option<i64>,
    // Composite cursor: both fields required to avoid skipping records with equal started_at.
    pub cursor_ts: Option<DateTime<Utc>>,
    pub cursor_id: Option<Uuid>,
}

pub async fn list_incidents(
    State(state): State<AppState>,
    Query(q): Query<IncidentsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let limit = q.limit.unwrap_or(20).clamp(1, 100);

    let items = match (q.cursor_ts, q.cursor_id) {
        (Some(ts), Some(id)) => {
            sqlx::query_as!(
                types::IncidentListItem,
                r#"
                SELECT id, title, status as "status!: String", impact as "impact!: String", started_at, resolved_at
                FROM incidents
                WHERE workspace_id = $1
                  AND status_page_id = $2
                  AND (started_at < $3 OR (started_at = $3 AND id < $4))
                ORDER BY started_at DESC, id DESC
                LIMIT $5
                "#,
                state.workspace_id,
                q.status_page_id,
                ts,
                id,
                limit
            )
            .fetch_all(&state.db)
            .await
            .map_err(ApiError::Db)?
        }
        _ => {
            sqlx::query_as!(
                types::IncidentListItem,
                r#"
                SELECT id, title, status as "status!: String", impact as "impact!: String", started_at, resolved_at
                FROM incidents
                WHERE workspace_id = $1 AND status_page_id = $2
                ORDER BY started_at DESC, id DESC
                LIMIT $3
                "#,
                state.workspace_id,
                q.status_page_id,
                limit
            )
            .fetch_all(&state.db)
            .await
            .map_err(ApiError::Db)?
        }
    };

    let next_cursor = items.last().map(|x| {
        serde_json::json!({
            "cursor_ts": x.started_at,
            "cursor_id": x.id,
        })
    });

    Ok(Json(types::ok(serde_json::json!({
        "items": items,
        "next_cursor": next_cursor
    }))))
}

pub async fn get_incident(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let incident = sqlx::query_as!(
        types::IncidentListItem,
        r#"
        SELECT id, title, status as "status!: String", impact as "impact!: String", started_at, resolved_at
        FROM incidents
        WHERE id = $1 AND workspace_id = $2
        "#,
        id,
        state.workspace_id
    )
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Db)?
    .ok_or(ApiError::NotFound("incident not found"))?;

    let updates = sqlx::query_as!(
        types::IncidentUpdateItem,
        r#"
        SELECT id, message, created_at
        FROM incident_updates
        WHERE incident_id = $1
        ORDER BY created_at ASC
        "#,
        id
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Db)?;

    let affected_services = sqlx::query_as!(
        types::AffectedServiceItem,
        r#"
        SELECT s.id, s.name
        FROM services s
        JOIN incident_services isc ON isc.service_id = s.id
        WHERE isc.incident_id = $1
        ORDER BY s.position ASC, s.created_at ASC
        "#,
        id
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Db)?;

    Ok(Json(types::ok(types::IncidentGetResp {
        incident,
        updates,
        affected_services,
    })))
}

pub async fn create_incident(
    State(state): State<AppState>,
    AppJson(req): AppJson<types::CreateIncidentReq>,
) -> Result<impl IntoResponse, ApiError> {
    validate_str(&req.title, "title", 200)?;
    validate_str(&req.message, "message", 10_000)?;

    let mut tx = state.db.begin().await.map_err(ApiError::Db)?;

    // Validate page belongs to workspace
    let page = sqlx::query!(
        r#"SELECT id FROM status_pages WHERE id = $1 AND workspace_id = $2"#,
        req.status_page_id,
        state.workspace_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::Db)?;

    if page.is_none() {
        return Err(ApiError::NotFound("status page not found"));
    }

    let status = req
        .status
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("investigating");
    let impact = req.impact.as_ref().map(|s| s.as_str()).unwrap_or("minor");
    let started_at = req.started_at.unwrap_or_else(Utc::now);

    let inc = sqlx::query!(
        r#"
        INSERT INTO incidents (workspace_id, status_page_id, title, status, impact, started_at)
        VALUES ($1, $2, $3, ($4::text)::incident_status, ($5::text)::incident_impact, $6)
        RETURNING id
        "#,
        state.workspace_id,
        req.status_page_id,
        req.title,
        status,
        impact,
        started_at
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::Db)?;

    let upd = sqlx::query!(
        r#"
        INSERT INTO incident_updates (incident_id, message)
        VALUES ($1, $2)
        RETURNING id
        "#,
        inc.id,
        req.message
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::Db)?;

    // Link affected services (validate they belong to this workspace first).
    if let Some(ref service_ids) = req.service_ids {
        if !service_ids.is_empty() {
            let valid_count: i64 = sqlx::query_scalar!(
                r#"SELECT COUNT(*) FROM services WHERE id = ANY($1) AND workspace_id = $2"#,
                service_ids as &[Uuid],
                state.workspace_id
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(ApiError::Db)?
            .unwrap_or(0);

            if valid_count as usize != service_ids.len() {
                return Err(ApiError::BadRequest(
                    "one or more service IDs are invalid".into(),
                ));
            }

            // Bulk insert via unnest — avoids a per-row round-trip.
            sqlx::query!(
                r#"
                INSERT INTO incident_services (incident_id, service_id)
                SELECT $1, sid FROM unnest($2::uuid[]) AS sid
                ON CONFLICT DO NOTHING
                "#,
                inc.id,
                service_ids as &[Uuid]
            )
            .execute(&mut *tx)
            .await
            .map_err(ApiError::Db)?;
        }
    }

    tx.commit().await.map_err(ApiError::Db)?;

    Ok((
        StatusCode::CREATED,
        Json(types::ok(types::CreateIncidentResp {
            incident_id: inc.id,
            update_id: upd.id,
        })),
    ))
}

pub async fn add_update(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    AppJson(req): AppJson<types::CreateUpdateReq>,
) -> Result<impl IntoResponse, ApiError> {
    validate_str(&req.message, "message", 10_000)?;

    let mut tx = state.db.begin().await.map_err(ApiError::Db)?;

    // Ensure incident exists, belongs to workspace, and is not already resolved
    let incident = sqlx::query!(
        r#"SELECT id, status as "status!: String" FROM incidents WHERE id = $1 AND workspace_id = $2"#,
        id,
        state.workspace_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::Db)?;

    match incident {
        None => return Err(ApiError::NotFound("incident not found")),
        Some(ref i) if i.status == "resolved" => {
            return Err(ApiError::Conflict(
                "incident is already resolved and cannot be modified",
            ))
        }
        _ => {}
    }

    if let Some(ref new_status) = req.status {
        sqlx::query!(
            r#"UPDATE incidents SET status = ($1::text)::incident_status WHERE id = $2"#,
            new_status.as_str(),
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(ApiError::Db)?;
    }

    let upd = sqlx::query_as!(
        types::IncidentUpdateItem,
        r#"
        INSERT INTO incident_updates (incident_id, message)
        VALUES ($1, $2)
        RETURNING id, message, created_at
        "#,
        id,
        req.message
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::Db)?;

    tx.commit().await.map_err(ApiError::Db)?;

    Ok((StatusCode::CREATED, Json(types::ok(upd))))
}

pub async fn resolve_incident(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    AppJson(req): AppJson<types::ResolveIncidentReq>,
) -> Result<impl IntoResponse, ApiError> {
    validate_str(&req.message, "message", 10_000)?;

    let mut tx = state.db.begin().await.map_err(ApiError::Db)?;

    let resolved_at = req.resolved_at.unwrap_or_else(Utc::now);

    // Ensure incident exists, belongs to workspace, and is not already resolved
    let incident = sqlx::query!(
        r#"SELECT id, status as "status!: String" FROM incidents WHERE id = $1 AND workspace_id = $2"#,
        id,
        state.workspace_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(ApiError::Db)?;

    match incident {
        None => return Err(ApiError::NotFound("incident not found")),
        Some(ref i) if i.status == "resolved" => {
            return Err(ApiError::Conflict("incident is already resolved"))
        }
        _ => {}
    }

    sqlx::query!(
        r#"
        UPDATE incidents
        SET status = 'resolved',
            resolved_at = $1
        WHERE id = $2 AND workspace_id = $3
        "#,
        resolved_at,
        id,
        state.workspace_id
    )
    .execute(&mut *tx)
    .await
    .map_err(ApiError::Db)?;

    let upd = sqlx::query_as!(
        types::IncidentUpdateItem,
        r#"
        INSERT INTO incident_updates (incident_id, message)
        VALUES ($1, $2)
        RETURNING id, message, created_at
        "#,
        id,
        req.message
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(ApiError::Db)?;

    tx.commit().await.map_err(ApiError::Db)?;

    Ok(Json(types::ok(upd)))
}

/* ---------------- Public: Page JSON ---------------- */

pub async fn public_page(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    // Load page
    let page = sqlx::query!(
        r#"
        SELECT id, slug, title
        FROM status_pages
        WHERE workspace_id = $1 AND slug = $2 AND published = true
        LIMIT 1
        "#,
        state.workspace_id,
        slug
    )
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Db)?
    .ok_or(ApiError::NotFound("page not found"))?;

    // Services
    let services = sqlx::query_as!(
        types::PublicServiceItem,
        r#"
        SELECT name, status as "status!: String", updated_at
        FROM services
        WHERE workspace_id = $1
        ORDER BY position ASC, created_at ASC
        "#,
        state.workspace_id
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Db)?;

    let overall_status = overall_status_from_services(
        &services
            .iter()
            .map(|s| s.status.clone())
            .collect::<Vec<_>>(),
    );

    // Active incidents (not resolved)
    let active = sqlx::query_as!(
        types::IncidentListItem,
        r#"
        SELECT id, title, status as "status!: String", impact as "impact!: String", started_at, resolved_at
        FROM incidents
        WHERE workspace_id = $1
          AND status_page_id = $2
          AND status <> 'resolved'
        ORDER BY started_at DESC
        "#,
        state.workspace_id,
        page.id
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Db)?;

    // Batch-load all updates for active incidents in a single query.
    let active_ids: Vec<Uuid> = active.iter().map(|i| i.id).collect();
    let mut updates_by_incident: std::collections::HashMap<Uuid, Vec<types::IncidentUpdateItem>> =
        std::collections::HashMap::new();
    if !active_ids.is_empty() {
        let rows = sqlx::query!(
            r#"
            SELECT incident_id, id, message, created_at
            FROM incident_updates
            WHERE incident_id = ANY($1)
            ORDER BY incident_id, created_at ASC
            "#,
            &active_ids as &[Uuid]
        )
        .fetch_all(&state.db)
        .await
        .map_err(ApiError::Db)?;

        for row in rows {
            updates_by_incident
                .entry(row.incident_id)
                .or_default()
                .push(types::IncidentUpdateItem {
                    id: row.id,
                    message: row.message,
                    created_at: row.created_at,
                });
        }
    }

    let active_incidents = active
        .into_iter()
        .map(|inc| {
            let updates = updates_by_incident.remove(&inc.id).unwrap_or_default();
            types::PublicIncidentItem {
                id: inc.id,
                title: inc.title,
                status: inc.status,
                impact: inc.impact,
                started_at: inc.started_at,
                updates,
            }
        })
        .collect::<Vec<_>>();

    // Recent incidents (resolved)
    let recent_incidents = sqlx::query_as!(
        types::PublicRecentIncidentItem,
        r#"
        SELECT id, title, status as "status!: String", impact as "impact!: String", started_at, resolved_at
        FROM incidents
        WHERE workspace_id = $1
          AND status_page_id = $2
          AND status = 'resolved'
        ORDER BY started_at DESC
        LIMIT 10
        "#,
        state.workspace_id,
        page.id
    )
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Db)?;

    let resp = types::PublicPageResp {
        page: types::PublicPageInfo {
            slug: page.slug,
            title: page.title,
        },
        overall_status,
        services,
        active_incidents,
        recent_incidents,
    };

    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
    Ok((headers, Json(types::ok(resp))))
}
