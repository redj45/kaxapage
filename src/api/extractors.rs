// Copyright (C) 2025 KaxaPage
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// See LICENSE file or https://www.gnu.org/licenses/agpl-3.0.txt
use axum::{
    extract::{rejection::JsonRejection, FromRequest, Request},
    http::StatusCode,
    Json,
};
use serde::de::DeserializeOwned;

use crate::api::types;

/// Drop-in replacement for `axum::Json` that maps serde deserialization failures
/// to `400 Bad Request` (instead of axum's default `422`) while preserving the
/// correct status code for other rejection types (415, body-size errors, etc.).
pub struct AppJson<T>(pub T);

impl<T, S> FromRequest<S> for AppJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = (StatusCode, Json<types::ApiErr>);

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match Json::<T>::from_request(req, state).await {
            Ok(Json(value)) => Ok(AppJson(value)),
            Err(rejection) => {
                let (status, msg) = match &rejection {
                    // Serde validation errors (e.g. unknown enum variant) → 400
                    JsonRejection::JsonDataError(_) | JsonRejection::JsonSyntaxError(_) => {
                        (StatusCode::BAD_REQUEST, rejection.to_string())
                    }
                    // Missing Content-Type → 415
                    JsonRejection::MissingJsonContentType(_) => {
                        (StatusCode::UNSUPPORTED_MEDIA_TYPE, rejection.to_string())
                    }
                    // Everything else (body read errors, etc.) → 400
                    _ => (StatusCode::BAD_REQUEST, rejection.to_string()),
                };
                Err((status, Json(types::err("bad_request", msg))))
            }
        }
    }
}
