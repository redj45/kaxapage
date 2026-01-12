// Copyright (C) 2025 KaxaPage
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// See LICENSE file or https://www.gnu.org/licenses/agpl-3.0.txt
// src/middleware/admin_auth.rs
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::api::error::ApiError;
use crate::app::AppState;

fn cookie_value<'a>(cookie_header: &'a str, name: &str) -> Option<&'a str> {
    cookie_header.split(';').find_map(|part| {
        let part = part.trim();
        let (k, v) = part.split_once('=')?;
        if k.trim() == name { Some(v.trim()) } else { None }
    })
}

pub async fn admin_auth(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    // Prefer HttpOnly cookie; fall back to Bearer for API clients / curl.
    let token = req
        .headers()
        .get("cookie")
        .and_then(|v| v.to_str().ok())
        .and_then(|h| cookie_value(h, "kp_admin").map(str::to_owned))
        .or_else(|| {
            req.headers()
                .get(axum::http::header::AUTHORIZATION)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.strip_prefix("Bearer "))
                .map(str::to_owned)
        });

    let ok = match token {
        Some(ref t) if !state.admin_token.is_empty() && !t.is_empty() => {
            crate::app::constant_time_eq(t.as_bytes(), state.admin_token.as_bytes())
        }
        _ => false,
    };

    if !ok {
        return ApiError::Unauthorized.into_response();
    }

    next.run(req).await
}
