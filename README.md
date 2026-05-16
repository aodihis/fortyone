# Fortyone

A multiplayer 41-Rummy card game — Rust backend with WebSocket, Rust/WASM frontend using Yew.

```
fortyone/
├── be/   — Axum backend (REST + WebSocket, JWT auth)
└── fe/   — Yew frontend (compiled to WebAssembly)
```

## Quick Start

### Prerequisites

| Tool | Purpose |
|------|---------|
| Rust + Cargo | Both backend and frontend |
| `wasm32-unknown-unknown` target | Frontend WASM compilation |
| [Trunk](https://trunkrs.dev/) | Frontend build/dev server |

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk
```

### Configuration

**Backend** — copy and edit `be/.env.example`:
```bash
cp be/.env.example be/.env
# Set a strong JWT_SECRET
```

**Frontend** — copy and edit `fe/.env.example`:
```bash
cp fe/.env.example fe/.env
# Set API_URL to point at your backend
```

### Run both

```bash
make dev        # backend + frontend in parallel
make be         # backend only
make fe         # frontend only (http://localhost:8000)
```

### Other commands

```bash
make build      # compile both (release)
make test       # run backend test suite
make check      # quick compile check, no output artifacts
```

## How it works

1. A player calls `GET /create` → receives a `game_id`
2. Each player calls `POST /{game_id}/join?player_name=<name>` → receives a short-lived JWT token
3. Each player connects via WebSocket at `GET /{game_id}/ws?token=<jwt>`
4. The host sends `start_game`; turns proceed with `draw`, `take_bin`, `discard`, and `close` actions

See [be/README.md](be/README.md) for the full API reference.

## Deployment (Docker + Traefik)

Images are published to GitHub Container Registry (GHCR) automatically on every push to `main` via GitHub Actions.

### One-time GitHub setup

Add one secret to your repository (Settings → Secrets → Actions):

| Secret | Value |
|--------|-------|
| `API_URL` | `https://api.yourdomain.com` — your backend's public URL, baked into the frontend WASM at build time |

`GITHUB_TOKEN` is provided automatically by GitHub Actions — no setup needed.

### On your VPS (no repo clone needed)

```bash
# 1. Create app directory
mkdir -p /opt/apps/fortyone
cd /opt/apps/fortyone

# 2. Copy these two files from the repo:
#    docker-compose.prod.yml
#    .env.example

# 3. Create your .env
cp .env.example .env
# Edit .env and fill in BE_HOST, FE_HOST, JWT_SECRET, ALLOWED_ORIGIN

# 4. Pull images and start
docker compose -f docker-compose.prod.yml pull
docker compose -f docker-compose.prod.yml up -d
```

### Update to latest version

```bash
cd /opt/apps/fortyone
docker compose -f docker-compose.prod.yml pull
docker compose -f docker-compose.prod.yml up -d
```

### Build and push manually (optional)

```bash
echo $GITHUB_TOKEN | docker login ghcr.io -u aodihis --password-stdin
make docker-push API_URL=https://api.yourdomain.com
```

> **Note:** The Traefik labels in `docker-compose.prod.yml` assume an external Docker network named `traefik` and a cert resolver named `le`. Adjust these to match your `/opt/docker/traefik` configuration if they differ.
