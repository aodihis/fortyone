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
# Edit be/.env and set a strong JWT_SECRET
```

**Frontend** — copy and edit `fe/.env.example`:
```bash
cp fe/.env.example fe/.env
# Edit fe/.env to point API_URL at your backend
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
