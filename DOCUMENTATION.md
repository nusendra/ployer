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

### Container Management

**List containers**

```bash
GET /api/v1/containers?all=true
Authorization: Bearer <token>
```

Response:

```json
{
  "containers": [
    {
      "id": "abc123",
      "name": "my-app",
      "image": "nginx:latest",
      "state": "running",
      "status": "Up 2 hours",
      "created": 1707820800,
      "ports": [
        {
          "container_port": 80,
          "host_port": 8080,
          "protocol": "tcp"
        }
      ]
    }
  ]
}
```

**Create container**

```bash
POST /api/v1/containers
Authorization: Bearer <token>
Content-Type: application/json

{
  "image": "nginx:latest",
  "name": "my-nginx",
  "env": ["ENV=production", "DEBUG=false"],
  "ports": {
    "80/tcp": "8080"
  },
  "volumes": {
    "/host/data": "/app/data"
  },
  "cmd": ["nginx", "-g", "daemon off;"]
}
```

Response (201 Created):

```json
{
  "container_id": "abc123def456"
}
```

**Start/Stop/Restart container**

```bash
POST /api/v1/containers/:id/start
POST /api/v1/containers/:id/stop
POST /api/v1/containers/:id/restart
Authorization: Bearer <token>
```

Response: 204 No Content

**Get container logs**

```bash
GET /api/v1/containers/:id/logs?tail=100
Authorization: Bearer <token>
```

Response:

```json
{
  "logs": [
    "2024-02-13 10:00:00 Starting server...",
    "2024-02-13 10:00:01 Server listening on port 80"
  ]
}
```

**Get container stats**

```bash
GET /api/v1/containers/:id/stats
Authorization: Bearer <token>
```

Response:

```json
{
  "stats": {
    "cpu_usage": 25.5,
    "memory_usage_mb": 128.5,
    "memory_limit_mb": 512.0,
    "network_rx_bytes": 1048576,
    "network_tx_bytes": 524288
  }
}
```

**Delete container**

```bash
DELETE /api/v1/containers/:id
Authorization: Bearer <token>
```

Response: 204 No Content

### Network Management

**List networks**

```bash
GET /api/v1/networks
Authorization: Bearer <token>
```

**Create network**

```bash
POST /api/v1/networks
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "my-network",
  "driver": "bridge"
}
```

**Delete network**

```bash
DELETE /api/v1/networks/:id
Authorization: Bearer <token>
```

### Volume Management

**List volumes**

```bash
GET /api/v1/volumes
Authorization: Bearer <token>
```

**Create volume**

```bash
POST /api/v1/volumes
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "my-data"
}
```

**Delete volume**

```bash
DELETE /api/v1/volumes/:name
Authorization: Bearer <token>
```

### WebSocket (Real-time Updates)

**Connect to WebSocket**

```javascript
const token = localStorage.getItem('token');
const ws = new WebSocket(`ws://localhost:3001/api/v1/ws?token=${token}`);

ws.onopen = () => {
  // Subscribe to channels
  ws.send(JSON.stringify({
    type: 'subscribe',
    channel: 'container:abc123:logs'
  }));
};

ws.onmessage = (event) => {
  const message = JSON.parse(event.data);

  if (message.type === 'container_logs') {
    console.log(message.line);
  }

  if (message.type === 'container_stats') {
    console.log(message.cpu_usage, message.memory_usage_mb);
  }
};
```

**Available channels:**
- `server:<id>` - Server health updates
- `container:<id>:logs` - Container log streaming
- `container:<id>:stats` - Container resource stats
- `deployment:<id>` - Deployment progress

**Message types from server:**
- `server_health` - Server status changed
- `container_logs` - New log line from container
- `container_stats` - Container resource metrics
- `deployment_status` - Deployment status update
- `pong` - Response to ping
- `error` - Error message

### Application Management

**List applications**

```bash
GET /api/v1/applications
Authorization: Bearer <token>
```

Response:

```json
{
  "applications": [
    {
      "id": "uuid",
      "name": "my-app",
      "server_id": "uuid",
      "git_url": "git@github.com:user/repo.git",
      "git_branch": "main",
      "build_strategy": "dockerfile",
      "dockerfile_path": null,
      "port": 3000,
      "auto_deploy": true,
      "status": "running",
      "created_at": "2026-02-13T00:00:00Z",
      "updated_at": "2026-02-13T00:00:00Z"
    }
  ]
}
```

**Create application**

```bash
POST /api/v1/applications
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "my-app",
  "server_id": "uuid",
  "git_url": "git@github.com:user/repo.git",
  "git_branch": "main",
  "build_strategy": "dockerfile",
  "dockerfile_path": "./Dockerfile",
  "port": 3000,
  "auto_deploy": true,
  "env_vars": {
    "NODE_ENV": "production",
    "API_KEY": "secret123"
  }
}
```

Response (201 Created):

```json
{
  "application": { ... }
}
```

Note: If `git_url` is provided, a deploy key is automatically generated.

**Get application**

```bash
GET /api/v1/applications/:id
Authorization: Bearer <token>
```

**Update application**

```bash
PUT /api/v1/applications/:id
Authorization: Bearer <token>
Content-Type: application/json

{
  "name": "updated-name",
  "port": 3001,
  "auto_deploy": false
}
```

**Delete application**

```bash
DELETE /api/v1/applications/:id
Authorization: Bearer <token>
```

Response: 204 No Content

### Environment Variables

**List environment variables**

```bash
GET /api/v1/applications/:id/envs
Authorization: Bearer <token>
```

Response:

```json
{
  "env_vars": [
    {
      "key": "NODE_ENV",
      "value": "production"
    },
    {
      "key": "API_KEY",
      "value": "secret123"
    }
  ]
}
```

Note: Values are automatically decrypted.

**Add environment variable**

```bash
POST /api/v1/applications/:id/envs
Authorization: Bearer <token>
Content-Type: application/json

{
  "key": "DATABASE_URL",
  "value": "postgres://localhost/db"
}
```

Response: 201 Created

Note: Values are automatically encrypted with AES-256-GCM.

**Update environment variable**

```bash
PUT /api/v1/applications/:id/envs/:key
Authorization: Bearer <token>
Content-Type: application/json

{
  "key": "DATABASE_URL",
  "value": "postgres://newhost/db"
}
```

Response: 204 No Content

**Delete environment variable**

```bash
DELETE /api/v1/applications/:id/envs/:key
Authorization: Bearer <token>
```

Response: 204 No Content

### Deploy Keys

**Get deploy key**

```bash
GET /api/v1/applications/:id/deploy-key
Authorization: Bearer <token>
```

Response:

```json
{
  "public_key": "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQ...",
  "created_at": "2026-02-13T00:00:00Z"
}
```

**Generate new deploy key**

```bash
POST /api/v1/applications/:id/deploy-key
Authorization: Bearer <token>
```

Response (201 Created):

```json
{
  "public_key": "ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAACAQ...",
  "created_at": "2026-02-13T00:00:00Z"
}
```

Note: This generates a new RSA 4096 key pair. The old key is deleted.

### Deployments

**Trigger deployment**

```bash
POST /api/v1/applications/:id/deploy
Authorization: Bearer <token>
```

Response (201 Created):

```json
{
  "deployment": {
    "id": "uuid",
    "application_id": "uuid",
    "server_id": "uuid",
    "commit_sha": null,
    "commit_message": null,
    "status": "queued",
    "build_log": null,
    "container_id": null,
    "image_tag": "ployer-my-app:uuid",
    "started_at": "2026-02-14T00:00:00Z",
    "finished_at": null
  }
}
```

Note: Deployment runs in the background. Status will progress through: queued → cloning → building → deploying → running.

**List deployments**

```bash
GET /api/v1/deployments
Authorization: Bearer <token>

# Filter by application
GET /api/v1/deployments?application_id=uuid
Authorization: Bearer <token>
```

Response:

```json
{
  "deployments": [
    {
      "id": "uuid",
      "application_id": "uuid",
      "server_id": "uuid",
      "commit_sha": "abc123",
      "commit_message": "Fix bug in auth",
      "status": "running",
      "build_log": "Step 1/5 : FROM node:18...\n...",
      "container_id": "docker-container-id",
      "image_tag": "ployer-my-app:uuid",
      "started_at": "2026-02-14T00:00:00Z",
      "finished_at": "2026-02-14T00:05:00Z"
    }
  ]
}
```

**Get deployment details**

```bash
GET /api/v1/deployments/:id
Authorization: Bearer <token>
```

Response:

```json
{
  "deployment": {
    "id": "uuid",
    "application_id": "uuid",
    "status": "running",
    "build_log": "Full build logs...",
    ...
  }
}
```

**Cancel deployment**

```bash
POST /api/v1/deployments/:id/cancel
Authorization: Bearer <token>
```

Response: 204 No Content

Note: Can only cancel deployments that are queued, cloning, building, or deploying. Running deployments cannot be cancelled.

### Domain Management

**List domains for an application**

```bash
GET /api/v1/applications/:id/domains
Authorization: Bearer <token>
```

Response:

```json
{
  "domains": [
    {
      "id": "uuid",
      "application_id": "uuid",
      "domain": "my-app.example.com",
      "is_primary": true,
      "ssl_active": true,
      "created_at": "2026-02-15T00:00:00Z"
    }
  ]
}
```

**Add a custom domain**

```bash
POST /api/v1/applications/:id/domains
Authorization: Bearer <token>
Content-Type: application/json

{
  "domain": "app.example.com",
  "is_primary": false
}
```

Response (201 Created):

```json
{
  "domain": {
    "id": "uuid",
    "application_id": "uuid",
    "domain": "app.example.com",
    "is_primary": false,
    "ssl_active": false,
    "created_at": "2026-02-15T00:00:00Z"
  }
}
```

Note: When you deploy an application, a subdomain is automatically generated in the format `{app-name}.{base-domain}`.

**Remove a domain**

```bash
DELETE /api/v1/applications/:id/domains/:domain
Authorization: Bearer <token>
```

Response: 204 No Content

**Verify domain DNS**

```bash
POST /api/v1/applications/:id/domains/:domain/verify
Authorization: Bearer <token>
```

Response:

```json
{
  "success": true,
  "message": "Domain verified successfully"
}
```

Note: This checks if the domain points to the server and updates the SSL status.

**Set domain as primary**

```bash
POST /api/v1/applications/:id/domains/:domain/primary
Authorization: Bearer <token>
```

Response: 204 No Content

Note: Only one domain can be primary per application.

### Webhooks

**Create webhook**

```bash
POST /api/v1/applications/:id/webhooks
Authorization: Bearer <token>
Content-Type: application/json

{
  "provider": "github"  # or "gitlab"
}
```

Response:

```json
{
  "id": "webhook-uuid",
  "application_id": "app-uuid",
  "provider": "github",
  "webhook_url": "http://your-domain.com/api/v1/webhooks/github?app_id=app-uuid",
  "secret": "generated-secret-token",
  "enabled": true
}
```

**Get webhook configuration**

```bash
GET /api/v1/applications/:id/webhooks
Authorization: Bearer <token>
```

Response: Same as create webhook response, or 404 if no webhook configured.

**Delete webhook**

```bash
DELETE /api/v1/applications/:id/webhooks
Authorization: Bearer <token>
```

Response: 204 No Content

**List webhook deliveries**

```bash
GET /api/v1/applications/:id/webhooks/deliveries
Authorization: Bearer <token>
```

Response:

```json
[
  {
    "id": "delivery-uuid",
    "provider": "github",
    "event_type": "push",
    "branch": "main",
    "commit_sha": "abc123def456",
    "commit_message": "Fix bug in deployment",
    "author": "John Doe",
    "status": "success",  # or "failed", "skipped"
    "deployment_id": "deployment-uuid",
    "delivered_at": "2024-01-15T10:30:00Z"
  }
]
```

**GitHub webhook endpoint**

```bash
POST /api/v1/webhooks/github?app_id=<app-uuid>
X-Hub-Signature-256: sha256=<hmac-signature>
Content-Type: application/json

{
  "ref": "refs/heads/main",
  "head_commit": {
    "id": "abc123",
    "message": "Update feature",
    "author": { "name": "Jane Smith" }
  },
  "repository": {
    "clone_url": "https://github.com/user/repo.git"
  }
}
```

**GitLab webhook endpoint**

```bash
POST /api/v1/webhooks/gitlab?app_id=<app-uuid>
X-Gitlab-Token: <secret-token>
Content-Type: application/json

{
  "ref": "refs/heads/main",
  "checkout_sha": "abc123",
  "commits": [
    {
      "message": "Update feature",
      "author": { "name": "Jane Smith" }
    }
  ],
  "repository": {
    "git_ssh_url": "git@gitlab.com:user/repo.git"
  }
}
```

**Webhook Configuration Guide**

*For GitHub:*
1. Go to your repository → Settings → Webhooks → Add webhook
2. Paste the webhook URL from Ployer
3. Set Content type to `application/json`
4. Paste the secret token from Ployer
5. Select event: "Push events"
6. Click "Add webhook"

*For GitLab:*
1. Go to your repository → Settings → Webhooks
2. Paste the webhook URL from Ployer
3. Paste the secret token
4. Check "Push events"
5. Click "Add webhook"

**Auto-Deploy Behavior:**
- Webhook validates the signature/token
- Checks if the push is on the configured branch (from application settings)
- If branch matches, triggers automatic deployment
- Records delivery status (success/failed/skipped)
- Links delivery to the triggered deployment

### Monitoring

**Configure health check**

```bash
POST /api/v1/applications/:app_id/health-check
Authorization: Bearer <token>
Content-Type: application/json

{
  "path": "/health",
  "interval_seconds": 30,
  "timeout_seconds": 5,
  "healthy_threshold": 2,
  "unhealthy_threshold": 3
}
```

Response:

```json
{
  "id": "health-check-uuid",
  "application_id": "app-uuid",
  "path": "/health",
  "interval_seconds": 30,
  "timeout_seconds": 5,
  "healthy_threshold": 2,
  "unhealthy_threshold": 3,
  "created_at": "2024-01-15T10:00:00Z"
}
```

**Get health check configuration**

```bash
GET /api/v1/applications/:app_id/health-check
Authorization: Bearer <token>
```

Response: Same as configure health check, or 404 if not configured.

**Get health check results**

```bash
GET /api/v1/applications/:app_id/health-check/results
Authorization: Bearer <token>
```

Response:

```json
[
  {
    "id": "result-uuid",
    "container_id": "abc123",
    "status": "healthy",
    "response_time_ms": 45,
    "status_code": 200,
    "error_message": null,
    "checked_at": "2024-01-15T10:30:00Z"
  }
]
```

**Get container stats**

```bash
GET /api/v1/applications/:app_id/stats?hours=1
Authorization: Bearer <token>
```

Query parameters:
- `hours` (optional, default: 1) - Number of hours of historical data to retrieve

Response:

```json
[
  {
    "container_id": "abc123",
    "cpu_percent": 5.2,
    "memory_mb": 128.5,
    "memory_limit_mb": 512.0,
    "network_rx_mb": 10.5,
    "network_tx_mb": 5.2,
    "recorded_at": "2024-01-15T10:30:00Z"
  }
]
```

**Monitoring Features:**
- Health checks run automatically every 15 seconds for all configured applications
- Auto-restart triggered when consecutive unhealthy checks exceed threshold
- Container stats collected every 60 seconds
- Stats retained for 24 hours, then automatically cleaned up
- WebSocket events broadcast on health status changes

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
