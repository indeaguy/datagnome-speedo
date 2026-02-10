# Speedo – Daily Newsletter

Multi-user daily newsletter app: users sign in (Supabase), configure a newsletter via a web form, and receive a generated digest by email. The backend (Rocket) stores configs in Supabase Postgres, runs an in-process scheduler, calls an OpenClaw Gateway to generate content, and sends email via SMTP.

## Stack

- **Frontend**: React (Vite + TypeScript), Supabase Auth, React Router.
- **Backend**: Rust (Rocket 0.5), Supabase REST API (HTTPS only), JWT auth, reqwest (OpenClaw), lettre (SMTP).
- **Data**: Supabase (Auth + Postgres). Tables: `newsletter_config`, `newsletter_run_log`; backend talks to them via the auto-generated REST API (no direct DB connection).
- **Deploy**: Docker (backend + frontend) and optional nginx + HTTPS on a VPS.

We do **not** assume you already have OpenClaw. You need to install and run an OpenClaw Gateway separately so the backend can call it to generate newsletter content. Until then, you can run Speedo for auth and CRUD; the scheduler will log errors when it tries to generate (or you can leave OpenClaw env vars unset and only test the API + UI).

## Local development

1. **Backend**
   - Copy `.env.example` to `.env` in the **project root** and set at least `SUPABASE_URL` (e.g. `https://PROJECT_REF.supabase.co`), `SUPABASE_SERVICE_ROLE_KEY`, and `SUPABASE_JWT_AUDIENCE`. Optionally set `SUPABASE_JWT_SECRET` (legacy) or leave unset to use JWKS from `SUPABASE_URL`. Optionally set OpenClaw and SMTP for the scheduler. For local dev set `VITE_API_BASE_URL=http://localhost:8080`.
   - From repo root: `cargo run --manifest-path backend/Cargo.toml` (so `.env` in the root is loaded). API listens on `http://127.0.0.1:8080`.

2. **Frontend**
   - Same `.env` in project root: set `VITE_SUPABASE_URL`, `VITE_SUPABASE_ANON_KEY`, `VITE_API_BASE_URL` (e.g. `http://localhost:8080` for local). Vite proxies `/api` to the backend when using `npm run dev`.
   - From repo root: `cd frontend && npm install && npm run dev`. Open the URL shown (e.g. http://localhost:5173).

3. **Database**
   - Apply the migration in `backend/migrations/` to your Supabase project (e.g. via Supabase Dashboard SQL or MCP). The plan used a migration named `create_newsletter_config_and_run_log`.

## Setting up OpenClaw

Speedo calls an **OpenClaw Gateway** over HTTP to generate newsletter text. You install and run OpenClaw separately, then point Speedo at it. Below is a detailed setup for **local** (your machine) and for **production on a VPS** (e.g. Websavers), including the exact config OpenClaw needs for Speedo.

**What OpenClaw must have for Speedo**

- The **HTTP responses** endpoint enabled: `POST /v1/responses` (OpenResponses API). It is **off by default**.
- **Gateway auth** in token mode so Speedo can call it with a bearer token.
- An **agent** (e.g. `main`) that can act as a newsletter writer (and model auth so it can call an LLM: Anthropic API key, etc.).

Config is JSON5 in `~/.openclaw/openclaw.json` (or `$OPENCLAW_CONFIG_PATH`). You will add something like:

```json
{
  "gateway": {
    "http": {
      "endpoints": {
        "responses": { "enabled": true }
      }
    },
    "auth": {
      "mode": "token",
      "token": "YOUR_CHOSEN_TOKEN"
    }
  },
  "agents": {
    "defaults": { "workspace": "~/.openclaw/workspace" },
    "list": [{ "id": "main" }]
  }
}
```

Use the same `YOUR_CHOSEN_TOKEN` as `OPENCLAW_GATEWAY_TOKEN` in Speedo’s `.env`. Reference: [Gateway configuration](https://docs.clawd.bot/gateway/configuration), [OpenResponses API](https://docs.clawd.bot/gateway/openresponses-http-api), [Authentication](https://docs.clawd.bot/gateway/authentication).

---

### OpenClaw: local (your Mac/Linux/Windows)

Use this when running Speedo on your laptop and you want newsletter generation to work locally.

1. **Prereqs**  
   Node 22+. Check with `node --version`. Install Node from [nodejs.org](https://nodejs.org) or your package manager if needed.

2. **Install OpenClaw**  
   - **macOS/Linux**: `curl -fsSL https://openclaw.ai/install.sh | bash`  
   - **Windows (PowerShell)**: `iwr -useb https://openclaw.ai/install.ps1 | iex`  
   Other options (Docker, Nix, etc.): [docs.clawd.bot/install](https://docs.clawd.bot/install).

3. **Onboarding**  
   Run:
   ```bash
   openclaw onboard --install-daemon
   ```
   The wizard will set up auth (e.g. Anthropic API key or OAuth), gateway settings, and optionally channels. You can accept defaults for most things; you need at least one **model provider** (e.g. Anthropic) so the agent can generate text.  
   If you prefer not to install a daemon, you can skip `--install-daemon` and start the gateway manually in a terminal (step 5).

4. **Config for Speedo**  
   Create or edit the OpenClaw config file. Default path: `~/.openclaw/openclaw.json`.  
   - If the file doesn’t exist, create it with the JSON block from “What OpenClaw must have for Speedo” above (enable `gateway.http.endpoints.responses`, set `gateway.auth.mode` and `gateway.auth.token`).  
   - If it already exists (e.g. from onboarding), add or merge in:
     - `gateway.http.endpoints.responses.enabled: true`
     - `gateway.auth.mode: "token"`
     - `gateway.auth.token: "<pick a long random string>"`  
   Save the token somewhere safe; you’ll put it in Speedo’s `.env` as `OPENCLAW_GATEWAY_TOKEN`.

5. **Start the gateway**  
   - If you used `--install-daemon`: the gateway may already be running. Check with `openclaw gateway status` (or `openclaw status`).  
   - If not, run in a terminal: `openclaw gateway` (or `openclaw gateway --port 18789`). Leave this running. Default port is **18789**.

6. **Verify**  
   - `openclaw doctor` — no config errors.  
   - `openclaw dashboard` — Control UI opens; you can chat in the browser to confirm the agent works.

7. **Point Speedo at OpenClaw**  
   In Speedo’s `.env` (in the Speedo repo root):
   ```bash
   OPENCLAW_GATEWAY_URL=http://127.0.0.1:18789/v1/responses
   OPENCLAW_GATEWAY_TOKEN=<same token as gateway.auth.token>
   OPENCLAW_AGENT_ID=main
   ```
   Restart the Speedo backend so it picks up the new env. The scheduler will then be able to call OpenClaw when a newsletter is due.

**Summary (local):** Install OpenClaw → onboard (model auth) → add `responses.enabled` and `auth.token` to config → start gateway → set the three `OPENCLAW_*` vars in Speedo’s `.env`.

---

### OpenClaw: production (Websavers VPS or same server as Speedo)

You have two choices: run OpenClaw **on the same VPS** as Speedo (simplest), or on **another machine**. Same-VPS is recommended so you don’t manage two servers or firewall rules.

**Option A: OpenClaw on the same VPS as Speedo**

1. **Prepare the VPS**  
   Same as for Speedo: Ubuntu 22.04 (or similar), Docker + Compose if you use Docker for Speedo. OpenClaw can run **on the host** (not in Docker) so the Speedo backend container can reach it via the host’s IP or `host.docker.internal`.

2. **Install OpenClaw on the VPS**  
   SSH into the VPS. Install Node 22+ if needed, then:
   ```bash
   curl -fsSL https://openclaw.ai/install.sh | bash
   ```
   Run onboarding **without** a desktop (headless):
   ```bash
   openclaw onboard
   ```
   When prompted for model auth, use an API key (e.g. Anthropic). You will not have a browser for OAuth on the server; API key is the right choice.

3. **Config file on the VPS**  
   On the VPS, edit `~/.openclaw/openclaw.json` (or the path given by `OPENCLAW_CONFIG_PATH`). Ensure it contains:
   - `gateway.http.endpoints.responses.enabled: true`
   - `gateway.auth.mode: "token"`
   - `gateway.auth.token: "<strong random token>"`
   - `agents` with at least one agent (e.g. `id: "main"`) and `agents.defaults.workspace` set.  
   You can use env substitution in config, e.g. `"token": "${OPENCLAW_GATEWAY_TOKEN}"`, and set `OPENCLAW_GATEWAY_TOKEN` in `~/.openclaw/.env` so the token isn’t in the repo.

4. **Run the gateway and keep it running**  
   - **Foreground (for testing):** `openclaw gateway`  
   - **Production:** run as a service. Example systemd unit `/etc/systemd/system/openclaw-gateway.service`:
     ```ini
     [Unit]
     Description=OpenClaw Gateway
     After=network.target

     [Service]
     Type=simple
     User=YOUR_VPS_USER
     WorkingDirectory=/home/YOUR_VPS_USER
     Environment="OPENCLAW_CONFIG_PATH=/home/YOUR_VPS_USER/.openclaw/openclaw.json"
     ExecStart=/usr/bin/openclaw gateway
     Restart=on-failure
     RestartSec=5

     [Install]
     WantedBy=multi-user.target
     ```
     Then: `sudo systemctl daemon-reload`, `sudo systemctl enable openclaw-gateway`, `sudo systemctl start openclaw-gateway`.  
   The gateway listens on **18789** on the host. Ensure nothing else uses that port, or set `gateway.port` in config.

5. **Speedo backend reaching OpenClaw**  
   Speedo runs in Docker on the same host. The backend container must call the host’s gateway:
   - **Linux:** use the host’s primary IP (e.g. `172.17.0.1` for Docker’s default bridge) or `host.docker.internal` if your Docker version supports it.
   - **host.docker.internal:** set in Speedo’s `.env`: `OPENCLAW_GATEWAY_URL=http://host.docker.internal:18789/v1/responses`. (If that hostname isn’t available, use the host IP.)
   - Use the **same** token in `OPENCLAW_GATEWAY_TOKEN` as in OpenClaw’s `gateway.auth.token`.

6. **Firewall**  
   You do **not** need to expose 18789 to the internet. Only the Speedo backend (on the same host) needs to reach it. If Speedo is in Docker, Docker can reach the host’s localhost/IP; no extra firewall rule for 18789 is required unless you want to access the Control UI from outside (then open 18789 and consider auth and TLS).

**Option B: OpenClaw on a different machine**

1. Install and configure OpenClaw on that machine (same steps as Option A: install, onboard, config with `responses.enabled` and `auth.token`).
2. On that machine, allow inbound TCP **18789** from the Speedo VPS IP only (firewall/security group).
3. In Speedo’s `.env` on the VPS, set:
   ```bash
   OPENCLAW_GATEWAY_URL=http://<openclaw-host-ip-or-hostname>:18789/v1/responses
   OPENCLAW_GATEWAY_TOKEN=<same token>
   OPENCLAW_AGENT_ID=main
   ```
4. Restart the Speedo backend and test (e.g. trigger a newsletter run or wait for the scheduler).

**Docker-only option (OpenClaw in Docker on the VPS)**

If you prefer everything in Docker: OpenClaw provides a [Docker install path](https://docs.clawd.bot/install/docker). You would run OpenClaw’s stack (e.g. `docker-setup.sh` from the OpenClaw repo) on the VPS so the OpenClaw gateway container is on the same Docker network as Speedo’s backend. Then set `OPENCLAW_GATEWAY_URL=http://openclaw-gateway:18789/v1/responses` (using the OpenClaw gateway service name and port from their compose file). This requires cloning or using OpenClaw’s Docker setup on the VPS; the exact service name and port are in their docs.

---

**Testing without OpenClaw**

You can run Speedo (auth, config CRUD, UI) with OpenClaw env vars **unset or empty**. The scheduler will then return a clear error when it tries to generate (“OpenClaw not configured”). Once OpenClaw is installed and the three `OPENCLAW_*` vars are set, newsletter generation will work.

## Environment variables

| Variable | Where | Purpose |
|----------|--------|---------|
| `SUPABASE_URL` | Backend | Supabase project URL (e.g. `https://PROJECT_REF.supabase.co`). Used for REST API and JWKS. |
| `SUPABASE_SERVICE_ROLE_KEY` | Backend | Service role key (Project Settings → API). Backend uses it for Supabase REST API; keep secret. |
| `SUPABASE_JWT_SECRET` | Backend | Optional. Legacy JWT secret. If unset, backend uses JWKS from `SUPABASE_URL` for JWT verification. |
| `SUPABASE_JWT_AUDIENCE` | Backend | Usually `authenticated`. |
| `OPENCLAW_GATEWAY_URL` | Backend | OpenClaw Gateway URL, e.g. `http://host:18789/v1/responses`. |
| `OPENCLAW_GATEWAY_TOKEN` | Backend | Token for `gateway.auth.token`. |
| `OPENCLAW_AGENT_ID` | Backend | Agent id, e.g. `main`. |
| `SMTP_HOST`, `SMTP_PORT`, `SMTP_USER`, `SMTP_PASS`, `SMTP_FROM` | Backend | SMTP for sending newsletters. |
| `CORS_ORIGINS` | Backend | When frontend is on a different origin (e.g. API at api.speedo.email), set to the frontend origin, e.g. `https://speedo.email`. Comma-separated for multiple. Default `*`. |
| `VITE_SUPABASE_URL` | Frontend build | Supabase project URL. |
| `VITE_SUPABASE_ANON_KEY` | Frontend build | Supabase anon key. |
| `VITE_API_BASE_URL` | Frontend build | Origin for API calls (e.g. `https://yourdomain.com`). No trailing slash. |

## Deployment (Websavers VPS)

### Test setup: backend on VPS, frontend local

Use this to test with the backend at `http://142.44.145.173:8080` and the frontend running on your machine.

**On the VPS (142.44.145.173, Ubuntu):**

1. **Dependencies and app**
   - `sudo apt update && sudo apt install -y build-essential pkg-config libssl-dev` (for Rust/OpenSSL).
   - Install Rust: `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y` then `source $HOME/.cargo/env`.
   - Clone the repo (e.g. `git clone <repo> /opt/speedo && cd /opt/speedo`).

2. **Backend .env**
   - In `/opt/speedo` create `.env` with at least: `SUPABASE_URL`, `SUPABASE_SERVICE_ROLE_KEY`, `SUPABASE_JWT_AUDIENCE`, and **`CORS_ORIGINS=http://localhost:5173,http://127.0.0.1:5173`** so the local frontend can call the API. Set SMTP/OpenClaw if you need them.

3. **Build and run**
   - `cd /opt/speedo && cargo build --release --manifest-path backend/Cargo.toml`
   - From repo root so `.env` is loaded, run the backend listening on all interfaces: `cd /opt/speedo && ROCKET_ADDRESS=0.0.0.0 ROCKET_PORT=8080 ./backend/target/release/speedo-backend`. (Rocket defaults to 127.0.0.1 otherwise and would not be reachable from your machine.)

4. **Firewall**
   - Allow port 8080: `sudo ufw allow 8080 && sudo ufw status` (enable ufw if needed).

5. **Keep it running**
   - Run in a `screen` or `tmux` session, or add a systemd unit so it survives logout.

**Locally:**

1. In the repo `.env` set **`VITE_API_BASE_URL=http://142.44.145.173:8080`** (and your `VITE_SUPABASE_URL`, `VITE_SUPABASE_ANON_KEY`).
2. **Supabase:** In Authentication → URL Configuration, add **`http://localhost:5173`** (and `http://127.0.0.1:5173` if you use that) to Redirect URLs.
3. Start the frontend: `cd frontend && npm run dev`. Open http://localhost:5173 and sign in; the app will call the API on the VPS.

**Check:** `curl -s http://142.44.145.173:8080/api/health` should return `{"status":"ok"}` or `{"status":"db_error"}`.

### Option A: Split deployment (frontend on web host, backend on subdomain)

Use this when you serve the frontend as static files (e.g. upload via SFTP/FTP to the web server) and run the backend on a subdomain (e.g. `api.speedo.email`). Ubuntu 24.04 is fine.

1. **DNS**
   - `speedo.email` → A record to your VPS IP (for the static frontend).
   - `api.speedo.email` → A record to the same VPS IP (for the API).

2. **Backend on the VPS**
   - Install Rust (or build the backend binary elsewhere and copy it). Clone the repo, create `.env` with `SUPABASE_URL`, `SUPABASE_SERVICE_ROLE_KEY`, `SUPABASE_JWT_AUDIENCE`, SMTP, OpenClaw vars, and **`CORS_ORIGINS=https://speedo.email`** (so the frontend origin can call the API). Run the backend (e.g. `cargo run --release` or run the built binary) listening on a port (e.g. 8080). Run it under systemd so it survives reboots.

3. **nginx**
   - **api.speedo.email**: server block with `server_name api.speedo.email`; `location /` → `proxy_pass http://127.0.0.1:8080;` (plus `proxy_set_header Host`, `X-Real-IP`, `X-Forwarded-For`, `X-Forwarded-Proto`).
   - **speedo.email**: server block with `server_name speedo.email www.speedo.email`; `root /var/www/speedo` (or wherever you upload the frontend); `location /` → `try_files $uri $uri/ /index.html;` for SPA routing.
   - Run `certbot --nginx -d speedo.email -d api.speedo.email` for HTTPS.

4. **Frontend build and upload**
   - Build with `VITE_API_BASE_URL=https://api.speedo.email`, `VITE_SUPABASE_URL`, `VITE_SUPABASE_ANON_KEY`. Upload the contents of `frontend/dist` to the web root (e.g. `/var/www/speedo`) via SFTP (prefer over plain FTP).

5. **Supabase**
   - In Supabase Auth redirect URLs, add `https://speedo.email` (and `https://speedo.email/**` if required).

### Option B: Single-domain Docker

1. **Prerequisites**
   - Ubuntu 22.04 or 24.04. Docker Engine + Docker Compose (Compose V2). Ports 80/443 open (e.g. `ufw allow 80 && ufw allow 443 && ufw enable`). Domain A record pointing to the VPS.

2. **App and env**
   - Clone the repo (e.g. to `/opt/speedo`). Create `.env` in the project root with all variables above. Backend reads env at runtime; frontend needs `VITE_*` at **build** time (passed as build args in `docker-compose.yml`).

3. **OpenClaw**
   - Follow **OpenClaw: production** above (same VPS or different machine). Ensure the gateway has the HTTP responses endpoint enabled and token auth, and set `OPENCLAW_GATEWAY_URL`, `OPENCLAW_GATEWAY_TOKEN`, and `OPENCLAW_AGENT_ID` in `.env` so the backend container can reach it.

4. **Build and run**
   - From project root: `docker compose up -d --build`. Backend exposes 8080, frontend 3000 (or 80 in the Dockerfile; compose maps 3000:80). Check with `docker compose ps` and `docker compose logs -f backend`.

5. **Health**
   - `curl -s http://localhost:8080/api/health` should return `{"status":"ok"}` (or `db_error` if DB is down).

6. **Reverse proxy and HTTPS**
   - Install nginx. Add a server block: `server_name yourdomain.com`; `location /` → `proxy_pass http://127.0.0.1:3000;`; `location /api/` → `proxy_pass http://127.0.0.1:8080/api/;` (with `proxy_set_header Host`, `X-Real-IP`, `X-Forwarded-For`, `X-Forwarded-Proto`). Enable the site, run `certbot --nginx -d yourdomain.com`. Set `VITE_API_BASE_URL=https://yourdomain.com`, rebuild frontend (`docker compose up -d --build frontend`), and add the same URL to Supabase redirect URLs.

7. **Start on boot**
   - systemd: create a unit with `WorkingDirectory=/opt/speedo`, `ExecStart=/usr/bin/docker compose up -d`, `ExecStop=/usr/bin/docker compose down`, then `systemctl enable speedo`. Or cron: `@reboot cd /opt/speedo && docker compose up -d`.

8. **Troubleshooting**
   - 502: check container ports and nginx `proxy_pass`.
   - 401 on API: check `SUPABASE_JWT_SECRET` and `SUPABASE_JWT_AUDIENCE`; ensure frontend sends the Supabase access token.
   - Newsletter not sending: backend logs; verify OpenClaw URL and SMTP credentials.
   - DB/API errors: check `SUPABASE_URL` and `SUPABASE_SERVICE_ROLE_KEY`; ensure the backend can reach Supabase over HTTPS.
   - Wrong API in browser: rebuild frontend after changing `VITE_API_BASE_URL`.

9. **Checklist**
   - Docker + Compose installed; 80/443 open; domain A record set; `.env` complete; OpenClaw reachable; `docker compose up -d --build` OK; health check OK; nginx + certbot; `VITE_API_BASE_URL` and Supabase redirects set; start-on-boot configured; one full sign-up → create config → run (or wait for scheduler) test.
