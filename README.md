# Secure Job Orchestrator

A small, correctness-focused system that demonstrates secure job orchestration with strict trust boundaries.

- **Rust** control plane manages job lifecycle, cryptographic metadata, and verification.
- **C++** compute module performs deterministic processing only.
- **Supabase (Postgres)** stores job state and metadata only.
- **Next.js + React + TypeScript + Tailwind** provide a thin UI for submitting and inspecting jobs.

## Architecture at a glance

```
client/   -> Next.js UI (submit, list, detail)
server/   -> Rust API + C++ compute module + schema
```

### Trust boundaries

- Rust owns crypto (Ed25519 signing, SHA-256 hashing) and lifecycle transitions.
- C++ output is always treated as untrusted until verified by Rust.
- Supabase is persistence only; no business logic or auth rules are used.

### Cryptographic assumptions

- **Ed25519** signatures authenticate backend-generated job metadata.
- **SHA-256** hashes ensure payload integrity.
- The compute module only emits a deterministic checksum (FNV-1a); Rust recomputes and validates it.

## Database schema (Supabase)

Single table, no joins, no auth/RLS rules:

```sql
create table if not exists jobs (
  id uuid primary key,
  job_type text not null,
  payload jsonb not null,
  state text not null,
  payload_hash text not null,
  backend_signature text not null,
  verification_status text not null,
  created_at timestamp with time zone not null default now(),
  updated_at timestamp with time zone not null default now()
);
```

The schema lives in `server/schema.sql`.

## API (Rust backend)

- `POST /jobs` submit a job for validation and deterministic processing
- `GET /jobs` list jobs
- `GET /jobs/{id}` job details

Each response includes job state, verification status, and signed metadata.

## Local development

### 1) Supabase

Use the existing Supabase project configured for this repo and run the schema:

- Open the Supabase SQL editor and run `server/schema.sql`.

### 2) Backend (Rust)

```bash
cd server
cp .env.example .env
# Fill in SUPABASE_URL, SUPABASE_SERVICE_ROLE_KEY, JOB_SIGNING_KEY

make -C compute
cargo run
```

`JOB_SIGNING_KEY` must be a 32-byte hex value. Example:

```bash
python - <<'PY'
import secrets
print(secrets.token_hex(32))
PY
```

### 3) Frontend (Next.js)

```bash
cd client
npm install
npm run dev
```

Set `NEXT_PUBLIC_API_BASE` in `client/.env.local` if the backend runs somewhere other than `http://localhost:8080`.

## What this project demonstrates

- Clear ownership of cryptographic authority in Rust
- Verifiable job metadata with Ed25519
- Deterministic compute module integration via stdin/stdout JSON
- Explicit job lifecycle state transitions
- Minimal persistence model in Supabase

## Out of scope

- Auth, RBAC, payments, queues, background workers
- Scaling, Kubernetes, gRPC, advanced analytics
- Custom cryptographic protocols or multi-language key splitting
