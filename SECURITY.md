# Security Policy

## Supported Versions

Only the latest released version of KaxaPage receives security fixes.

| Version | Supported |
| ------- | --------- |
| latest  | ✅        |
| older   | ❌        |

## Reporting a Vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

If you discover a security vulnerability, please report it through one of the following private channels:

- **GitHub Security Advisory:** use [Report a Vulnerability](https://github.com/kaxapage/kaxapage/security/advisories/new) — this creates a private draft advisory visible only to maintainers

Please include as much detail as possible in your report:

- A description of the vulnerability and its potential impact
- Steps to reproduce the issue (proof-of-concept code or curl commands are welcome)
- The version of KaxaPage you tested against
- Any suggested mitigations, if you have them

## What to Expect After Reporting

| Milestone                                                                       | Target timeframe                   |
| ------------------------------------------------------------------------------- | ---------------------------------- |
| Acknowledgement of your report                                                  | Within **48 hours**                |
| Confirmation of the vulnerability (or explanation why it is not considered one) | Within **7 days**                  |
| Security fix released                                                           | Within **90 days** of confirmation |

We will keep you informed of progress throughout the process and credit you in the release notes (unless you prefer to remain anonymous).

If a fix takes longer than 90 days for reasons beyond our control, we will notify you and agree on a revised timeline before any public disclosure.

## Known Security Considerations

### ADMIN_TOKEN strength

The `ADMIN_TOKEN` environment variable is the sole credential protecting all write operations on the admin API (`/admin/*`). You **must** use a long, randomly generated token in production — we recommend at least 32 bytes of cryptographically random data, e.g.:

```sh
openssl rand -hex 32
```

Never commit the token to source control or expose it in logs.

### Deploy behind a reverse proxy

KaxaPage is designed to run behind a reverse proxy (nginx, Caddy, Traefik, etc.). The built-in HTTP server does **not** perform TLS termination, rate-limiting, or IP filtering. Place a hardened reverse proxy in front of KaxaPage before exposing it to the internet.

### HTTPS is mandatory in production

Always serve KaxaPage over HTTPS in production environments. Transmitting the `ADMIN_TOKEN` over plain HTTP exposes it to network interception. Configure TLS at the reverse proxy layer and redirect all HTTP traffic to HTTPS.

### Database access

The PostgreSQL connection string (`DATABASE_URL`) grants full read/write access to the KaxaPage database. Keep it secret, restrict database user permissions to only the tables KaxaPage requires, and ensure the database is not reachable from the public internet.
