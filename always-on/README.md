# Houston Always On

Self-host the Houston Engine on your own server. Your agents keep running
while your laptop is closed. Your desktop and mobile clients connect over
the same HTTP + WebSocket protocol they use locally — the only thing that
changes is the `baseUrl`.

## Prerequisites

- Linux host, 1 vCPU / 512 MB RAM minimum.
- Docker 24+ (or `cargo` if you prefer a native build).
- A long random bearer token (`openssl rand -hex 32`).

## Quick start (Docker)

```bash
cd always-on/
cp .env.example .env
$EDITOR .env   # fill in HOUSTON_ENGINE_TOKEN
docker compose up -d
curl -H "Authorization: Bearer $TOKEN" http://localhost:7777/v1/health
```

You should see:

```json
{"status":"ok","version":"0.4.0","protocol":1}
```

## Deploy to a managed cloud (Railway)

Railway builds the same `always-on/Dockerfile` and runs it as a managed
container — no VM, no reverse proxy to operate, TLS + a public domain
included. This is the lean first rung for running the engine in the cloud and
pairing a phone to it. (For per-agent hard isolation across many tenants you
graduate to Firecracker microVMs — see `knowledge-base/cloud-compute.md`.)

1. **New project** → *Deploy from GitHub repo* → pick this repo.
2. **Point it at the config**: Service → Settings → *Config-as-code* path =
   `always-on/railway.json`. That selects the Dockerfile build + the
   `/healthz` healthcheck. Leave the root directory at the repo root — the
   Dockerfile needs the whole workspace as build context.
3. **Variables**:
   - `HOUSTON_ENGINE_TOKEN` = `openssl rand -hex 32` (clients send this).
   - `HOUSTON_HOME=/data/state/.houston` and `HOUSTON_DOCS=/data/state/workspaces`.
   - `PORT` is injected by Railway; the entrypoint binds it automatically.
4. **Volume**: add a Volume mounted at **`/data/state`** (NOT `/data` — that
   would shadow the provider CLIs baked in at `/data/.local`). This persists
   the DB, the tunnel identity (`tunnel.json` → stable `tunnelId`, so phones
   never re-pair), and workspaces across redeploys.
5. **Deploy**, then grab the public domain and verify:

```bash
curl https://<your>.up.railway.app/healthz                        # public liveness
curl -H "Authorization: Bearer $TOKEN" https://<your>.up.railway.app/v1/health
```

`railway up` from the repo root works too once the service exists.

### Same image, microVM host (Fly)

This exact image is what the `houston-sandbox` `fly` backend boots as
`FLY_IMAGE`: push it to Fly's registry (`fly deploy` / `docker push
registry.fly.io/<app>`) and the sandbox runner provisions one Firecracker
microVM per agent. Railway = single-tenant container today; Fly = the
isolation upgrade when multi-tenant earns it.

## Reverse proxy (recommended)

Terminate TLS at your proxy (`caddy`, `nginx`, `traefik`…) and forward
`/v1/*` and `/v1/ws` to `127.0.0.1:7777`. Example Caddyfile:

```
houston.example.com {
    reverse_proxy 127.0.0.1:7777
}
```

WebSocket upgrade headers are forwarded by default in modern proxies.

## Connect the desktop app

In Houston → Settings → Connect to remote engine, paste:

- URL: `https://houston.example.com`
- Token: `$HOUSTON_ENGINE_TOKEN`

Local OS-native features (reveal in file manager, file pickers) stay disabled
when you're connected to a remote engine.

## Connect the mobile app (PWA)

No extra wiring: the engine dials **outbound** to the Houston relay
(`tunnel.gethouston.ai`, set via `HOUSTON_TUNNEL_URL`, on by default) and
registers a reverse tunnel — exactly as a desktop engine does. The phone pairs
to it over that tunnel; the relay and PWA can't tell a cloud engine from a Mac
("same code, two doors"). The container just needs outbound network, which
Railway/Fly give by default.

1. After first boot completes (the engine needs network once to allocate its
   tunnel), mint the durable pairing code:

   ```bash
   curl -X POST -H "Authorization: Bearer $TOKEN" \
     https://<your-domain>/v1/tunnel/pairing
   # → { "code": "<tunnelId>-<secret>", ... }
   ```

   (`GET /v1/tunnel/status` reports whether tunnel allocation has completed.)
2. On the phone, open `https://tunnel.gethouston.ai/pair/<code>` (or scan its
   QR). Safari/Chrome loads the PWA, redeems the code over the tunnel, and the
   engine mints a device-scoped bearer.
3. The PWA now talks to the cloud engine via
   `https://tunnel.gethouston.ai/e/<tunnelId>/v1/...`. A cloud engine is
   always-on, so there are none of the Mac-asleep gaps the desktop has.

Provider sign-in (Claude / Codex / Composio) happens from a connected client:
the desktop or PWA triggers login and the engine runs the device-code flow
(`codex login --device-auth`, Claude's paste-back code) — no provider secrets
need to live in the deploy's env.

## Environment

| Var | Default | Purpose |
|---|---|---|
| `HOUSTON_BIND` | `127.0.0.1:0` | `ip:port` to bind. Set to `0.0.0.0:7777` for remote. |
| `HOUSTON_BIND_ALL` | unset | Must be `1` to allow binding `0.0.0.0`. |
| `HOUSTON_ENGINE_TOKEN` | auto-generated | Bearer token clients must send. |
| `HOUSTON_HOME` | `~/.houston` | Data dir (DB, `engine.json`, workspaces). |
| `HOUSTON_DOCS` | `$HOUSTON_HOME/workspaces` | Workspaces root. |
| `HOUSTON_NO_PARENT_WATCHDOG` | unset | Set to `1` to disable the stdin-EOF parent watchdog. Required for non-interactive standalone runs (systemd/docker) where stdin is `/dev/null` — already set in the unit and compose files here. |
| `PORT` | unset | Injected by PaaS platforms (Railway, Fly, Heroku). When set, the container entrypoint binds `0.0.0.0:$PORT` and overrides `HOUSTON_BIND`. |
| `RUST_LOG` | `info,houston=debug` | tracing filter. |

## Native build (no Docker)

```bash
cargo build --release -p houston-engine-server --bin houston-engine
HOUSTON_BIND=0.0.0.0:7777 HOUSTON_BIND_ALL=1 \
  HOUSTON_ENGINE_TOKEN=$TOKEN \
  ./target/release/houston-engine
```

A systemd unit template lives at `always-on/houston-engine.service`.

## Updating

The engine exposes its version at `GET /v1/version`. When we release a new
major-minor, the `X-Houston-Engine-Version` header bumps; desktop clients
refuse to talk to an engine with a higher protocol major. Pull, rebuild,
restart.

## Status

Ships with Phase 5 of the engine rollout. Phases 1-4 track the path from
"Tauri-only backend" to "standalone binary" — see
`knowledge-base/engine-server.md`.
