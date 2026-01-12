// Copyright (C) 2025 KaxaPage
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// See LICENSE file or https://www.gnu.org/licenses/agpl-3.0.txt
pub mod error;
pub mod extractors;
pub mod handlers;
pub mod types;

use crate::app::AppState;
use axum::Router;

pub fn router(state: AppState) -> Router<AppState> {
    handlers::router(state)
}
