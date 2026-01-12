# Contributing to KaxaPage

Thank you for considering contributing to KaxaPage! This document explains how to get your development environment up and running, how to run the test suites, and what to keep in mind when opening a pull request.

---

## Table of Contents

1. [Prerequisites](#prerequisites)
2. [Getting Started](#getting-started)
3. [Project Structure](#project-structure)
4. [Running Tests](#running-tests)
5. [Code Style](#code-style)
6. [Submitting Changes](#submitting-changes)
7. [Reporting Bugs](#reporting-bugs)

---

## Prerequisites

Make sure the following tools are installed before you begin:

| Tool                    | Minimum version | Notes                                             |
| ----------------------- | --------------- | ------------------------------------------------- |
| Rust (stable)           | 1.75+           | Install via [rustup](https://rustup.rs/)          |
| Node.js                 | 20+             | Required to build the admin SPA                   |
| PostgreSQL              | 16              | Or use the provided Docker Compose setup          |
| Docker + Docker Compose | any recent      | Optional, but the easiest way to spin up Postgres |

---

## Getting Started

### 1. Fork and clone

```sh
# Fork the repository on GitHub, then:
git clone https://github.com/<your-username>/kaxapage.git
cd kaxapage
```

### 2. Start the database

The repository ships with a `docker-compose.yml` file that starts a local PostgreSQL 16 instance:

```sh
docker compose up -d
```

If you prefer a local Postgres installation, create a database manually and note the connection string.

### 3. Configure environment variables

Set the required environment variables:

```sh
export DATABASE_URL='postgres://kaxapage:kaxapage@localhost:5432/kaxapage'
export ADMIN_TOKEN='your-secret-token'
export RUST_LOG=info
```

Key variables:

| Variable       | Description                                                                             |
| -------------- | --------------------------------------------------------------------------------------- |
| `DATABASE_URL` | PostgreSQL connection string, e.g. `postgres://kaxapage:secret@localhost:5432/kaxapage` |
| `ADMIN_TOKEN`  | Secret token used to authenticate admin API requests                                    |
| `LISTEN_ADDR`  | Address and port to listen on (default: `0.0.0.0:8080`)                                 |
| `COOKIE_SECURE`| Set to `true` in production (HTTPS) to enable `Secure` flag on the auth cookie         |

### 4. Run the application

The `run` script builds the frontend, applies migrations, and starts the server in one step:

```sh
./run
```

The public status page is now available at `http://localhost:8080` and the admin SPA at `http://localhost:8080/admin`.

---

## Project Structure

```
kaxapage/
├── src/               # Rust application source (handlers, models, routes)
├── web/admin/         # Admin SPA (Vue 3 + TypeScript + Vite)
├── migrations/        # SQLx database migrations (applied in order)
├── tests/             # Integration tests (Rust, run against a live database)
├── build.rs           # Build script (embeds admin SPA assets)
├── docker-compose.yml # Docker Compose configuration for local Postgres
└── run                # Helper script: builds frontend, runs migrations, starts server
```

---

## Running Tests

### Unit tests

Unit tests live alongside the source code in `src/` and do not require a running database:

```sh
cargo test --lib
```

### Integration tests

Integration tests are located in `tests/` and require a live PostgreSQL database. Set `DATABASE_URL` to a test database before running them (a separate database from your development one is recommended so it can be wiped freely):

```sh
DATABASE_URL='postgres://kaxapage:kaxapage@localhost:5432/kaxapage' \
ADMIN_TOKEN='test-token' \
cargo test --test integration
```

The test harness applies migrations automatically at the start of each run.

---

## Code Style

### Formatting

All Rust code must be formatted with the standard formatter before committing:

```sh
cargo fmt
```

### Linting

The project uses Clippy with default settings. Fix all warnings before opening a PR:

```sh
cargo clippy --all-targets -- -D warnings
```

We follow standard Rust idioms. Avoid `unwrap()` / `expect()` in production paths unless a panic is genuinely the correct behaviour — prefer `?` and proper error propagation instead.

---

## Submitting Changes

### Branch naming

Create a feature branch from `main` using one of the following prefixes:

| Prefix   | When to use                                                        |
| -------- | ------------------------------------------------------------------ |
| `feat/`  | New feature or enhancement, e.g. `feat/email-notifications`        |
| `fix/`   | Bug fix, e.g. `fix/rss-encoding`                                   |
| `chore/` | Maintenance, dependency updates, tooling, e.g. `chore/update-sqlx` |

### Pull request description

A good PR description includes:

- **What** — a short summary of the change
- **Why** — the motivation or problem being solved
- **How** — a brief explanation of the approach taken
- **Testing** — how you verified the change works (new tests, manual steps, etc.)
- A reference to the related issue, if applicable (e.g. `Closes #42`)

### Checklist before opening a PR

- [ ] `cargo fmt` has been run and there are no formatting differences
- [ ] `cargo clippy --all-targets -- -D warnings` passes with no warnings
- [ ] `cargo test --lib` passes
- [ ] `cargo test --test integration` passes against a test database
- [ ] New behaviour is covered by tests where practical
- [ ] `CHANGELOG.md` has been updated under `[Unreleased]`

---

## Reporting Bugs

Please use the [GitHub issue tracker](https://github.com/kaxapage/kaxapage/issues) to report bugs. Before opening a new issue, search existing issues to avoid duplicates.

When filing a bug report, use the **Bug Report** issue template and include:

- A clear, descriptive title
- Steps to reproduce the problem
- Expected behaviour vs. actual behaviour
- KaxaPage version or commit hash
- Relevant log output or error messages (`RUST_LOG=debug` for verbose output)
- Environment details (OS, Postgres version, Rust version)

For **security vulnerabilities**, please do **not** open a public issue — see [SECURITY.md](SECURITY.md) instead.
