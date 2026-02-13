# Speedo – Daily Newsletter

Multi-user daily newsletter app: users sign in (Supabase), configure a newsletter via a web form, and receive a generated digest by email. Approved users can send a one-off sample from the edit page. Backend (Rocket) stores configs in Supabase Postgres, runs an in-process scheduler, calls an OpenClaw Gateway to generate content, and sends email via SMTP.

## Stack

- **Frontend**: React (Vite + TypeScript), Supabase Auth, React Router.
- **Backend**: Rust (Rocket 0.5), Supabase REST API (HTTPS only), JWT auth, reqwest (OpenClaw), lettre (SMTP).
- **Data**: Supabase (Auth + Postgres). Tables: `newsletter_config`, `newsletter_run_log`, `approved_users`; backend uses REST API only (no direct DB).
- **Deploy**: Docker or backend binary + reverse proxy (Apache/nginx) + static frontend.

OpenClaw is optional. Without it you can run auth and CRUD; the scheduler will log errors when generation is due. See **OpenClaw** below to enable newsletter generation.

## How it works

```
  User  <───►  Frontend (React + Supabase Auth)
                    │
                    │ JWT
                    ▼
               Backend (Rocket API + Scheduler)
                    │
         ┌──────────┼──────────┐
         ▼          ▼          ▼
    Supabase    OpenClaw     SMTP ──► User
    (Auth +     (optional)
    Postgres)
```

- **Sign-in & config**: User signs in with Supabase Auth in the frontend; the frontend calls the backend API with the JWT. The backend reads/writes `newsletter_config` and `approved_users` via Supabase REST.
- **Scheduled run**: The in-process scheduler (in the backend) runs on a schedule; for each due newsletter it calls OpenClaw to generate the body, then sends the email via SMTP.
- **Send sample**: From the edit page, an approved user triggers a one-off send; same flow (backend → OpenClaw → SMTP) to the configured delivery email.

## Local development

1. **Env** – Copy `.env.example` to `.env` in the project root. Set at least `SUPABASE_URL`, `SUPABASE_SERVICE_ROLE_KEY`, `SUPABASE_JWT_AUDIENCE`; for local frontend set `VITE_API_BASE_URL=http://localhost:8080`, `VITE_SUPABASE_URL`, `VITE_SUPABASE_ANON_KEY`.

2. **Backend** – From repo root: `cargo run --manifest-path backend/Cargo.toml`. API at `http://127.0.0.1:8080`.

3. **Frontend** – `cd frontend && npm install && npm run dev`. Open the URL shown (e.g. http://localhost:5173).

4. **Database** – Apply migrations in `backend/migrations/` to your Supabase project (Dashboard SQL or MCP): `create_newsletter_config_and_run_log`, then `add_approved_users`.

5. **Auth** – Optional: Supabase Dashboard → Authentication → Providers → Email → turn off **Confirm email** so sign-in works without confirmation. New users are gated until approved: add their auth user UUID to the `approved_users` table (Table Editor → `approved_users` → Insert row).

## OpenClaw (newsletter generation)

To enable the scheduler to generate and send newsletters:

1. Install and run an OpenClaw Gateway (see [docs.clawd.bot/install](https://docs.clawd.bot/install)).
2. In OpenClaw config (`~/.openclaw/openclaw.json`): set `gateway.http.endpoints.responses.enabled: true`, `gateway.auth.mode: "token"`, `gateway.auth.token: "<secret>"`, and at least one agent (e.g. `main`) with model auth.
3. In Speedo `.env`: `OPENCLAW_GATEWAY_URL` (e.g. `http://127.0.0.1:18789/v1/responses`), `OPENCLAW_GATEWAY_TOKEN` (same as OpenClaw’s token), `OPENCLAW_AGENT_ID` (e.g. `main`).
4. Start the gateway and restart the Speedo backend.

See **AGENTS.md** for how the backend uses OpenClaw (headers, prompt). Without the three `OPENCLAW_*` vars set, generation is skipped and the scheduler logs a clear error.

## Environment variables

| Variable | Where | Purpose |
|----------|--------|---------|
| `SUPABASE_URL` | Backend | Supabase project URL. REST API + JWKS. |
| `SUPABASE_SERVICE_ROLE_KEY` | Backend | Service role key (Project Settings → API). Keep secret. |
| `SUPABASE_JWT_SECRET` | Backend | Optional. If unset, backend uses JWKS from `SUPABASE_URL`. |
| `SUPABASE_JWT_AUDIENCE` | Backend | Usually `authenticated`. |
| `OPENCLAW_GATEWAY_URL` | Backend | e.g. `http://host:18789/v1/responses`. |
| `OPENCLAW_GATEWAY_TOKEN` | Backend | Same as OpenClaw `gateway.auth.token`. |
| `OPENCLAW_AGENT_ID` | Backend | e.g. `main`. |
| `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASS`, `SMTP_FROM` | Backend | SMTP for sending. |
| `SMTP_TLS_SKIP_VERIFY` | Backend | Optional. Set to skip TLS hostname verification. |
| `CORS_ORIGINS` | Backend | Frontend origin(s) when different from API. Default `*`. |
| `VITE_SUPABASE_URL`, `VITE_SUPABASE_ANON_KEY`, `VITE_API_BASE_URL` | Frontend build | Supabase and API URL for the client. |

## Deployment

- **Docker**: Clone repo, create `.env`, then `docker compose up -d --build`. Put nginx (or similar) in front: `/` → frontend, `/api/` → backend (e.g. 8080). Set `VITE_API_BASE_URL` to the public URL and add it to Supabase redirect URLs.
- **Bare metal**: Build backend (`cargo build --release --manifest-path backend/Cargo.toml`), run with `ROCKET_ADDRESS=0.0.0.0 ROCKET_PORT=8080`. Build frontend with production `VITE_*` vars, serve static files from nginx/Apache; proxy `/api/` to the backend. Use `CORS_ORIGINS` for the frontend origin and HTTPS + certbot as needed.
