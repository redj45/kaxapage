// Copyright (C) 2025 KaxaPage
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// See LICENSE file or https://www.gnu.org/licenses/agpl-3.0.txt
use crate::app::{overall_status_from_services, AppState};
use axum::{
    extract::State,
    http::{header, HeaderMap, StatusCode},
    response::{Html, IntoResponse},
};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use uuid::Uuid;

fn escape_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(c),
        }
    }
    out
}

fn fmt_dt(dt: DateTime<Utc>) -> String {
    dt.format("%Y-%m-%d %H:%M UTC").to_string()
}

fn status_badge(status: &str) -> (&'static str, &'static str) {
    // (css_class, label)
    match status {
        "major_outage" => ("bad", "Major outage"),
        "partial_outage" => ("warn", "Partial outage"),
        "degraded" => ("warn", "Degraded"),
        "maintenance" => ("accent", "Maintenance"),
        _ => ("ok", "Operational"),
    }
}

fn incident_badge(status: &str) -> (&'static str, &'static str) {
    match status {
        "resolved" => ("ok", "Resolved"),
        "monitoring" => ("accent", "Monitoring"),
        "identified" => ("warn", "Identified"),
        "investigating" => ("warn", "Investigating"),
        _ => ("warn", "Unknown"),
    }
}

fn impact_label(impact: &str) -> &'static str {
    match impact {
        "critical" => "Critical",
        "major" => "Major",
        "minor" => "Minor",
        "none" => "None",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── escape_html ──────────────────────────────────────────────────────────

    #[test]
    fn escape_html_no_special() {
        assert_eq!(escape_html("hello world"), "hello world");
        assert_eq!(escape_html(""), "");
        assert_eq!(escape_html("abc123"), "abc123");
    }

    #[test]
    fn escape_html_ampersand() {
        assert_eq!(escape_html("foo & bar"), "foo &amp; bar");
        assert_eq!(escape_html("&&"), "&amp;&amp;");
    }

    #[test]
    fn escape_html_lt_gt() {
        assert_eq!(escape_html("<script>"), "&lt;script&gt;");
        assert_eq!(escape_html("a < b > c"), "a &lt; b &gt; c");
    }

    #[test]
    fn escape_html_quotes() {
        assert_eq!(escape_html(r#"say "hi""#), "say &quot;hi&quot;");
        assert_eq!(escape_html("it's"), "it&#x27;s");
    }

    #[test]
    fn escape_html_mixed() {
        assert_eq!(
            escape_html(r#"<a href="test">O'Brien & Co</a>"#),
            "&lt;a href=&quot;test&quot;&gt;O&#x27;Brien &amp; Co&lt;/a&gt;"
        );
    }

    // ── status_badge ─────────────────────────────────────────────────────────

    #[test]
    fn status_badge_all_variants() {
        assert_eq!(status_badge("operational"), ("ok", "Operational"));
        assert_eq!(status_badge("degraded"), ("warn", "Degraded"));
        assert_eq!(status_badge("partial_outage"), ("warn", "Partial outage"));
        assert_eq!(status_badge("major_outage"), ("bad", "Major outage"));
        assert_eq!(status_badge("maintenance"), ("accent", "Maintenance"));
    }

    #[test]
    fn status_badge_unknown_fallback() {
        assert_eq!(status_badge(""), ("ok", "Operational"));
        assert_eq!(status_badge("unknown"), ("ok", "Operational"));
    }

    // ── incident_badge ───────────────────────────────────────────────────────

    #[test]
    fn incident_badge_all_variants() {
        assert_eq!(incident_badge("resolved"), ("ok", "Resolved"));
        assert_eq!(incident_badge("monitoring"), ("accent", "Monitoring"));
        assert_eq!(incident_badge("identified"), ("warn", "Identified"));
        assert_eq!(incident_badge("investigating"), ("warn", "Investigating"));
    }

    #[test]
    fn incident_badge_unknown_fallback() {
        assert_eq!(incident_badge(""), ("warn", "Unknown"));
        assert_eq!(incident_badge("random"), ("warn", "Unknown"));
    }

    // ── impact_label ─────────────────────────────────────────────────────────

    #[test]
    fn impact_label_all_variants() {
        assert_eq!(impact_label("critical"), "Critical");
        assert_eq!(impact_label("major"), "Major");
        assert_eq!(impact_label("minor"), "Minor");
        assert_eq!(impact_label("none"), "None");
    }

    #[test]
    fn impact_label_unknown_fallback() {
        assert_eq!(impact_label(""), "Unknown");
        assert_eq!(impact_label("high"), "Unknown");
    }
}

pub async fn healthz(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.db)
        .await
    {
        Ok(_) => (StatusCode::OK, "ok"),
        Err(e) => {
            tracing::error!(error=%e, "healthz db ping failed");
            (StatusCode::SERVICE_UNAVAILABLE, "db down")
        }
    }
}

pub async fn page_html(State(state): State<AppState>) -> impl IntoResponse {
    // Fetch the first published status page for this workspace.
    let page = match sqlx::query!(
        r#"
        SELECT id, slug, title
        FROM status_pages
        WHERE workspace_id = $1 AND published = true
        ORDER BY created_at ASC
        LIMIT 1
        "#,
        state.workspace_id
    )
    .fetch_optional(&state.db)
    .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error=%e, "db error: load page");
            return (StatusCode::INTERNAL_SERVER_ERROR, "db error").into_response();
        }
    };

    let page = match page {
        Some(p) => p,
        None => return (StatusCode::NOT_FOUND, "no published status page found").into_response(),
    };

    // 2) services
    let services = match sqlx::query!(
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
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error=%e, "db error: load services");
            return (StatusCode::INTERNAL_SERVER_ERROR, "db error").into_response();
        }
    };

    let overall = overall_status_from_services(
        &services
            .iter()
            .map(|s| s.status.clone())
            .collect::<Vec<_>>(),
    );

    // 3) active incidents
    let active = match sqlx::query!(
        r#"
        SELECT id, title, status as "status!: String", impact as "impact!: String", started_at
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
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error=%e, "db error: load active incidents");
            return (StatusCode::INTERNAL_SERVER_ERROR, "db error").into_response();
        }
    };

    // 4) recent incidents (resolved)
    let recent = match sqlx::query!(
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
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error=%e, "db error: load recent incidents");
            return (StatusCode::INTERNAL_SERVER_ERROR, "db error").into_response();
        }
    };

    // 5) history (last 90 days)
    let history = match sqlx::query!(
        r#"
        SELECT
        id,
        title,
        status as "status!: String",
        impact as "impact!: String",
        started_at,
        resolved_at
        FROM incidents
        WHERE workspace_id = $1
        AND status_page_id = $2
        AND started_at >= (NOW() AT TIME ZONE 'utc') - INTERVAL '90 days'
        ORDER BY started_at DESC
        "#,
        state.workspace_id,
        page.id
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error=%e, "db error: load history incidents");
            return (StatusCode::INTERNAL_SERVER_ERROR, "db error").into_response();
        }
    };

    // Batch-load updates for all active + recent incidents in a single query.
    let active_ids: Vec<Uuid> = active.iter().map(|i| i.id).collect();
    let recent_ids: Vec<Uuid> = recent.iter().map(|i| i.id).collect();
    let all_incident_ids: Vec<Uuid> = active_ids
        .iter()
        .chain(recent_ids.iter())
        .cloned()
        .collect();

    let mut updates_map: std::collections::HashMap<Uuid, Vec<(DateTime<Utc>, String)>> =
        std::collections::HashMap::new();
    if !all_incident_ids.is_empty() {
        let rows = match sqlx::query!(
            r#"
            SELECT incident_id, message, created_at
            FROM incident_updates
            WHERE incident_id = ANY($1)
            ORDER BY incident_id, created_at ASC
            "#,
            &all_incident_ids as &[Uuid]
        )
        .fetch_all(&state.db)
        .await
        {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(error=%e, "db error: load incident updates");
                return (StatusCode::INTERNAL_SERVER_ERROR, "db error").into_response();
            }
        };
        for row in rows {
            updates_map
                .entry(row.incident_id)
                .or_default()
                .push((row.created_at, row.message));
        }
    }

    let mut active_blocks = String::new();
    for inc in active {
        let updates = updates_map.remove(&inc.id).unwrap_or_default();

        let (cls, label) = incident_badge(&inc.status);
        let mut updates_html = String::new();
        for (created_at, message) in updates {
            updates_html.push_str(&format!(
                r#"<div class="upd"><div class="ts">{}</div><div class="msg">{}</div></div>"#,
                escape_html(&fmt_dt(created_at)),
                escape_html(&message),
            ));
        }

        active_blocks.push_str(&format!(
            r#"
            <div class="inc-card">
              <div class="inc-head">
                <div class="inc-title">{}</div>
                <span class="status-badge {}">{}</span>
              </div>
              <div class="inc-meta">
                <span class="meta-item">Impact: {}</span>
                <span class="meta-item">Started: {}</span>
              </div>
              <div class="updates">{}</div>
            </div>
            "#,
            escape_html(&inc.title),
            cls,
            label,
            escape_html(impact_label(&inc.impact)),
            escape_html(&fmt_dt(inc.started_at)),
            updates_html
        ));
    }

    let mut service_rows = String::new();
    for s in services {
        let (cls, label) = status_badge(&s.status);
        service_rows.push_str(&format!(
            r#"
            <div class="comp-row">
              <div class="comp-name">{}</div>
              <div class="comp-right">
                <span class="comp-ts">{}</span>
                <span class="status-badge {}">{}</span>
              </div>
            </div>
            "#,
            escape_html(&s.name),
            escape_html(&fmt_dt(s.updated_at)),
            cls,
            label,
        ));
    }

    let mut recent_rows = String::new();
    for r in recent {
        let updates = updates_map.remove(&r.id).unwrap_or_default();
        let mut updates_html = String::new();
        for (created_at, message) in updates {
            updates_html.push_str(&format!(
                r#"<div class="upd"><div class="ts">{}</div><div class="msg">{}</div></div>"#,
                escape_html(&fmt_dt(created_at)),
                escape_html(&message),
            ));
        }

        recent_rows.push_str(&format!(
            r#"
            <div class="inc-card inc-resolved">
              <div class="inc-head">
                <div class="inc-title">{}</div>
                <span class="status-badge badge-ok">Resolved</span>
              </div>
              <div class="inc-meta">
                <span class="meta-item">Impact: {}</span>
                <span class="meta-item">Started: {}</span>
                <span class="meta-item">Resolved: {}</span>
              </div>
              {}
            </div>
            "#,
            escape_html(&r.title),
            escape_html(impact_label(&r.impact)),
            escape_html(&fmt_dt(r.started_at)),
            escape_html(&r.resolved_at.map(fmt_dt).unwrap_or_else(|| "-".into())),
            if updates_html.is_empty() {
                String::new()
            } else {
                format!(r#"<div class="updates">{}</div>"#, updates_html)
            },
        ));
    }

    let mut by_day: BTreeMap<String, Vec<String>> = BTreeMap::new();

    for it in history {
        let day = it.started_at.format("%Y-%m-%d").to_string();
        let (cls, label) = incident_badge(&it.status);

        let resolved_cls = if it.status == "resolved" {
            " inc-resolved"
        } else {
            ""
        };
        let item = format!(
            r#"<div class="inc-card{}">
                <div class="inc-head">
                  <div class="inc-title">{}</div>
                  <span class="status-badge {}">{}</span>
                </div>
                <div class="inc-meta">
                  <span class="meta-item">Impact: {}</span>
                  <span class="meta-item">Started: {}</span>
                  {}
                </div>
            </div>"#,
            resolved_cls,
            escape_html(&it.title),
            cls,
            label,
            escape_html(impact_label(&it.impact)),
            escape_html(&fmt_dt(it.started_at)),
            match it.resolved_at {
                Some(dt) => format!(
                    r#"<span class="meta-item">Resolved: {}</span>"#,
                    escape_html(&fmt_dt(dt))
                ),
                None => r#"<span class="meta-item">Active</span>"#.to_string(),
            }
        );

        by_day.entry(day).or_default().push(item);
    }

    let mut history_html = String::new();
    for (day, items) in by_day.iter().rev() {
        history_html.push_str(&format!(
            r#"<div class="inc-day"><div class="inc-date">{}</div>{}</div>"#,
            escape_html(day),
            items.join("")
        ));
    }

    if history_html.is_empty() {
        history_html =
            r#"<div class="no-data">No incidents in the last 90 days.</div>"#.to_string();
    }

    let (overall_dot_cls, overall_banner_cls, overall_banner_label) = match overall.as_str() {
        "operational" => ("dot-ok", "banner-ok", "All Systems Operational"),
        "degraded" => ("dot-warn", "banner-warn", "Degraded Performance"),
        "partial_outage" => ("dot-warn", "banner-warn", "Partial Outage"),
        "major_outage" => ("dot-bad", "banner-bad", "Major Outage"),
        "maintenance" => ("dot-info", "banner-info", "Under Maintenance"),
        _ => ("dot-ok", "banner-ok", "All Systems Operational"),
    };

    let html = format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width,initial-scale=1" />
  <title>{title}</title>
  <meta name="description" content="Status page: {title}" />
  <style>
    :root {{
      --bg:     #0b0f14;
      --bg2:    #0a111a;
      --surface: rgba(255,255,255,.045);
      --surface2: rgba(255,255,255,.07);
      --border: rgba(255,255,255,.10);
      --text:   #e9eef5;
      --muted:  #a9b4c2;
      --muted2: #7f8a99;
      --mono:   ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
      --radius: 12px;
    }}
    *, *::before, *::after {{ box-sizing: border-box; margin: 0; padding: 0; }}
    html, body {{ height: 100%; }}
    body {{
      font-family: ui-sans-serif, system-ui, -apple-system, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
      background: var(--bg);
      color: var(--text);
      line-height: 1.55;
      display: flex;
      flex-direction: column;
      min-height: 100vh;
    }}
    a {{ color: inherit; text-decoration: none; }}

    /* ── background ── */
    .bg {{
      position: fixed; inset: 0; z-index: -1; pointer-events: none;
      background:
        radial-gradient(1200px 500px at 15% 10%, rgba(134,240,193,.09), transparent 55%),
        radial-gradient(900px  400px at 85% 20%, rgba(124,199,255,.08), transparent 55%),
        linear-gradient(180deg, var(--bg), var(--bg2));
    }}

    /* ── layout ── */
    .wrap {{ width: min(30%, 100% - 2rem); margin: 0 auto; padding: 2rem 1rem; flex: 1; }}
    @media (max-width: 900px) {{ .wrap {{ width: min(92%, 100% - 2rem); }} }}

    /* ── header ── */
    .sp-header {{ display: flex; justify-content: space-between; align-items: center; margin-bottom: 2rem; }}
    .sp-logo {{ display: flex; align-items: center; gap: 10px; }}
    .sp-logo-icon {{
      width: 36px; height: 36px; border-radius: 10px;
      background: linear-gradient(135deg, rgba(134,240,193,.22), rgba(124,199,255,.18));
      border: 1px solid rgba(255,255,255,.14);
      display: flex; align-items: center; justify-content: center;
      font-weight: 700; font-size: 14px; letter-spacing: .5px; color: var(--text);
    }}
    .sp-logo-name {{ font-size: 17px; font-weight: 600; color: var(--text); }}
    a.sp-logo {{ text-decoration: none; cursor: pointer; }}
    a.sp-logo:hover {{ opacity: .85; }}

    /* ── banner ── */
    .banner {{
      border-radius: var(--radius); padding: .9rem 1.2rem;
      display: flex; align-items: center; gap: 10px;
      margin-bottom: 2rem;
      border: 1px solid;
    }}
    .banner-ok   {{ background: rgba(52,211,153,.08);  border-color: rgba(52,211,153,.25); }}
    .banner-warn {{ background: rgba(251,191,36,.08);  border-color: rgba(251,191,36,.25); }}
    .banner-bad  {{ background: rgba(251,113,133,.08); border-color: rgba(251,113,133,.25); }}
    .banner-info {{ background: rgba(124,199,255,.08); border-color: rgba(124,199,255,.25); }}
    .banner-dot {{ width: 10px; height: 10px; border-radius: 50%; flex-shrink: 0; }}
    .dot-ok   {{ background: rgba(52,211,153,.90); box-shadow: 0 0 0 3px rgba(52,211,153,.20); }}
    .dot-warn {{ background: rgba(251,191,36,.90);  box-shadow: 0 0 0 3px rgba(251,191,36,.20); }}
    .dot-bad  {{ background: rgba(251,113,133,.90); box-shadow: 0 0 0 3px rgba(251,113,133,.20); }}
    .dot-info {{ background: rgba(124,199,255,.90); box-shadow: 0 0 0 3px rgba(124,199,255,.20); }}
    .banner-ok   .banner-text {{ color: rgba(52,211,153,.95);  font-weight: 600; font-size: 15px; }}
    .banner-warn .banner-text {{ color: rgba(251,191,36,.95);  font-weight: 600; font-size: 15px; }}
    .banner-bad  .banner-text {{ color: rgba(251,113,133,.95); font-weight: 600; font-size: 15px; }}
    .banner-info .banner-text {{ color: rgba(124,199,255,.95); font-weight: 600; font-size: 15px; }}

    /* ── section title ── */
    .section-title {{
      font-size: 11px; font-weight: 600; color: var(--muted2);
      text-transform: uppercase; letter-spacing: .09em;
      margin: 2rem 0 .6rem;
    }}

    /* ── component group ── */
    .comp-group {{
      background: var(--surface);
      border: 1px solid var(--border);
      border-radius: var(--radius);
      margin-bottom: 10px;
      overflow: hidden;
    }}
    .comp-row {{
      display: flex; align-items: center; justify-content: space-between;
      padding: .8rem 1.1rem;
      border-top: 1px solid rgba(255,255,255,.06);
      gap: 12px;
    }}
    .comp-row:first-child {{ border-top: none; }}
    .comp-group:hover {{ background: var(--surface2); }}
    .comp-name {{ font-size: 14px; color: var(--text); font-weight: 500; }}
    .comp-right {{ display: flex; align-items: center; gap: 10px; flex-shrink: 0; }}
    .comp-ts {{ font-family: var(--mono); font-size: 11px; color: var(--muted2); }}

    /* ── status badges ── */
    .status-badge {{
      font-size: 12px; font-weight: 500;
      padding: 3px 10px; border-radius: 999px;
      border: 1px solid;
      white-space: nowrap;
    }}
    .badge-ok    {{ background: rgba(52,211,153,.10);  border-color: rgba(52,211,153,.30);  color: rgba(134,240,193,.95); }}
    .badge-warn  {{ background: rgba(251,191,36,.10);  border-color: rgba(251,191,36,.30);  color: rgba(251,191,36,.95); }}
    .badge-bad   {{ background: rgba(251,113,133,.10); border-color: rgba(251,113,133,.30); color: rgba(251,113,133,.95); }}
    .badge-info  {{ background: rgba(124,199,255,.10); border-color: rgba(124,199,255,.30); color: rgba(124,199,255,.95); }}
    .badge-muted {{ background: rgba(255,255,255,.05); border-color: rgba(255,255,255,.12); color: var(--muted); }}

    /* ── divider ── */
    .divider {{ border: none; border-top: 1px solid rgba(255,255,255,.08); margin: 1.75rem 0; }}

    /* ── incidents ── */
    .inc-day {{ margin-bottom: 1.2rem; }}
    .inc-date {{ font-size: 12px; font-weight: 500; color: var(--muted2); margin-bottom: .4rem; font-family: var(--mono); }}
    .no-data {{ font-size: 14px; color: var(--muted2); }}
    .inc-card {{
      background: var(--surface);
      border: 1px solid var(--border);
      border-left: 3px solid rgba(251,113,133,.45);
      border-radius: 8px;
      padding: .75rem 1rem;
      margin-bottom: 8px;
    }}
    .inc-card.inc-resolved {{ border-left-color: rgba(52,211,153,.35); }}
    .inc-head {{ display: flex; justify-content: space-between; align-items: flex-start; gap: 12px; margin-bottom: 6px; }}
    .inc-title {{ font-size: 14px; font-weight: 600; color: var(--text); }}
    .inc-meta {{ display: flex; gap: 8px; flex-wrap: wrap; margin-top: 4px; }}
    .meta-item {{ font-size: 12px; color: var(--muted2); }}
    .meta-item + .meta-item::before {{ content: "·"; margin-right: 8px; }}

    /* ── updates inside incident ── */
    .updates {{ margin-top: 10px; border-top: 1px solid rgba(255,255,255,.07); }}
    .upd {{ padding: 8px 0; border-bottom: 1px solid rgba(255,255,255,.05); }}
    .upd:last-child {{ border-bottom: none; }}
    .upd .ts {{ font-family: var(--mono); font-size: 11px; color: var(--muted2); margin-bottom: 4px; }}
    .upd .msg {{ font-size: 13px; white-space: pre-wrap; line-height: 1.5; color: var(--muted); }}

    /* ── history ── */
    .hday {{ margin-bottom: 1.2rem; }}
    .hdate {{ font-family: var(--mono); font-size: 12px; color: var(--muted2); margin-bottom: .4rem; font-weight: 500; }}

    /* ── footer ── */
    .sp-footer {{ text-align: center; padding: 1.5rem 1rem; font-size: 12px; color: var(--muted2); opacity: .55; }}
  </style>
</head>
<body>
  <div class="bg"></div>

  <div class="wrap">

    <div class="sp-header">
      <a class="sp-logo" href="/" aria-label="Home">
        <div class="sp-logo-icon">KP</div>
        <span class="sp-logo-name">{title}</span>
      </a>
    </div>

    <div class="banner {overall_banner_cls}">
      <div class="banner-dot {overall_dot_cls}"></div>
      <span class="banner-text">{overall_banner_label}</span>
    </div>

    <div class="section-title">Components</div>
    <div class="comp-group">
      {service_rows}
    </div>

    <hr class="divider" />

    <div class="section-title">Active incidents</div>
    {active_blocks_or_empty}

    <hr class="divider" />

    <div class="section-title">Past incidents</div>
    {recent_rows_or_empty}

    <hr class="divider" />

    <div class="section-title">History (last 90 days)</div>
    {history_html}

  </div>

  <footer class="sp-footer">
    Powered by KaxaPage
  </footer>
</body>
</html>
"#,
        title = escape_html(&page.title),
        overall_banner_cls = overall_banner_cls,
        overall_dot_cls = overall_dot_cls,
        overall_banner_label = overall_banner_label,
        service_rows = service_rows,
        active_blocks_or_empty = if active_blocks.is_empty() {
            r#"<div class="no-data">No active incidents.</div>"#.to_string()
        } else {
            active_blocks
        },
        recent_rows_or_empty = if recent_rows.is_empty() {
            r#"<div class="no-data">No incidents yet.</div>"#.to_string()
        } else {
            recent_rows
        },
        history_html = history_html,
    );

    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());

    (headers, Html(html)).into_response()
}

pub async fn page_rss(State(state): State<AppState>) -> impl IntoResponse {
    // Find page
    let page = match sqlx::query!(
        r#"
        SELECT id, slug, title
        FROM status_pages
        WHERE workspace_id = $1 AND published = true
        ORDER BY created_at ASC
        LIMIT 1
        "#,
        state.workspace_id
    )
    .fetch_optional(&state.db)
    .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error=%e, "db error: load page for rss");
            return (StatusCode::INTERNAL_SERVER_ERROR, "db error").into_response();
        }
    };

    let page = match page {
        Some(p) => p,
        None => return (StatusCode::NOT_FOUND, "page not found").into_response(),
    };

    // Latest incidents (20)
    let items = match sqlx::query!(
        r#"
        SELECT id, title, status as "status!: String", impact as "impact!: String", started_at, resolved_at
        FROM incidents
        WHERE workspace_id = $1 AND status_page_id = $2
        ORDER BY started_at DESC
        LIMIT 20
        "#,
        state.workspace_id,
        page.id
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::error!(error=%e, "db error: load rss incidents");
            return (StatusCode::INTERNAL_SERVER_ERROR, "db error").into_response();
        }
    };

    let site = std::env::var("PUBLIC_BASE_URL").unwrap_or_default();
    let page_url = format!("{}/", site);
    let rss_url = format!("{}/rss.xml", site);

    let mut xml_items = String::new();
    for it in items {
        let link = format!("{}/", site);
        let pub_date = it.started_at.to_rfc2822();
        let desc = format!(
            "Status: {} | Impact: {} | Started: {}",
            it.status,
            it.impact,
            fmt_dt(it.started_at)
        );

        xml_items.push_str(&format!(
            r#"
            <item>
              <title>{}</title>
              <link>{}</link>
              <guid isPermaLink="false">{}</guid>
              <pubDate>{}</pubDate>
              <description><![CDATA[{}]]></description>
            </item>
            "#,
            escape_html(&it.title),
            escape_html(&link),
            escape_html(&it.id.to_string()),
            escape_html(&pub_date),
            desc
        ));
    }

    let xml = format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0">
  <channel>
    <title>{}</title>
    <link>{}</link>
    <description>{}</description>
    <language>en</language>
    <atom:link xmlns:atom="http://www.w3.org/2005/Atom" href="{}" rel="self" type="application/rss+xml" />
    {}
  </channel>
</rss>
"#,
        escape_html(&format!("{} — Incidents", page.title)),
        escape_html(&page_url),
        escape_html("Incident updates feed"),
        escape_html(&rss_url),
        xml_items
    );

    let mut headers = HeaderMap::new();
    headers.insert(
        header::CONTENT_TYPE,
        "application/rss+xml; charset=utf-8".parse().unwrap(),
    );
    headers.insert(header::CACHE_CONTROL, "no-cache".parse().unwrap());
    (headers, xml).into_response()
}
