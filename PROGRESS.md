# Ployer — Development Progress

A lightweight Coolify alternative built with Rust + SvelteKit + Caddy, targeting <50MB RAM.

## Tech Stack

| Layer | Technology |
|-------|-----------|
| Backend | Rust (Axum) |
| Frontend | SvelteKit (adapter-static) |
| Reverse Proxy | Caddy |
| Database | SQLite (sqlx) |
| Containers | Docker (bollard) |
| Real-time | WebSocket (Axum built-in) |

## Project Structure

```
ployer/
├── Cargo.toml                     # Workspace root
├── config/default.toml            # Default configuration
├── migrations/001_initial.sql     # SQLite schema (all 9 tables)
├── crates/
│   ├── ployer-core/               # Domain models, config, error types
│   ├── ployer-db/                 # SQLite pool + migrations
│   ├── ployer-docker/             # Docker client (bollard)
│   ├── ployer-proxy/              # Caddy Admin API client (reqwest)
│   ├── ployer-git/                # Git clone (git2)
│   ├── ployer-server/             # System stats (sysinfo)
│   └── ployer-api/                # Axum HTTP server (binary entry point)
│       └── src/
│           ├── main.rs            # CLI + server startup
│           ├── app_state.rs       # SharedState (db, docker, caddy, ws)
│           └── routes/
│               ├── mod.rs         # API router
│               └── health.rs      # GET /api/v1/health
└── frontend/                      # SvelteKit (static build)
    └── src/
        ├── app.css                # Global dark theme styles
        ├── lib/api/client.ts      # Typed API client with auth headers
        ├── lib/stores/auth.ts     # Auth store (token, user)
        └── routes/
            ├── +layout.svelte     # Sidebar layout (Dashboard, Apps, Servers, Settings)
            ├── +layout.ts         # SPA mode (prerender, no SSR)
            ├── +page.svelte       # Dashboard with stat cards
            ├── apps/+page.svelte  # Applications placeholder
            ├── servers/+page.svelte # Servers placeholder
            └── settings/+page.svelte # Settings placeholder
```

## Database Schema

9 tables defined in `migrations/001_initial.sql`:

| Table | Purpose |
|-------|---------|
| `users` | User accounts with email, password_hash, role |
| `api_keys` | API key authentication per user |
| `servers` | Local + remote servers with SSH config |
| `applications` | App config: git URL, branch, build strategy, port |
| `environment_variables` | Encrypted env vars per application |
| `domains` | Custom domains per application |
| `deployments` | Deployment history with build logs |
| `deploy_keys` | SSH deploy keys per application |
| `health_checks` | Health check config per application |

SQLite WAL mode enabled for concurrent reads.

## API Endpoints

| Method | Path | Status |
|--------|------|--------|
| GET | `/api/v1/health` | Done |
| POST | `/api/v1/auth/register` | Done |
| POST | `/api/v1/auth/login` | Done |
| GET | `/api/v1/auth/me` | Done |

---

## Phase Progress

### Phase 0: Project Skeleton — COMPLETE

**Completed:**
- Cargo workspace with 7 crates, all compiling clean
- Domain models: User, Server, Application, Deployment, Domain, EnvironmentVariable, DeployKey, HealthCheck, WsEvent
- AppConfig with defaults (server, database, auth, docker, caddy)
- PloyerError enum with thiserror
- SQLite connection pool (WAL mode, foreign keys) + migration runner
- DockerClient stub (bollard — connect, ping)
- CaddyClient stub (reqwest — ping)
- GitService stub (git2 — clone)
- ServerManager stub (sysinfo — local CPU/RAM stats)
- Axum server with CLI (clap): `ployer start`, `ployer migrate`
- SharedState: SqlitePool, DockerClient, CaddyClient, AppConfig, broadcast::Sender<WsEvent>
- GET `/api/v1/health` — returns status, version, service checks
- SvelteKit frontend: adapter-static, dark theme, sidebar nav, dashboard, stub pages
- API client with Bearer token support, auth store

**Verified:**
- `cargo build --workspace` — compiles (only expected dead_code warnings)
- `bun run build` (frontend) — produces static output in `frontend/build/`
- `curl localhost:3001/api/v1/health` → `{"status":"ok","version":"0.1.0","services":{"database":true,"docker":false}}`

---

### Phase 1: Auth + User Management — COMPLETE

**Completed:**
- UserRepository and ApiKeyRepository in ployer-db with CRUD operations
- Password hashing with argon2 (hash_password, verify_password)
- JWT token generation/validation with jsonwebtoken
- AuthService with first-user-is-admin logic
- Auth middleware for protected routes
- POST `/api/v1/auth/register` — user registration, first user becomes admin
- POST `/api/v1/auth/login` — email/password authentication, returns JWT
- GET `/api/v1/auth/me` — get current user from token
- Frontend login/register page with tab switcher
- Auth store integration with localStorage token persistence
- Protected layout with auth check and logout button
- Redirect to /login when unauthenticated

**Verified:**
- Registration creates user with role "admin" for first user, "user" for others
- Login validates credentials and returns JWT token
- /me endpoint validates token and returns user data
- Frontend redirects to login when no token present
- Logout clears token and redirects to login

---

### Phase 2: Server Management — PENDING

**Scope:**
- Servers table + repo
- SSH client via russh, connection testing
- Local server auto-detection
- Background health monitoring (30s interval)
- Frontend: server list, add form, health indicators

---

### Phase 3: Docker Management — PENDING

**Scope:**
- Container CRUD, log streaming, resource stats
- WebSocket infrastructure (broadcast, subscriptions)
- Network + volume management
- Frontend: container list, terminal (xterm.js), resource charts

---

### Phase 4: Application CRUD + Git — PENDING

**Scope:**
- Git clone via git2, deploy key generation
- App CRUD endpoints, encrypted env var storage
- Frontend: app list, create wizard, env var editor

---

### Phase 5: Deployment Pipeline — PENDING

**Scope:**
- Build strategy detection (Dockerfile vs Nixpacks)
- Image building, deployment orchestrator
- Rolling updates with health check gating
- Frontend: deploy button, live build log, history, rollback

---

### Phase 6: Domains + Caddy Integration — PENDING

**Scope:**
- Caddy Admin API client (dynamic route management)
- Automatic HTTPS, auto subdomain generation
- Frontend: domain management, SSL status

---

### Phase 7: Webhooks + Auto-Deploy — PENDING

**Scope:**
- GitHub/GitLab webhook parsing + signature verification
- Auto-trigger deployment on push
- Frontend: webhook URL display, delivery history

---

### Phase 8: Monitoring + Health Checks — PENDING

**Scope:**
- Per-app health check polling, auto-restart
- Container stats aggregation
- Frontend: dashboard charts, health indicators

---

### Phase 9: Polish + Hardening — PENDING

**Scope:**
- Error handling audit, rate limiting, input validation
- CORS, graceful shutdown, config file support
- CLI: install, reset-password
- Structured JSON logging, frontend dark mode polish
