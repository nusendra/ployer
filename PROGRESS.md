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
├── migrations/
│   ├── 001_initial.sql            # Initial schema (9 core tables)
│   └── 002_webhooks.sql           # Webhook tables (webhooks, webhook_deliveries)
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

11 tables defined in `migrations/`:

| Table | Purpose | Migration |
|-------|---------|-----------|
| `users` | User accounts with email, password_hash, role | 001_initial.sql |
| `api_keys` | API key authentication per user | 001_initial.sql |
| `servers` | Local + remote servers with SSH config | 001_initial.sql |
| `applications` | App config: git URL, branch, build strategy, port | 001_initial.sql |
| `environment_variables` | Encrypted env vars per application | 001_initial.sql |
| `domains` | Custom domains per application | 001_initial.sql |
| `deployments` | Deployment history with build logs | 001_initial.sql |
| `deploy_keys` | SSH deploy keys per application | 001_initial.sql |
| `health_checks` | Health check config per application | 001_initial.sql |
| `webhooks` | Webhook config per application (provider, secret, enabled) | 002_webhooks.sql |
| `webhook_deliveries` | Webhook delivery history with status and commit metadata | 002_webhooks.sql |

SQLite WAL mode enabled for concurrent reads.

## API Endpoints

| Method | Path | Status |
|--------|------|--------|
| GET | `/api/v1/health` | Done |
| POST | `/api/v1/auth/register` | Done |
| POST | `/api/v1/auth/login` | Done |
| GET | `/api/v1/auth/me` | Done |
| GET | `/api/v1/servers` | Done |
| POST | `/api/v1/servers` | Done |
| GET | `/api/v1/servers/:id` | Done |
| PUT | `/api/v1/servers/:id` | Done |
| DELETE | `/api/v1/servers/:id` | Done |
| POST | `/api/v1/servers/:id/validate` | Done |
| GET | `/api/v1/servers/:id/resources` | Done |
| GET | `/api/v1/containers` | Done |
| POST | `/api/v1/containers` | Done |
| GET | `/api/v1/containers/:id` | Done |
| POST | `/api/v1/containers/:id/start` | Done |
| POST | `/api/v1/containers/:id/stop` | Done |
| POST | `/api/v1/containers/:id/restart` | Done |
| DELETE | `/api/v1/containers/:id` | Done |
| GET | `/api/v1/containers/:id/logs` | Done |
| GET | `/api/v1/containers/:id/stats` | Done |
| GET | `/api/v1/networks` | Done |
| POST | `/api/v1/networks` | Done |
| GET | `/api/v1/networks/:id` | Done |
| DELETE | `/api/v1/networks/:id` | Done |
| GET | `/api/v1/volumes` | Done |
| POST | `/api/v1/volumes` | Done |
| GET | `/api/v1/volumes/:name` | Done |
| DELETE | `/api/v1/volumes/:name` | Done |
| GET | `/api/v1/ws` | Done |
| GET | `/api/v1/applications` | Done |
| POST | `/api/v1/applications` | Done |
| GET | `/api/v1/applications/:id` | Done |
| PUT | `/api/v1/applications/:id` | Done |
| DELETE | `/api/v1/applications/:id` | Done |
| GET | `/api/v1/applications/:id/envs` | Done |
| POST | `/api/v1/applications/:id/envs` | Done |
| PUT | `/api/v1/applications/:id/envs/:key` | Done |
| DELETE | `/api/v1/applications/:id/envs/:key` | Done |
| GET | `/api/v1/applications/:id/deploy-key` | Done |
| POST | `/api/v1/applications/:id/deploy-key` | Done |
| POST | `/api/v1/applications/:id/deploy` | Done |
| GET | `/api/v1/deployments` | Done |
| GET | `/api/v1/deployments/:id` | Done |
| POST | `/api/v1/deployments/:id/cancel` | Done |
| GET | `/api/v1/applications/:id/domains` | Done |
| POST | `/api/v1/applications/:id/domains` | Done |
| DELETE | `/api/v1/applications/:id/domains/:domain` | Done |
| POST | `/api/v1/applications/:id/domains/:domain/verify` | Done |
| POST | `/api/v1/applications/:id/domains/:domain/primary` | Done |
| POST | `/api/v1/applications/:id/webhooks` | Done |
| GET | `/api/v1/applications/:id/webhooks` | Done |
| DELETE | `/api/v1/applications/:id/webhooks` | Done |
| GET | `/api/v1/applications/:id/webhooks/deliveries` | Done |
| POST | `/api/v1/webhooks/github` | Done |
| POST | `/api/v1/webhooks/gitlab` | Done |

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

### Phase 2: Server Management — COMPLETE

**Completed:**
- ServerRepository in ployer-db with full CRUD operations (create, find_by_id, list, update, update_status, delete, find_local)
- Server API endpoints: list, create, get, update, delete, validate, resources
- Auth helper function `extract_user_id()` for protected routes
- Local server auto-detection on startup (registers hostname as local server)
- TCP connection testing via `ServerManager::test_ssh_connection()`
- POST `/api/v1/servers/:id/validate` — tests server connectivity, updates status
- GET `/api/v1/servers/:id/resources` — returns CPU/RAM stats for local servers
- Background health monitor service (30s interval, Tokio task)
- Health monitor checks all servers, updates status, broadcasts WsEvent::ServerHealth
- Frontend server management UI with:
  - Server list with status indicators (online/offline/unknown)
  - Add server form (name, host, port, username, SSH key)
  - Test connection button per server
  - View resources modal for local servers (CPU, memory stats)
  - Delete server with confirmation
  - Auto-refresh on actions

**Verified:**
- Local server auto-registered on first startup with status "online"
- Server CRUD operations work through API endpoints
- Connection validation updates server status in database
- Health monitor runs every 30 seconds and updates server statuses
- Frontend displays servers with correct status badges
- Resource stats modal shows live CPU/memory data for local server

---

### Phase 3: Docker Management — COMPLETE

**Completed:**
- Enhanced DockerClient with full container lifecycle management
- Container operations: list, inspect, create, start, stop, restart, remove
- Container logs streaming (tail support) and real-time stats
- Network management: list, create, inspect, remove
- Volume management: list, create, inspect, remove
- WebSocket infrastructure with JWT authentication and channel subscriptions
- Container API endpoints: GET/POST/DELETE with proper error handling
- Network and volume API endpoints
- Frontend container management UI with:
  - Container list with status indicators
  - Create container form (env vars, ports, volumes, network)
  - Start/stop/restart/delete actions with loading states
  - Log viewer modal with live streaming
  - Stats viewer modal with live updates
  - WebSocket client with auto-reconnect
- WebSocket real-time updates for container logs and stats

**Verified:**
- Container CRUD operations work through API
- WebSocket connects with JWT auth
- Real-time log streaming functional
- Resource stats update in real-time
- Network and volume management functional
- Frontend displays containers with proper status badges

---

### Phase 4: Application CRUD + Git — COMPLETE

**Completed:**
- ApplicationRepository, EnvVarRepository, DeployKeyRepository in ployer-db with full CRUD operations
- Enhanced GitService with SSH deploy key generation (RSA 4096)
- Git operations: clone with SSH auth, pull with fast-forward merge, commit info retrieval, branch checkout
- Encryption service with AES-256-GCM for sensitive data (environment variables, SSH private keys)
- Crypto module with comprehensive unit tests for encryption/decryption
- AppConfig.get_secret_key() method for deriving encryption key from JWT secret using SHA-256
- Application API endpoints: list, create, get, update, delete
- Environment variable endpoints: list, add, update, delete (automatic encryption/decryption)
- Deploy key endpoints: get public key, generate new key pair
- Frontend application management UI with:
  - Application list with status indicators and build strategy badges
  - Create application form with build strategy selection (Dockerfile, Nixpacks, Docker Compose)
  - Git configuration (URL, branch) and auto-deploy toggle
  - Environment variables editor with add/delete functionality
  - Deploy key viewer with regeneration capability
  - Edit application modal with all configurable fields
  - Server selection from available servers
- Shared TypeScript types file for better code organization
- Applications link in navigation menu

**Verified:**
- Application CRUD operations working through API endpoints
- Environment variables automatically encrypted when stored, decrypted when retrieved
- Deploy keys automatically generated when application created with git_url
- SSH keys are RSA 4096 in PEM format
- Private keys encrypted at rest with AES-256-GCM
- Frontend UI displays applications correctly with proper server associations
- Environment variables can be added, updated, and deleted through UI
- Deploy key regeneration works and displays public key for repository configuration

---

### Phase 5: Deployment Pipeline — COMPLETE

**Completed:**
- DeploymentRepository with full CRUD operations and status tracking
- Docker build service with streaming log output via mpsc channels
- DeploymentService orchestrator that manages the full pipeline:
  1. Clone git repository with SSH authentication
  2. Build Docker image with streaming logs
  3. Create and start new container
  4. Health check (5-second wait for MVP)
  5. Rolling update preparation
  6. Status updates and log broadcasting
- Deployment API endpoints: trigger deploy, list deployments, get details, cancel
- WebSocket integration for real-time deployment logs and status updates
- Frontend deployment UI with:
  - Deploy button on applications page
  - Deployment history modal showing all deployments per app
  - Live deployment log viewer
  - Deployment status badges (queued, cloning, building, deploying, running, failed, cancelled)
  - Commit information display
- Build context tar creation for Docker image building
- Real-time build log streaming from Docker daemon
- Deployment status lifecycle management
- Container creation with automatic naming and tagging

**Verified:**
- Deployment creation and execution working end-to-end
- Docker images built with tag format `ployer-{app-name}:{deployment-id}`
- Containers created with name format `{app-name}-{deployment-id}`
- Build logs stored in database and streamed via WebSocket
- Deployment history accessible via API and UI
- Status updates broadcast to connected WebSocket clients
- Deploy keys automatically used for private repository access

---

### Phase 6: Domains + Caddy Integration — COMPLETE

**Completed:**
- Enhanced CaddyClient with dynamic route management (add_route, remove_route, list_routes, get_ssl_status)
- DomainRepository with full CRUD operations and SSL status tracking
- Domain API endpoints: list, add, remove, verify DNS, set primary domain
- Auto-subdomain generation integrated into deployment pipeline
- Automatic subdomain creation: `{app-name}.{base-domain}` for every successful deployment
- Caddy integration in DeploymentService for automatic reverse proxy configuration
- Frontend domain management UI:
  - View all domains per application
  - Add custom domains with DNS instructions
  - Set primary domain
  - Verify domain DNS
  - SSL status badges (active/pending)
  - Remove domains
  - Auto-generated subdomain display

**Verified:**
- Domain CRUD operations work through API endpoints
- Auto-subdomains created automatically on deployment
- Caddy routes configured dynamically (when Caddy is available)
- Frontend displays domains with SSL status
- DNS configuration instructions shown to users
- Primary domain can be set and unset
- Domain verification updates SSL status

---

### Phase 7: Webhooks + Auto-Deploy — COMPLETE

**Completed:**
- Webhook tables schema (webhooks, webhook_deliveries) in new migration 002_webhooks.sql
- Webhook and WebhookDelivery models with WebhookProvider enum (GitHub, GitLab)
- WebhookDeliveryStatus enum (Success, Failed, Skipped) for tracking delivery outcomes
- WebhookRepository with full CRUD operations and delivery history tracking
- Webhook payload parser service with:
  - GitHub push event parsing (ref, head_commit, repository)
  - GitLab push event parsing (ref, checkout_sha, commits, repository)
  - HMAC-SHA256 signature verification for GitHub (X-Hub-Signature-256 header)
  - Token-based verification for GitLab (X-Gitlab-Token header)
- Webhook API endpoints:
  - POST/GET/DELETE `/applications/:app_id/webhooks` — create/get/delete webhook
  - GET `/applications/:app_id/webhooks/deliveries` — list webhook delivery history
  - POST `/webhooks/github?app_id=<id>` — GitHub webhook receiver
  - POST `/webhooks/gitlab?app_id=<id>` — GitLab webhook receiver
- Auto-deploy integration:
  - Validates incoming webhook against configured secret
  - Checks if push is on configured branch
  - Triggers deployment via DeploymentService if branch matches
  - Records delivery status (success/failed/skipped) with commit details
  - Links webhook delivery to deployment ID
- public_url field added to ServerConfig for webhook URL generation
- Frontend webhook management UI:
  - Webhook configuration modal accessible from application actions
  - Provider selection (GitHub/GitLab)
  - Webhook URL and secret token display with copy support
  - Step-by-step instructions for GitHub and GitLab webhook setup
  - Recent webhook deliveries list with:
    - Event type, status badges (success/failed/skipped)
    - Branch, commit SHA, author, commit message
    - Timestamp of delivery
  - Delete webhook configuration option

**Dependencies Added:**
- hmac = "0.12" — HMAC-SHA256 for GitHub webhook verification
- hex = "0.4" — Hexadecimal encoding for signature comparison

**Verified:**
- Webhook creation generates unique secret token
- Webhook URLs properly formatted with app_id query parameter
- GitHub signature verification rejects invalid signatures
- GitLab token verification works correctly
- Auto-deploy triggered only when push is on configured branch
- Webhook deliveries recorded with commit metadata
- Frontend displays webhook setup instructions correctly
- Delivery history shows status and commit details

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
