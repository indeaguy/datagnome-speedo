-- Newsletter config: one per user-defined daily newsletter
-- Applied to Supabase (Speedo project). Run via Supabase Dashboard or MCP.
create table if not exists public.newsletter_config (
  id uuid primary key default gen_random_uuid(),
  user_id uuid not null references auth.users(id) on delete cascade,
  title text not null default '',
  topics text[] not null default '{}',
  tone text not null default 'neutral',
  length text not null default 'medium' check (length in ('short', 'medium', 'long')),
  send_time_utc time not null default '09:00',
  timezone text not null default 'UTC',
  delivery_email text not null,
  is_active boolean not null default true,
  features jsonb not null default '{}',
  created_at timestamptz not null default now(),
  updated_at timestamptz not null default now()
);

create index if not exists newsletter_config_user_id on public.newsletter_config(user_id);
create index if not exists newsletter_config_is_active_send_time on public.newsletter_config(is_active, send_time_utc) where is_active = true;

create table if not exists public.newsletter_run_log (
  id uuid primary key default gen_random_uuid(),
  newsletter_config_id uuid not null references public.newsletter_config(id) on delete cascade,
  run_at timestamptz not null default now(),
  status text not null check (status in ('success', 'failure')),
  error_message text,
  openclaw_response_id text,
  created_at timestamptz not null default now()
);

create index if not exists newsletter_run_log_config_id on public.newsletter_run_log(newsletter_config_id);
create index if not exists newsletter_run_log_run_at on public.newsletter_run_log(run_at);

alter table public.newsletter_config enable row level security;
alter table public.newsletter_run_log enable row level security;

-- RLS policies: see Supabase migration for full policy definitions.
-- newsletter_config: select/insert/update/delete where auth.uid() = user_id
-- newsletter_run_log: select/insert via config ownership
