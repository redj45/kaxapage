// Copyright (C) 2025 KaxaPage
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// See LICENSE file or https://www.gnu.org/licenses/agpl-3.0.txt
use axum::{
    body::Body,
    http::{header, HeaderMap, StatusCode, Uri},
    response::Response,
};
use mime_guess::from_path;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "web/admin-dist"]
struct AdminDist;

pub async fn admin_handler(uri: Uri) -> Response<Body> {
    // /admin/assets/app.js -> "assets/app.js"
    let req_path = uri
        .path()
        .trim_start_matches("/admin")
        .trim_start_matches('/');
    let req_path = if req_path.is_empty() {
        "index.html"
    } else {
        req_path
    };

    // 1) Try exact file
    if let Some(asset) = AdminDist::get(req_path) {
        return respond_asset(req_path, asset.data.into_owned());
    }

    // 2) If request looks like a file (has extension), do NOT SPA-fallback.
    // This prevents returning index.html with text/css, text/javascript, etc.
    let looks_like_file = req_path.contains('.');
    if looks_like_file {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("asset not found"))
            .unwrap();
    }

    // 3) SPA fallback to index.html
    if let Some(asset) = AdminDist::get("index.html") {
        return respond_asset("index.html", asset.data.into_owned());
    }

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("admin index not found"))
        .unwrap()
}

fn respond_asset(path: &str, bytes: Vec<u8>) -> Response<Body> {
    let mime = from_path(path).first_or_octet_stream().to_string();

    let mut headers = HeaderMap::new();
    headers.insert(header::CONTENT_TYPE, mime.parse().unwrap());

    // Cache static assets aggressively
    if path.starts_with("assets/") || (path.contains('.') && path != "index.html") {
        headers.insert(
            header::CACHE_CONTROL,
            "public, max-age=31536000, immutable".parse().unwrap(),
        );
    } else {
        headers.insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
    }

    let mut resp = Response::new(Body::from(bytes));
    *resp.status_mut() = StatusCode::OK;
    *resp.headers_mut() = headers;
    resp
}
