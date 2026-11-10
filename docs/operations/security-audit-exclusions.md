# Security audit exclusions

KnitNPrint builds SQLx with PostgreSQL only. SQLx 0.8 also retains its optional
MySQL and SQLite drivers in Cargo.lock, even when they are not part of the enabled
dependency graph. Lockfile-wide auditing therefore reports advisories for packages
that are not compiled or shipped in the API.

The CI exclusions are limited to:

- [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071.html):
  RSA, retained by SQLx's disabled MySQL driver.
- [RUSTSEC-2026-0459](https://rustsec.org/advisories/RUSTSEC-2026-0459.html):
  libsqlite3-sys, retained by SQLx's disabled SQLite driver.

Before the audit, `scripts/check-postgres-only-dependencies.sh` checks the enabled
workspace dependency graph, including normal, build and development dependencies,
for all targets. It fails if MySQL, SQLite, libsqlite3-sys or RSA is enabled. If a
future change enables one, remove the corresponding exclusion and upgrade or
replace the vulnerable dependency before accepting that change.

All other Rust advisories, production npm auditing, repository dependency/secret
scans and API image vulnerability scans remain enabled. This does not exclude
SQLite advisories from the operating-system/container scanners.
