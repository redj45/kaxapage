// Copyright (C) 2025 KaxaPage
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// See LICENSE file or https://www.gnu.org/licenses/agpl-3.0.txt
use crate::api::types;
use axum::{http::StatusCode, response::IntoResponse, Json};

pub enum ApiError {
    BadRequest(String),
    Unauthorized,
    NotFound(&'static str),
    Conflict(&'static str),
    Db(sqlx::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ApiError::Db(e) => {
                tracing::error!(error=%e, "db error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(types::err("db_error", "database error")),
                )
                    .into_response()
            }
            ApiError::BadRequest(m) => {
                (StatusCode::BAD_REQUEST, Json(types::err("bad_request", &m))).into_response()
            }
            ApiError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                Json(types::err("unauthorized", "unauthorized")),
            )
                .into_response(),
            ApiError::NotFound(m) => {
                (StatusCode::NOT_FOUND, Json(types::err("not_found", m))).into_response()
            }
            ApiError::Conflict(m) => {
                (StatusCode::CONFLICT, Json(types::err("conflict", m))).into_response()
            }
        }
    }
}
