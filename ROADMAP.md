# Roadmap

This document outlines what is planned, what is being considered, and what is intentionally out of scope for KaxaPage.

> Items are not listed in strict priority order. The roadmap reflects current thinking and may change based on community feedback.

---

## ✅ Done (v0.1.0)

- Public status page (HTML + JSON API)
- RSS feed for incidents
- Service management (CRUD, reorder, status)
- Incident lifecycle (create → update → resolve)
- Resolved incident immutability (`409 Conflict`)
- Single-binary deploy via `rust-embed`
- Token-based admin auth with per-IP rate limiting
- Bootstrap API for first-run setup
- Docker + Docker Compose support
- GitHub Actions CI (fmt, clippy, build, unit tests, integration tests)
- Release workflow (Linux musl + macOS aarch64 binaries)

---

## 🚧 In Progress / Up Next

### v0.2.0

- [ ] **Scheduled maintenance windows** — create maintenance events with start/end times that appear on the public page
- [ ] **Service grouping** — organize services into logical groups (e.g. "Core", "Integrations")
- [ ] **Public page customization** — configurable page title, logo URL, and custom footer text via admin UI

---

## 💡 Planned

### Notifications

- [ ] **Email notifications** — notify subscribers when an incident is opened or updated (SMTP-based, opt-in)
- [ ] **Webhook support** — POST incident events to a configurable URL (useful for Slack, Discord, PagerDuty, etc.)

### Monitoring

- [ ] **Uptime checks** — built-in HTTP health checks that automatically update service status and create incidents when a service goes down
- [ ] **Uptime history** — 90-day uptime percentage displayed on the public page per service

### Auth & Multi-tenancy

- [ ] **Multiple admin users** — invite additional administrators with their own credentials
- [ ] **API key management** — generate and revoke API keys from the admin UI instead of relying on a single static token

### Public Page

- [ ] **Subscriber sign-up** — let visitors subscribe to incident updates via email
- [ ] **Atom feed** — alongside the existing RSS feed
- [ ] **Embed widget** — a small JavaScript snippet for embedding current status into another site

### Deployment

- [ ] **Official Docker image** — publish a pre-built image to Docker Hub / GitHub Container Registry on every release
- [ ] **Windows binary** — add `x86_64-pc-windows-msvc` to the release workflow
- [ ] **ARM Linux binary** — add `aarch64-unknown-linux-musl` for Raspberry Pi and cloud ARM instances

### Developer Experience

- [ ] **OpenAPI / Swagger spec** — generated API documentation served at `/api/docs`
- [x] **`.env` example file** — `.env.example` committed to the repository for easier local setup

---

## 🚫 Out of Scope

These features are explicitly **not** planned for KaxaPage. The goal is to remain a focused, self-hosted, single-binary tool.

- **SaaS / hosted version** — KaxaPage is self-hosted only
- **Metrics and APM** — use Prometheus, Grafana, or similar dedicated tools for deep observability
- **Complex RBAC** — fine-grained role and permission management is out of scope; the admin is trusted
- **Mobile app** — the public page and admin SPA are designed to be fully responsive

---

## 💬 Have an Idea?

Open a [Feature Request](https://github.com/kaxapage/kaxapage/issues/new?template=feature_request.yml) or start a conversation in [GitHub Discussions](https://github.com/kaxapage/kaxapage/discussions).
