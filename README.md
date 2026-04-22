# KaxaPage

[![CI](https://github.com/redj45/kaxapage/actions/workflows/ci.yml/badge.svg)](https://github.com/redj45/kaxapage/actions/workflows/ci.yml)
[![Release](https://github.com/redj45/kaxapage/actions/workflows/release.yml/badge.svg)](https://github.com/redj45/kaxapage/actions/workflows/release.yml)
[![Latest Release](https://img.shields.io/github/v/release/redj45/kaxapage?color=brightgreen)](https://github.com/redj45/kaxapage/releases/latest)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange.svg)](https://www.rust-lang.org)
[![PostgreSQL](https://img.shields.io/badge/postgres-16-blue.svg)](https://www.postgresql.org)

A self-hosted status page service built with Rust and Vue 3. Deploy it on your own server, manage services and incidents through a simple admin UI, and share a clean public status page with your users.

---

## Screenshots

| Public Status Page                      | Admin — Setup             |
| --------------------------------------- | ------------------------- |
| ![Public status page](docs/0public.png) | ![Setup](docs/1setup.png) |

| Admin — Services                | Admin — Incidents                 | Admin — Settings                |
| ------------------------------- | --------------------------------- | ------------------------------- |
| ![Services](docs/2services.png) | ![Incidents](docs/3incidents.png) | ![Settings](docs/4settings.png) |

---

## Features

- **Public status page** — real-time view of service health, active incidents, past incidents with full update history, incident history (last 90 days), and RSS feed
- **Service management** — create, reorder, and update service statuses (`operational`, `degraded`, `partial_outage`, `major_outage`, `maintenance`)
- **Incident lifecycle** — open incidents with impact levels, post timestamped updates, resolve with a final message
- **Resolved incident protection** — resolved incidents cannot be modified or re-resolved (returns `409`)
- **Single-binary deploy** — Rust backend embeds the compiled Vue admin SPA and static assets via `rust-embed`; no separate static file server needed
- **Token-based admin auth** — cookie-based session protected by a static `ADMIN_TOKEN` env var with per-IP rate limiting
- **Bootstrap API** — one-shot setup endpoint to create the workspace, status page, and initial services
- **Adaptive layout** — public page is 30% wide on desktop, full-width on mobile

---

## Tech Stack

| Layer    | Technology                      |
| -------- | ------------------------------- |
| Backend  | Rust, Axum 0.8, SQLx 0.8, Tokio |
| Database | PostgreSQL 16                   |
| Frontend | Vue 3, TypeScript, Vite         |
| Embed    | rust-embed (admin SPA + assets) |
| Testing  | axum-test 17, tokio test        |
| CI       | GitHub Actions                  |

---

## Quick Start

### Prerequisites

- Rust (stable, 1.75+)
- Node.js 20+
- PostgreSQL 16 (or Docker)

### 1. Clone and start the database

```sh
git clone https://github.com/redj45/kaxapage.git
cd kaxapage
docker compose up -d
```

### 2. Configure environment variables

```sh
cp .env.example .env
# Edit .env and set DATABASE_URL and ADMIN_TOKEN
```

Or export them directly:

```sh
export DATABASE_URL='postgres://kaxapage:kaxapage@localhost:5432/kaxapage'
export ADMIN_TOKEN='your-secret-token'
export RUST_LOG=info
```

### 3. Build and run

```sh
./run
```

The `run` script builds the admin SPA (`npm ci && npm run build`) and starts the Rust server. Migrations are applied automatically on startup.

- Public status page: `http://localhost:8080`
- Admin UI: `http://localhost:8080/admin`

### 4. Bootstrap your workspace

On first run, call the bootstrap endpoint once to create the workspace, status page, and initial services:

```sh
curl -X POST http://localhost:8080/api/v1/bootstrap \
  -H 'Content-Type: application/json' \
  -d '{
    "workspace_name": "Acme Corp",
    "page": { "slug": "acme", "title": "Acme Status" },
    "services": [
      { "name": "API",       "description": "REST API" },
      { "name": "Dashboard", "description": "Web dashboard" },
      { "name": "Database",  "description": "Primary database" }
    ]
  }'
```

Bootstrap is a one-shot operation — it returns `409` if a workspace already exists.

---

## Environment Variables

| Variable        | Required | Description                                                                 |
| --------------- | -------- | --------------------------------------------------------------------------- |
| `DATABASE_URL`  | yes      | PostgreSQL connection string                                                |
| `ADMIN_TOKEN`   | yes      | Static token used to authenticate admin sessions                            |
| `LISTEN_ADDR`   | no       | Bind address (default: `0.0.0.0:8080`)                                      |
| `COOKIE_SECURE` | no       | Set to `true` in production (HTTPS). Enables the `Secure` flag on the auth cookie. |
| `RUST_LOG`      | no       | Log filter, e.g. `info` or `kaxapage=debug`                                 |

See [`.env.example`](.env.example) for a template.

---

## Project Structure

```
kaxapage/
├── src/
│   ├── main.rs               # Entry point, server startup
│   ├── lib.rs                # Public re-exports for tests
│   ├── app.rs                # AppState, router, overall status helper
│   ├── db.rs                 # DB connect + migrate
│   ├── api/
│   │   ├── handlers.rs       # Admin REST API handlers
│   │   ├── types.rs          # Request / response types
│   │   ├── extractors.rs     # AppJson extractor — consistent 400/415 error responses
│   │   └── error.rs          # ApiError → HTTP response mapping
│   ├── routes/
│   │   └── public.rs         # Public HTML page, RSS feed, /healthz
│   └── middleware/
│       └── admin_auth.rs     # Cookie auth middleware
├── migrations/               # SQLx migration files
├── tests/
│   └── integration.rs        # Integration tests (require DATABASE_URL)
├── web/
│   └── admin/                # Vue 3 admin SPA (Vite)
├── docs/                     # Screenshots
├── Cargo.toml
├── Dockerfile
├── docker-compose.yml        # Local PostgreSQL via Docker
└── run                       # Local dev helper script
```

---

## API Overview

All admin endpoints require a valid `kp_admin` cookie (obtained via login).

### Auth

| Method | Path                   | Description          |
| ------ | ---------------------- | -------------------- |
| POST   | `/api/v1/admin/login`  | Set session cookie   |
| POST   | `/api/v1/admin/logout` | Clear session cookie |

### Services

| Method | Path                         | Description              |
| ------ | ---------------------------- | ------------------------ |
| GET    | `/api/v1/admin/services`     | List all services        |
| POST   | `/api/v1/admin/services`     | Create a service         |
| PATCH  | `/api/v1/admin/services/:id` | Update name/status/order |
| DELETE | `/api/v1/admin/services/:id` | Delete a service         |

### Incidents

| Method | Path                                  | Description                              |
| ------ | ------------------------------------- | ---------------------------------------- |
| GET    | `/api/v1/admin/incidents`             | List incidents (cursor-paginated)        |
| POST   | `/api/v1/admin/incidents`             | Create an incident                       |
| GET    | `/api/v1/admin/incidents/:id`         | Get incident with updates & services     |
| POST   | `/api/v1/admin/incidents/:id/updates` | Add an update (optionally change status) |
| POST   | `/api/v1/admin/incidents/:id/resolve` | Resolve the incident                     |

> Resolved incidents are immutable — `add_update` and `resolve` both return `409` if the incident is already resolved.

### Public

| Method | Path                         | Description             |
| ------ | ---------------------------- | ----------------------- |
| GET    | `/`                          | Public HTML status page |
| GET    | `/api/v1/public/pages/:slug` | Public JSON API         |
| GET    | `/rss.xml`                   | RSS incident feed       |
| GET    | `/healthz`                   | Health check            |

---

## Reference

### Incident Statuses

| Status          | Meaning                              |
| --------------- | ------------------------------------ |
| `investigating` | Issue detected, cause unknown        |
| `identified`    | Root cause identified                |
| `monitoring`    | Fix applied, watching for stability  |
| `resolved`      | Incident closed — no further changes |

### Service Statuses

| Status           | Meaning               |
| ---------------- | --------------------- |
| `operational`    | Fully working         |
| `degraded`       | Performance issues    |
| `partial_outage` | Some users affected   |
| `major_outage`   | Service unavailable   |
| `maintenance`    | Scheduled maintenance |

---

## Tests

### Unit tests (no database required)

```sh
cargo test --lib
```

Covers: `validate_slug`, `validate_str`, `validate_service_status`, `validate_incident_status`, `validate_incident_impact`, `overall_status_from_services`, `constant_time_eq`, `escape_html`, `status_badge`, `incident_badge`, `impact_label`.

**29 tests total.**

### Integration tests (require PostgreSQL)

```sh
DATABASE_URL='postgres://kaxapage:kaxapage@localhost:5432/kaxapage' \
ADMIN_TOKEN='test-token' \
cargo test --test integration
```

Covers full HTTP lifecycle via `axum-test` with a real database:

- Auth: login valid/invalid/empty, logout
- Pages: auth guard, list
- Services: full CRUD, validation errors, not found
- Incidents: full lifecycle, double resolve → `409`, update after resolve → `409`, validation errors, pagination
- Public: HTML `/`, JSON API, unknown slug → `404`, RSS feed
- Auth middleware: all admin endpoints without cookie → `401`

**26 tests total.**

---

## CI/CD

GitHub Actions runs on every push to `main` and on every pull request.

| Workflow      | Trigger             | Jobs                                                          |
| ------------- | ------------------- | ------------------------------------------------------------- |
| `ci.yml`      | push / PR to `main` | `fmt`, `clippy`, `build`, `unit-tests`, `integration-tests`   |
| `release.yml` | push tag `v*.*.*`   | builds Linux (musl) + macOS (aarch64), creates GitHub Release |

### Creating a release

```sh
git tag v0.2.0
git push origin v0.2.0
```

The release workflow automatically builds binaries for Linux x86_64 (static musl) and macOS aarch64, then publishes them as GitHub Release assets.

---

## Roadmap

See [ROADMAP.md](ROADMAP.md) for planned features and future direction.

---

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) before opening a pull request.

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this standard.

## Security

To report a security vulnerability, please follow the process described in [SECURITY.md](SECURITY.md). Do not open a public issue.

## License

The open-source version of KaxaPage is licensed under the
[GNU Affero General Public License v3.0](LICENSE).

For commercial use, SaaS deployment, or enterprise licensing,
see [COMMERCIAL_LICENSE.md](COMMERCIAL_LICENSE.md) or contact: hello@kaxapage.com
