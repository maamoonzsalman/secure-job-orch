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
