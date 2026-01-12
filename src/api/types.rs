// Copyright (C) 2025 KaxaPage
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// See LICENSE file or https://www.gnu.org/licenses/agpl-3.0.txt
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct ApiOk<T> {
    pub ok: bool,
    pub data: T,
}

#[derive(Debug, Serialize)]
pub struct ApiErr {
    pub ok: bool,
    pub error: ApiErrorBody,
}

#[derive(Debug, Serialize)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
}

pub fn ok<T: Serialize>(data: T) -> ApiOk<T> {
    ApiOk { ok: true, data }
}

pub fn err(code: impl Into<String>, message: impl Into<String>) -> ApiErr {
    ApiErr {
        ok: false,
        error: ApiErrorBody {
            code: code.into(),
            message: message.into(),
        },
    }
}

/* ---------------- String aliases for DB output types ---------------- */

pub type ServiceStatus = String; // from DB enum: operational|degraded|partial_outage|major_outage|maintenance
pub type IncidentStatus = String; // from DB enum: investigating|identified|monitoring|resolved
pub type IncidentImpact = String; // from DB enum: none|minor|major|critical

/* ---------------- Typed enums for API input validation ---------------- */

/// Validated service status for request bodies. Serde rejects unknown variants.
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ServiceStatusInput {
    Operational,
    Degraded,
    PartialOutage,
    MajorOutage,
    Maintenance,
}

impl ServiceStatusInput {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Operational => "operational",
            Self::Degraded => "degraded",
            Self::PartialOutage => "partial_outage",
            Self::MajorOutage => "major_outage",
            Self::Maintenance => "maintenance",
        }
    }
}

/// Validated incident status for request bodies. Serde rejects unknown variants.
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IncidentStatusInput {
    Investigating,
    Identified,
    Monitoring,
    Resolved,
}

impl IncidentStatusInput {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Investigating => "investigating",
            Self::Identified => "identified",
            Self::Monitoring => "monitoring",
            Self::Resolved => "resolved",
        }
    }
}

/// Validated incident impact for request bodies. Serde rejects unknown variants.
#[derive(Debug, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum IncidentImpactInput {
    None,
    Minor,
    Major,
    Critical,
}

impl IncidentImpactInput {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Minor => "minor",
            Self::Major => "major",
            Self::Critical => "critical",
        }
    }
}

/// Serde helper: distinguishes "field absent" (→ `None`) from "field: null" (→ `Some(None)`).
/// Use with `#[serde(default, deserialize_with = "maybe_null::deserialize")]`.
pub mod maybe_null {
    use serde::{Deserialize, Deserializer};

    pub fn deserialize<'de, T, D>(d: D) -> Result<Option<Option<T>>, D::Error>
    where
        T: Deserialize<'de>,
        D: Deserializer<'de>,
    {
        Option::deserialize(d).map(Some)
    }
}

/* ---------------- Auth ---------------- */

#[derive(Debug, Deserialize)]
pub struct LoginReq {
    pub token: String,
}

/* ---------------- Bootstrap ---------------- */

#[derive(Debug, Deserialize)]
pub struct BootstrapReq {
    pub workspace_name: String,
    pub page: BootstrapPage,
    #[serde(default)]
    pub services: Vec<BootstrapService>,
}

#[derive(Debug, Deserialize)]
pub struct BootstrapPage {
    pub slug: String,
    pub title: String,
}

#[derive(Debug, Deserialize)]
pub struct BootstrapService {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BootstrapResp {
    pub workspace_id: Uuid,
    pub status_page_id: Uuid,
    pub page_slug: String,
}

/* ---------------- Admin: Pages ---------------- */

#[derive(Debug, Serialize)]
pub struct AdminPageItem {
    pub id: Uuid,
    pub slug: String,
    pub title: String,
    pub published: bool,
}

#[derive(Debug, Deserialize)]
pub struct PatchPageReq {
    pub title: Option<String>,
    pub published: Option<bool>,
}

/* ---------------- Admin: Services ---------------- */

#[derive(Debug, Serialize)]
pub struct ServiceItem {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub position: i32,
    pub status: ServiceStatus,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateServiceReq {
    pub name: String,
    pub description: Option<String>,
    pub position: Option<i32>,
    pub status: Option<ServiceStatusInput>,
}

#[derive(Debug, Deserialize)]
pub struct PatchServiceReq {
    pub name: Option<String>,
    /// `None` = field absent (no change).
    /// `Some(None)` = explicit `null` (clears the description).
    /// `Some(Some(v))` = sets a new value.
    #[serde(default, deserialize_with = "maybe_null::deserialize")]
    pub description: Option<Option<String>>,
    pub position: Option<i32>,
    pub status: Option<ServiceStatusInput>,
}

/* ---------------- Admin: Incidents ---------------- */

#[derive(Debug, Serialize)]
pub struct IncidentListItem {
    pub id: Uuid,
    pub title: String,
    pub status: IncidentStatus,
    pub impact: IncidentImpact,
    pub started_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct IncidentUpdateItem {
    pub id: Uuid,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AffectedServiceItem {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug, Serialize)]
pub struct IncidentGetResp {
    pub incident: IncidentListItem,
    pub updates: Vec<IncidentUpdateItem>,
    pub affected_services: Vec<AffectedServiceItem>,
}

#[derive(Debug, Deserialize)]
pub struct CreateIncidentReq {
    pub status_page_id: Uuid,
    pub title: String,
    pub status: Option<IncidentStatusInput>,
    pub impact: Option<IncidentImpactInput>,
    pub started_at: Option<DateTime<Utc>>,
    pub message: String,
    #[serde(default)]
    pub service_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Serialize)]
pub struct CreateIncidentResp {
    pub incident_id: Uuid,
    pub update_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct CreateUpdateReq {
    pub message: String,
    pub status: Option<IncidentStatusInput>,
}

#[derive(Debug, Deserialize)]
pub struct ResolveIncidentReq {
    pub message: String,
    pub resolved_at: Option<DateTime<Utc>>,
}

/* ---------------- Public page ---------------- */

#[derive(Debug, Serialize)]
pub struct PublicPageResp {
    pub page: PublicPageInfo,
    pub overall_status: ServiceStatus,
    pub services: Vec<PublicServiceItem>,
    pub active_incidents: Vec<PublicIncidentItem>,
    pub recent_incidents: Vec<PublicRecentIncidentItem>,
}

#[derive(Debug, Serialize)]
pub struct PublicPageInfo {
    pub slug: String,
    pub title: String,
}

#[derive(Debug, Serialize)]
pub struct PublicServiceItem {
    pub name: String,
    pub status: ServiceStatus,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct PublicIncidentItem {
    pub id: Uuid,
    pub title: String,
    pub status: IncidentStatus,
    pub impact: IncidentImpact,
    pub started_at: DateTime<Utc>,
    pub updates: Vec<IncidentUpdateItem>,
}

#[derive(Debug, Serialize)]
pub struct PublicRecentIncidentItem {
    pub id: Uuid,
    pub title: String,
    pub status: IncidentStatus,
    pub impact: IncidentImpact,
    pub started_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}
