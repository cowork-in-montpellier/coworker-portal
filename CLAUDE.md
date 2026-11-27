# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`coworker-portal` is a Rust project (edition 2024) with a planned frontend component.

This application should be a intranet portal with an ui for coworking user to subscribe to services.
Those service will be registered as invoice in a database, will also create vouchers on a unify router via api and generate a voucher pdf.

Services will define the number of vouchers, the time of them for each invoice.


## Commands

### Rust backend

```bash
cargo build          # Build the project
cargo run            # Run the application
cargo test           # Run all tests
cargo test <name>    # Run a specific test by name
cargo clippy         # Lint
cargo fmt            # Format code
```

Mandatory libary
 - axum
 - tokio
 - anyhow

The cargo build should be extended to build the frontend and copy the dist into a resources folder to be exposed.

### Frontend (Vite + React + TypeScript)

```bash
cd frontend
npm install      # Install dependencies
npm run dev      # Dev server (standalone, hot reload)
npm run build    # Production build → frontend/dist/
npm run lint     # Type check
```

Mandatory libraries: `fp-ts` (functional patterns), `zod` (schema validation).

The frontend is served by the Rust backend from the `resources/` directory. The `build.rs` script runs `npm install && npm run build` automatically during `cargo build` and copies `frontend/dist/` → `public/`.

## Architecture

- `src/main.rs` — Rust entry point
- `frontend/` — Frontend application (not yet initialized)

## Database conventions

This application shares a PostgreSQL database with an external Django billing app (`billjobs_*` tables). To avoid naming conflicts:

**All app-owned tables must be prefixed with `portal_`.**

- Current app-owned tables: `portal_service`, `portal_voucher`, `portal_guest_bill`
- External tables (never rename): `billjobs_bill`, `billjobs_billline`, `billjobs_service`, `billjobs_userprofile`, `auth_user`

When adding a new table, always use the `portal_` prefix.

### SUTOM and calendar databases

The SUTOM game and the room calendar are kept apart from billing, each in its own SQLite
file, opened through `src/sqlite.rs`:

| Module | URL variable (default) | Migrations |
|---|---|---|
| `sutom` | `SUTOM_DATABASE_URL` (`sqlite:data/sutom.db`) | `migrations_sutom/` |
| `calendar` | `CALENDAR_DATABASE_URL` (`sqlite:data/calendar.db`) | `migrations_calendar/` |

Their tables need no prefix. Never add SUTOM or calendar tables to PostgreSQL, and never
touch billing data from `src/sutom/` or `src/calendar/`: SUTOM gets players' names through
the `sutom::ports::PlayerDirectory` port.
