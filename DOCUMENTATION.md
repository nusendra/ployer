# Ployer — Documentation

## Prerequisites

- **Rust** (1.75+) — [rustup.rs](https://rustup.rs)
- **Node.js** (20+) or **Bun** (1.0+) — for frontend builds
- **Docker** (optional) — required for container management features

## Quick Start

### 1. Build the backend

```bash
cargo build --workspace
```

### 2. Build the frontend

```bash
cd frontend
bun install    # or npm install
bun run build  # or npm run build
cd ..
```

### 3. Run the server

```bash
cargo run --bin ployer -- start
```

The API server starts on **http://localhost:3001**.

### 4. Verify

```bash
curl http://localhost:3001/api/v1/health
```

Expected response:

```json
{
  "status": "ok",
  "version": "0.1.0",
  "services": {
    "database": true,
    "docker": false
  }
}
```

`docker: false` is normal if Docker is not running.

## CLI Commands

| Command | Description |
|---------|-------------|
| `cargo run --bin ployer -- start` | Start the server (default) |
| `cargo run --bin ployer -- migrate` | Run database migrations only |

## Configuration

Default config is in `config/default.toml`:

```toml
[server]
host = "0.0.0.0"
port = 3001
base_domain = "localhost"

[database]
url = "sqlite://ployer.db?mode=rwc"

[auth]
jwt_secret = "change-me-in-production"
token_expiry_hours = 24

[docker]
socket_path = "/var/run/docker.sock"

[caddy]
admin_url = "http://localhost:2019"
```

The SQLite database file (`ployer.db`) is created automatically on first run.

## Frontend Development

For live-reload during frontend development:

```bash
cd frontend
bun run dev    # or npm run dev
```

The dev server runs on **http://localhost:5173** and proxies API calls to the backend.

## API Documentation

### Authentication

**Register a new user**

```bash
POST /api/v1/auth/register
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "password123",
  "name": "John Doe"
}
```

Response:

```json
{
  "user": {
    "id": "uuid",
    "email": "user@example.com",
    "name": "John Doe",
    "role": "admin",  // First user is admin, others are "user"
    "created_at": "2026-02-13T00:00:00Z",
    "updated_at": "2026-02-13T00:00:00Z"
  },
  "token": "eyJhbGciOiJIUzI1NiIs..."
}
```

**Login**

```bash
POST /api/v1/auth/login
Content-Type: application/json

{
  "email": "user@example.com",
  "password": "password123"
}
```

Response:

```json
{
  "user": { ... },
  "token": "eyJhbGciOiJIUzI1NiIs..."
}
```

**Get current user**

```bash
GET /api/v1/auth/me
Authorization: Bearer <token>
```

Response:

```json
{
  "user": { ... }
}
```

### Server Management

**List all servers**

```bash
GET /api/v1/servers
Authorization: Bearer <token>
```

Response:

```json
{
  "servers": [
    {
      "id": "uuid",
      "name": "localhost",
      "host": "localhost",
      "port": 22,
      "username": "root",
      "is_local": true,
      "status": "online",
      "last_seen_at": "2026-02-13T00:00:00Z",
      "created_at": "2026-02-13T00:00:00Z",
      "updated_at": "2026-02-13T00:00:00Z"
    }
  ]
}
```

**Create a server**

```bash
POST /api/v1/servers
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "Production Server",
  "host": "192.168.1.100",
  "port": 22,
  "username": "deploy",
  "ssh_key": "-----BEGIN PRIVATE KEY-----\n...",
  "is_local": false
}
```

Response (201 Created):

```json
{
  "server": { ... }
}
```

**Get server by ID**

```bash
GET /api/v1/servers/:id
Authorization: Bearer <token>
```

**Update server**

```bash
PUT /api/v1/servers/:id
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "Updated Name",
  "port": 2222
}
```

**Delete server**

```bash
DELETE /api/v1/servers/:id
Authorization: Bearer <token>
```

Response: 204 No Content

**Test server connection**

```bash
POST /api/v1/servers/:id/validate
Authorization: Bearer <token>
```

Response:

```json
{
  "reachable": true,
  "status": "online"
}
```

**Get server resources (local only)**

```bash
GET /api/v1/servers/:id/resources
Authorization: Bearer <token>
```

Response:

```json
{
  "stats": {
    "total_memory_mb": 16384,
    "used_memory_mb": 8192,
    "cpu_count": 8,
    "cpu_usage": 25.5
  }
}
```

### Health Check

```bash
GET /api/v1/health
```

Response:

```json
{
  "status": "ok",
  "version": "0.1.0",
  "services": {
    "database": true,
    "docker": false
  }
}
```
