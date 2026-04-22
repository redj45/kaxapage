# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2025-01-01

### Added

- Public status page served at `/` displaying current service health and recent incidents
- RSS feed at `/rss.xml` for incident updates, consumable by any feed reader
- Admin single-page application (SPA) at `/admin` for managing services and incidents
- Services CRUD — create, read, update, and delete monitored services via the admin API
- Incident lifecycle management — create, update, and resolve incidents through the admin API
- Protection against modifying resolved incidents — returns `409 Conflict` when attempting to update a closed incident
- Bootstrap API endpoint to set up the initial admin token on a fresh installation
- Adaptive layout — sidebar-based two-column design on desktop (≈30 % sidebar / content) and full-width single-column layout on mobile
- "Powered by KaxaPage" footer on the public status page
- Unit test suite — 29 tests covering core domain logic and helper functions
- Integration test suite — 26 tests covering HTTP endpoints and database interactions

[Unreleased]: https://github.com/redj45/kaxapage/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/redj45/kaxapage/releases/tag/v0.1.0
