# Agents and integration

## OpenClaw

**Next steps (handoff):** If Speedo is running but newsletter generation is not set up, see README section **OpenClaw (newsletter generation)** and the env table. In short: install OpenClaw (on the host or via Docker; see [docs.clawd.bot/install](https://docs.clawd.bot/install)), enable `gateway.http.endpoints.responses` and token auth, set `OPENCLAW_GATEWAY_URL`, `OPENCLAW_GATEWAY_TOKEN`, and `OPENCLAW_AGENT_ID` in `.env`, start the gateway, restart the backend.

This project uses an **OpenClaw Gateway** to generate newsletter content. The backend calls the OpenResponses-compatible HTTP endpoint (`POST /v1/responses`) with a prompt built from the user’s newsletter config (title, topics, tone, length, and enabled features with their custom instructions).

- **Agent**: Configure OpenClaw with an agent (e.g. `main`) that can act as a **daily newsletter writer**: it should produce a single newsletter document, include only the sections requested by the user, and follow per-section instructions (e.g. “Competitor analysis”, “Market segment summary”, “Identify risks” and any custom request text).
- **Config**: Enable the endpoint in OpenClaw (`gateway.http.endpoints.responses.enabled=true`) and set auth (e.g. `gateway.auth.mode="token"`). The backend sends `Authorization: Bearer OPENCLAW_GATEWAY_TOKEN` and `x-openclaw-agent-id: OPENCLAW_AGENT_ID` (from env).
- **Prompt**: The backend builds a user message from the newsletter config and optional system-style instructions so the agent outputs plain text or markdown suitable for email.

## Supabase auth and backend

- **Frontend**: Users sign in with Supabase Auth (email/password). The frontend sends the Supabase `access_token` in the `Authorization: Bearer <token>` header on every request to the backend.
- **Backend**: Uses the **Supabase REST API** (PostgREST) for data; no direct Postgres connection. Set `SUPABASE_URL` and `SUPABASE_SERVICE_ROLE_KEY` in env. A Rocket **request guard** (`User`) implements `FromRequest`: it reads the `Authorization` header, verifies the JWT using `SUPABASE_JWT_SECRET` (legacy) or JWKS from `SUPABASE_URL`, and injects a `UserContext` (user_id, email) into protected routes. Invalid or missing tokens result in 401. Newsletter CRUD and **send-sample** (`POST /api/me/newsletters/:id/send-sample`) routes use the **ApprovedUser** guard, which also requires the user to be in the `approved_users` table (otherwise 403). Unapproved users can still call `GET /api/me/approval-status` to see `{ approved: false }`; the frontend shows a “Thanks for registering… only manually approved users get access. Standby!” message until the admin inserts their `user_id` into `approved_users`.
