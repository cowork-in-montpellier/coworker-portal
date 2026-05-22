# Domain design

## Core aggregates

User : A user of the space
```typescript
type User = {
    id: int
    username: string
    password: string
    firstname: string
    lastName: string
    email: string
    billingAddress: string
}
```

Service : A commercial offer that user subscribe. Example : (10 days coupons, 1 Month full access, 1 Month 10 days access, ...)
This app owns the `service` table. Multiple services can map to the same `externalServiceId`, allowing different voucher specs for the same billing product.
```typescript
type Service = {
    id: int
    name: string
    description: string
    price: float
    voucherSpec: VoucherSpec
    externalServiceId: int  // references billjobs_service.id — used only when writing billjobs_billline
    isAvailable: boolean
}

type VoucherSpec = 
    | {kind: "Monthly"} // duration for Monthly service is to create one voucher valid for 30 days, expiring at 23:59:59 on the 30th day from creation (e.g. created April 15 → expires May 15 at midnight)
    | {
        kind: "Book"
        amount: int
        duration: int
    } 
```

Bill: A user buy of a service. When reading bills from the external system, a bill may reference a service that is not known to this application (created before this app existed, or via a removed service). The read model therefore distinguishes two variants:

```typescript
type Bill =
    | ManagedBill    // bill line maps to a service owned by this app
    | UnmanagedBill  // bill line references an unknown external service

type ManagedBill = {
    kind: "Managed"
    id: int
    number: string   // format: FYYYYMMNNN — see LastNumberCompute below
    user: User       // ref
    service: Service // ref — our internal service, resolved via external_service_id
    date: DateTime
    amount: float
    isPaid: boolean  // read from billjobs_bill."isPaid"; always false at creation
    issuerAddress: string
    billingAddress: string
    vouchers: [Voucher] // ref

    // invariant: number is computed at creation
    // invariant: date is computed at creation as system time
    // invariant: amount is a copy of service.price at creation time
    // invariant: isPaid is owned by the external system — this app never writes it
    // invariant: once created Bill is immutable from this app's perspective
}

type UnmanagedBill = {
    kind: "Unmanaged"
    id: int
    number: string
    date: DateTime
    amount: float
    isPaid: boolean  // read from billjobs_bill."isPaid"
    // no service ref — the bill line's service_id does not match any service.external_service_id
    // no vouchers — only bills created by this app have vouchers in our voucher table
}
```

> **Resolution rule:** when loading a bill, a correlated subquery looks for a `billjobs_billline` row whose `service_id` matches any `service.external_service_id`. The first match wins (`LIMIT 1`). If no match is found the bill is `Unmanaged`. Multi-line bills (created externally) are handled safely by this rule — only the first managed line is used.

Voucher: Represent access coupon a User can use in the coworking space.
Vouchers are persisted locally after being provisioned on Unify at invoice creation time. The stored `unifyId` enables later validity checks against the Unify edge without re-querying by note.
```typescript
type Voucher = {
    id: int           // local DB id
    unifyId: string   // Unify _id (MongoDB ObjectId) — immutable reference
    code: string      // 10-digit code from Unify, display as XXXXX-XXXXX
    createdAt: DateTime
    duration: int     // in hours
    status: VoucherStatus

    // invariant: unifyId and code are set at creation from Unify response and never updated
    // invariant: createdAt is computed at creation as system time
    // invariant: duration unit is always hours
    // invariant: duration is computed based on Service voucher spec — see MonthlyVoucherDuration
    // invariant: status is refreshed by the validity check command, not at creation
}

type VoucherStatus =
    | { kind: "Valid" }
    | { kind: "Used" }
    | { kind: "Expired" }
    | { kind: "Unknown" }  // Unify unreachable or voucher not found
```

## Business rules

### Monthly voucher duration derivation

**MonthlyVoucherDuration** — Duration computation for `Monthly` vouchers

When creating a voucher for a `Monthly` service, the duration is exactly 30 days, expiring at 23:59:59 on the 30th day from the creation date.

Rule:
- `expiryDay = creationDate + 30 days`
- `end = expiryDay at 23:59:59 UTC`
- `duration = ceil((end - createdAt) in hours)`

Example: created on April 15th at 14:00 UTC → expiry day is May 15th → end is May 15th 23:59:59 UTC → duration ≈ 730 hours.

> **Invariant:** the duration is computed at creation time and frozen.

### Bill last number compute

**LastNumberCompute** — Bill number generation rule

Format: `F` + `YYYYMM` + `NNN`
- `F` — fixed prefix (Facture)
- `YYYYMM` — year and month of the bill date
- `NNN` — 3-digit zero-padded incremental counter

The counter is computed at creation time by fetching the last saved bill and incrementing its counter:
- If no bill exists yet → `001`
- Otherwise → parse the last 3 chars of the latest bill number as an integer, increment by 1, zero-pad to 3 digits

The counter is **global** (not per-month): a new month does not reset it. Example sequence: `F202604003` → `F202605004`.

> **Invariant:** `number` is immutable once set. It must be assigned at creation and never updated.
> **Caution:** the increment must be computed atomically (DB sequence or row lock) to prevent duplicates under concurrent bill creation.


## Edges


### Postgres — owned by this app

All int are stored as int4.

This app owns the schema and data for:

**`portal_service`** → `Service`
| Column | Type | Notes |
|--------|------|-------|
| `id` | int4 | |
| `name` | varchar(256) | |
| `description` | text | |
| `price` | float8 | |
| `kind` | varchar(10) | `'Monthly'` or `'Book'` |
| `amount` | int4 | null for Monthly; number of vouchers for Book |
| `duration` | int4 | null for Monthly; hours per voucher for Book |
| `external_service_id` | int4 | references billjobs_service.id |
| `is_available` | boolean | filter on listing |
| `is_guest_available` | boolean | default false; must be true to appear in guest service list |

**`portal_voucher`** → `Voucher`
| Column | Type | Notes |
|--------|------|-------|
| `unify_id` | varchar(50) | primary key; Unify ObjectId |
| `bill_id` | int4 | FK → billjobs_bill.id |
| `unify_create_time` | int8 | Unix timestamp from Unify create response |
| `code` | varchar(10) | 10-digit code; displayed as XXXXX-XXXXX |
| `created_at` | timestamptz | |
| `duration` | int4 | hours |
| `status` | varchar(10) | mutable; updated by validity check |
| `active_days` | date[] | append-only diary of dates the voucher had at least one active guest; empty array for non-monthly vouchers |

**`portal_guest_bill`** — guest token → bill mapping
| Column | Type | Notes |
|--------|------|-------|
| `guest_token` | uuid | primary key; randomly generated at guest bill creation |
| `bill_id` | int4 | FK → billjobs_bill.id |

### Postgres — owned by external app (django-billjobs)

> **Anti-pattern:** we directly access the database of the existing invoicing app (django-billjobs). This is intentional and temporary — the target is to switch to an API contract later.

This app reads from (no schema ownership):

**`auth_user`** → `User`
| Column | Type | Notes |
|--------|------|-------|
| `id` | int4 | |
| `username` | varchar(150) | |
| `password` | varchar(128) | Django PBKDF2-SHA256 format |
| `first_name` | varchar(150) | |
| `last_name` | varchar(150) | |
| `email` | varchar(254) | |
| `is_active` | boolean | filter on login |

**`billjobs_service`** — referenced by `service.external_service_id`; no longer read directly by this app
| Column | Type | Notes |
|--------|------|-------|
| `id` | int4 | only used as FK target in billjobs_billline |
| `reference` | varchar(5) | |

This app writes to (anti-pattern, compatible with shared invoicing schema):

**`billjobs_bill`** → `Bill`
| Column | Type | Notes |
|--------|------|-------|
| `id` | int4 | |
| `number` | varchar(16) | unique, see LastNumberCompute |
| `user_id` | int4 | FK → auth_user.id |
| `billing_date` | date | set at creation |
| `amount` | float8 | copy of service.price |
| `issuer_address` | varchar(1024) | |
| `billing_address` | varchar(1024) | copied from user profile |
| `isPaid` | boolean | always false at creation; updated by external system only |

**`billjobs_billline`** → persistence detail for `Bill`, not in core domain
| Column | Type | Notes |
|--------|------|-------|
| `id` | int4 | |
| `bill_id` | int4 | FK → billjobs_bill.id |
| `service_id` | int4 | FK → billjobs_service.id |
| `quantity` | int2 | always 1 |
| `total` | float8 | = amount (service.price at creation) |
| `note` | varchar(1024) | left empty |


### Unify

Vouchers are managed by Unify via a cookie-authenticated API. The app must login first and carry the `unifises` session cookie on every request.

#### Authentication
`POST /api/login` → receive `unifises` cookie → attach to all subsequent calls.

#### Create vouchers
`POST /api/s/{site}/cmd/hotspot`

Domain → Unify field mapping:

| Domain | Unify field | Notes |
|--------|-------------|-------|
| `VoucherSpec.amount` (Book) / `1` (Monthly) | `n` | number of vouchers to generate |
| `VoucherSpec` duration | `expire_number` | hours value (see below) |
| *(fixed)* | `expire_unit` | always `60` (hour multiplier) |
| *(fixed)* | `quota` | always `2` (phone + computer) |
| `FYYYYMMNNN_FirstName` | `note` | bill number + `_` + user firstname |

Duration mapping per spec kind:
- `Book`: `expire_number = VoucherSpec.duration` (already in hours)
- `Monthly`: `expire_number = MonthlyVoucherDuration` (hours until end of month)

The response only returns a `create_time` Unix timestamp — **not** the voucher codes. The `create_time` must be used immediately to retrieve the batch (see below).

#### Retrieve vouchers
`POST /api/s/{site}/stat/voucher` with body `{ "create_time": <unix_timestamp> }`

Returns all vouchers created at or after that timestamp. Filter by `note` matching `FYYYYMMNNN_FirstName` to isolate the batch for a given bill.

Unify → Domain field mapping:

| Unify field | Domain | Notes |
|-------------|--------|-------|
| `_id` | `Voucher.unifyId` | MongoDB ObjectId |
| `code` | `Voucher.code` | 10 digits, display as `XXXXX-XXXXX` |
| `create_time` | `Voucher.createdAt` | Unix timestamp → DateTime |
| `duration` | `Voucher.duration` | Unify stores minutes; convert to hours (`/ 60`) |
| `status` | `Voucher.status` | `VALID_ONE`/`VALID_MULTI` → Valid, `USED_MULTIPLE` → Used |

#### Two-step creation flow
1. Call create → capture `create_time` from response
2. Call list with `create_time` → filter by `note` → map to `Voucher` domain objects


## Features

### Login as a user
As a coworker I want to connect to this application.

Acceptance criteria :
 - User can connect with a user / password
 - Logged user have a token that provide authentication for all other features

### Create a bill
As a logged user I would like to create a bill for myself.
I provide the service type I want and the application executes the following steps in a single transaction:
 - Receive a bill creation command with the service type
 - Create the bill (number, date, amount snapshot)
 - Create the vouchers on Unify
 - Store the bill and vouchers
 - Return the bill with all computed information

Acceptance:
 - Bill number follows LastNumberCompute
 - Vouchers are created on Unify with the correct duration and note
 - Vouchers are stored locally with status Valid
 - The bill is always created for the authenticated user — no impersonation
 - If Unify voucher creation fails, the bill is not persisted (DB transaction rolled back)

### List my bills
As a logged user I can see my own bills with pagination and filtering.

Acceptance:
 - Results are scoped to the authenticated user only
 - Supports offset/limit pagination
 - Filterable by date range and bill number

### Generate voucher PDF
As a logged user I can generate a voucher PDF for a bill containing the codes and duration in a compact way.

> **Template TBD** — the PDF layout and content (beyond voucher code + duration) will be defined in a future iteration. Implementation should keep the rendering logic isolated behind a template abstraction so it can be swapped without touching the domain.

### Voucher check
As a logged user I can get the live status of vouchers for a bill by querying Unify directly.

Acceptance:
 - Status is fetched live from Unify using the stored `unifyId` for each voucher
 - Local `Voucher.status` is updated in DB after the check
 - Returns current Unify status mapped to `VoucherStatus` for each voucher of the bill

### Guest purchase
A visitor can buy a service without a coworking account via a public `/buy` route.

**Flow:**
1. Visitor accesses `/buy` (no auth required)
2. Selects a service from the guest-available service list
3. Optionally provides billing name and address
4. Submits → bill created in backend as the configured generic guest user
5. Redirected to `/buy/summary/:guestToken` showing the bill, vouchers, and download buttons

**Key design decisions:**
- Services must be explicitly opted-in with `is_guest_available = true`
- Bill is owned by `GUEST_USER_ID` (a generic Django `auth_user` record)
- A `guest_token` UUID is generated at creation and stored in the `portal_guest_bill` table (separate from `billjobs_bill` to avoid modifying the external app's schema). It is the only access credential for subsequent guest operations — sequential integer bill IDs are never exposed publicly
- Customer name: if provided in the form, it is prepended as the first line of `billing_address` (e.g. `"François Dupont\n12 rue de la Paix\n75001 Paris"`). Django's `generate_pdf` view renders `billing_address` verbatim in the address box, so the name appears in the invoice despite always using the generic user account
- PDF proxy for guest bills uses a shared Django superuser session (`DJANGO_SUPERUSER_USERNAME` / `DJANGO_SUPERUSER_PASSWORD`), acquired at server startup and cached in `AppState`. On 403, the session is re-acquired once and retried

**`portal_guest_bill` table** — see schema in the Edges section above.

**Environment variables:**

| Variable | Default | Description |
|----------|---------|-------------|
| `GUEST_USER_ID` | `1` | `auth_user.id` of the generic guest account |
| `DJANGO_SUPERUSER_USERNAME` | _(empty)_ | Django superuser for guest PDF proxy |
| `DJANGO_SUPERUSER_PASSWORD` | _(empty)_ | Django superuser for guest PDF proxy |

### Scheduled voucher sync
A background task runs on a configurable cron schedule and refreshes the status of all locally-stored `Valid` vouchers against Unify.

**Purpose:** sessions can expire or vouchers can be used without any user triggering a manual check. This task ensures the local `portal_voucher.status` column stays consistent with the Unify source of truth even without user interaction.

**Algorithm:**
1. Load all `portal_voucher` rows with `status = 'Valid'`
2. Group them by `unify_create_time` (one Unify API call per batch)
3. For each batch: call `GET /api/s/{site}/stat/voucher` with the batch's `create_time`
4. Update each voucher's local `status` from the Unify response
5. Vouchers absent from the Unify response are marked `Expired` (revoked upstream)

**Scheduling:** configured via cron expression; default runs Monday–Friday, every hour from 09:00 to 19:00 Europe/Paris time (`0 0 9-19 * * 1-5`, 6-field format with seconds). The expression is validated at startup — an invalid expression prevents the server from starting.

**Environment variable:**

| Variable | Default | Description |
|----------|---------|-------------|
| `VOUCHER_SYNC_CRON` | `0 0 9-19 * * 1-5` | 6-field cron expression (sec min hour dom month dow) for the voucher sync task, evaluated in Europe/Paris timezone |

### Monthly voucher usage diary
A daily background task tracks which days each monthly voucher had at least one device actively connected via Unify.

**Purpose:** provide a forward-looking record of how many days a monthly voucher was actually used. The count of active days is `cardinality(active_days)` on the `portal_voucher` row — no separate aggregation needed. Non-monthly vouchers carry an empty array by default, leaving the door open to extend tracking to other types later.

**Constraint:** the diary is forward-only. Unify's guest API returns current or recent state only; days before the first task run cannot be reconstructed retroactively.

**Algorithm:**
1. Load all `portal_voucher.unify_id` values for vouchers linked to a `Monthly` service (via `billjobs_billline → portal_service WHERE kind = 'Monthly'`)
2. Call `POST /api/s/{site}/stat/guest` with `{ "within": 24 }` to get all guest devices seen in the last 24 hours
3. Filter to guests whose `voucher_id` matches one of our monthly voucher IDs
4. Group by `voucher_id`, collecting distinct MAC addresses (phone + laptop count as one voucher used)
5. For each active voucher: append today's date (Europe/Paris) to `active_days` — the update is idempotent (skips the append if the date is already present)

**Scheduling:** runs every hour from 09:00 to 19:00 Europe/Paris time by default (`0 0 9-19 * * *`). Each run queries only connections since today's midnight (not the previous 24 hours), so running multiple times per day is safe and idempotent.

**Environment variable:**

| Variable | Default | Description |
|----------|---------|-------------|
| `MONTHLY_USAGE_CRON` | `0 0 9-19 * * *` | 6-field cron expression for the monthly usage diary task, evaluated in Europe/Paris timezone |

**Guest Summary Page (`/buy/summary/:guestToken`):**
- Bill number, date, service name, amount
- Voucher cards with status (seeded from creation response)
- `↻` voucher status refresh button
- `⎙` invoice PDF download (via superuser session proxy)
- `⎙` voucher PDF download (visible only when at least one voucher is Valid)
### Member Invitations
Authenticated members can invite new users by email. The invited person receives a time-limited link to create their own account without requiring admin intervention.

**Flow:**
1. Authenticated user submits an email via `/invite`
2. Backend checks the email is not already registered
3. A 64-char alphanumeric token is generated (with up to 5 retries on UNIQUE collision) and stored in `portal_invitation_tokens`; any existing unused invitations for the same email are deleted first
4. An email is sent to the invitee with a link to `/accept-invite?token=…`, valid for 48 hours
5. Invitee opens the link, fills in username, first name, last name, and password
6. Backend validates the token (not expired, not used), checks username availability, creates `auth_user` and an empty `billjobs_userprofile`, then invalidates all pending invitations for that email
7. Invitee is redirected to `/login`

**`portal_invitation_tokens` table:**

| Column | Type | Notes |
|--------|------|-------|
| `id` | SERIAL PK | — |
| `email` | VARCHAR(254) | invited email address |
| `invited_by` | INTEGER FK → `auth_user.id` | audit trail |
| `token` | VARCHAR(64) UNIQUE | 64-char alphanumeric, index on `(token)` and `(email)` |
| `expires_at` | TIMESTAMPTZ | `NOW() + 48 hours` |
| `used_at` | TIMESTAMPTZ nullable | set when the invitation is accepted |

**Business rules:**
- The invited email must not belong to an existing active `auth_user`
- Accepting any invitation invalidates all other pending invitations for the same email
- New users start with an empty `billjobs_userprofile.billing_address`; they can update it from their profile page
- Password must meet the same requirements as the change-password form (8+ chars, at least one digit, one lowercase, one uppercase)
