# Coworking Tooling

`coworker-portal` is the intranet portal for a coworking space. It lets members log in,
subscribe to services (vouchers for Wi-Fi/network access), generate invoice pdf via the existing
django-billjobs invoicing system, and manage shared meeting rooms.

The backend is a Rust/axum application; it serves a React/TypeScript frontend (built and
embedded at compile time) and shares its Postgres database with the existing Django
billing app (`billjobs_*` tables).

## Features

- **Authentication** — login with the existing `auth_user` accounts, JWT-based sessions,
  forgot/reset password by email, and member-to-member account invitations.
- **Service subscriptions & vouchers** — members buy a service (e.g. "1 Month full
  access", "10 days book"), which creates an invoice, provisions Wi-Fi vouchers on the
  Unify controller, and stores them locally with their live status.
- **Voucher management** — list bills, check live voucher status against Unify,
  download invoice/voucher PDFs (proxied from django-billjobs), and revoke vouchers.
- **Guest purchases** — visitors can buy a guest-available service without an account
  via a public `/buy` flow, with their own bill/voucher summary page.
- **Background sync** — scheduled jobs keep voucher status in sync with Unify and
  maintain a daily usage diary for monthly vouchers.
- **Meeting rooms & bookings** — list rooms, book time slots with overlap detection,
  per-room and global iCalendar feeds, and optional two-way sync with Google Calendar.

For the full domain model (entities, business rules, module boundaries, ports/adapters,
DB schema, external API contracts), see **[DOMAIN.md](DOMAIN.md)**. For coding
conventions and module layout, see **[CLAUDE.md](CLAUDE.md)**.

## Contributing / Local development

### Prerequisites

- Rust (edition 2024 toolchain) + `cargo`
- Node.js + `npm`
- Docker (for the local Postgres database)

### Setup

1. Start Postgres (fixtures are loaded automatically on first start):

   ```bash
   docker compose up -d
   ```

2. Create a `.env` file at the repo root with at least:

   ```bash
   DATABASE_URL=postgres://coworking:coworking@localhost:5432/coworking
   JWT_SECRET=some-local-secret
   UNIFY_MOCK=true   # avoids calling a real Unify controller locally
   ```

   See `src/*/config.rs` (or `DOMAIN.md`) for all available environment variables
   (SMTP, Django PDF proxy, Google Calendar sync, etc.) — they're optional for basic
   local development.

3. Run the backend (applies DB migrations automatically):

   ```bash
   cargo run
   ```

   The server listens on `http://localhost:3000` by default, with Swagger UI at
   `/swagger`.

4. For frontend development with hot reload, run the Vite dev server separately:

   ```bash
   cd frontend
   npm install
   npm run dev
   ```

   `cargo build`/`cargo run` also builds the frontend automatically and serves it from
   `public/` (see `build.rs`), so a standalone frontend dev server is only needed for
   fast iteration.

### Useful commands

```bash
cargo build          # Build the backend (also builds the frontend, see build.rs)
cargo test           # Run backend tests
cargo clippy         # Lint the backend
cargo fmt            # Format backend code

cd frontend
npm run lint         # Type-check the frontend
npm run build        # Production build → frontend/dist/
```

### Test accounts

The fixtures loaded from `fixtures/` provide these accounts:

| Username | Password |
|----------|----------|
| admin | adminpass123 |
| alice | alicepass123 |
| bob | bobpass123 |

To regenerate `fixtures.sql` after editing `examples/gen_fixtures.rs`:

```bash
cargo run --example gen_fixtures > fixtures.sql
```