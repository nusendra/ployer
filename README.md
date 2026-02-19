# Ployer

A lightweight, self-hosted PaaS — deploy your apps from Git with automatic SSL, health checks, and webhooks. Built with Rust + SvelteKit, targeting **<50MB RAM** idle.

> Think Coolify, but leaner.

---

## Features

- **One-command deploy** — push to Git, Ployer builds and deploys automatically
- **Automatic SSL** — via Caddy + Let's Encrypt, zero config
- **Webhooks** — GitHub and GitLab push events trigger auto-deploys
- **Health checks** — HTTP polling with auto-restart on failure
- **Container stats** — CPU, memory, and network I/O monitoring
- **Real-time logs** — WebSocket streaming for build and runtime logs
- **Multi-server** — manage apps across multiple servers
- **Encrypted secrets** — environment variables encrypted at rest (AES-256-GCM)

---

## Requirements

| | Minimum | Recommended |
|---|---|---|
| **OS** | Ubuntu 22.04 / Debian 12 | Ubuntu 24.04 LTS |
| **RAM** | 1 GB | 2 GB+ |
| **CPU** | 1 vCPU | 2 vCPU |
| **Disk** | 20 GB | 40 GB+ |
| **Arch** | x86_64 | x86_64 / arm64 |

> **Important:** Install on a **fresh, dedicated server**. Ployer owns ports 80 and 443 via Caddy. It will conflict with Nginx, Apache, Coolify, or any other reverse proxy already running on those ports.

---

## Quick Install

Point your domain's DNS `A record` to your server IP first, then run:

```bash
curl -fsSL https://raw.githubusercontent.com/nusendra/ployer/main/install.sh | sudo bash
```

> **Note:** `curl | bash` runs in non-interactive mode and will auto-detect your server IP. To use a custom domain with HTTPS, download and run the script directly instead:
> ```bash
> curl -fsSL https://raw.githubusercontent.com/nusendra/ployer/main/install.sh -o install.sh
> sudo bash install.sh
> ```

The installer will:
1. Detect your OS and install Docker if needed
2. Ask for your domain (or use server IP for quick testing)
3. Generate a secure JWT secret automatically
4. Build and start all services
5. Print the dashboard URL when ready

**That's it.** HTTPS is provisioned automatically when you use a domain.

---

## Manual Installation

If you prefer to set things up yourself:

### 1. Install Docker

```bash
curl -fsSL https://get.docker.com | bash
```

### 2. Clone the repository

```bash
git clone https://github.com/nusendra/ployer.git /data/ployer
cd /data/ployer
```

### 3. Configure environment

```bash
cp .env.example .env
nano .env
```

Set at minimum:

```env
PLOYER_JWT_SECRET=your-long-random-secret-here
PLOYER_BASE_DOMAIN=ployer.yourdomain.com
PLOYER_PUBLIC_URL=https://ployer.yourdomain.com
PLOYER_ALLOWED_ORIGINS=https://ployer.yourdomain.com
```

### 4. Configure Caddy

Edit `Caddyfile` and replace `your-staging-domain.com` with your actual domain:

```
ployer.yourdomain.com {
    reverse_proxy ployer:3001
}
```

### 5. Start services

```bash
docker compose up -d --build
```

### 6. Open the dashboard

Navigate to `https://ployer.yourdomain.com` — you'll be prompted to create the first admin account.

---

## Upgrading

Re-run the install script — it detects an existing installation, pulls the latest code, and restarts:

```bash
curl -fsSL https://raw.githubusercontent.com/nusendra/ployer/main/install.sh | sudo bash
```

Or manually:

```bash
cd /data/ployer
git pull
docker compose up -d --build
```

---

## Configuration

All configuration is via environment variables in `/data/ployer/.env`:

| Variable | Default | Description |
|---|---|---|
| `PLOYER_JWT_SECRET` | *(required)* | Secret key for signing JWT tokens. Use a long random string. |
| `PLOYER_BASE_DOMAIN` | `localhost` | Base domain for auto-generated app subdomains |
| `PLOYER_PUBLIC_URL` | `http://localhost` | Full public URL of the Ployer dashboard |
| `PLOYER_ALLOWED_ORIGINS` | `*` | CORS allowed origins. Lock down in production. |
| `PLOYER_DATABASE_URL` | `sqlite:///data/ployer.db` | SQLite database path |
| `PLOYER_PORT` | `3001` | Internal API port |
| `PLOYER_CADDY_URL` | `http://caddy:2019` | Caddy Admin API URL |
| `PLOYER_DOCKER_SOCKET` | `/var/run/docker.sock` | Docker socket path |
| `LOG_FORMAT` | *(plain text)* | Set to `json` for structured JSON logging |

After editing `.env`, restart:

```bash
docker compose -f /data/ployer/docker-compose.yml up -d
```

---

## Resetting a Password

If you get locked out:

```bash
docker compose -f /data/ployer/docker-compose.yml exec ployer \
  ./ployer reset-password --email you@example.com --password newpassword123
```

---

## Common Commands

```bash
# View live logs
docker compose -f /data/ployer/docker-compose.yml logs -f

# View only API logs
docker compose -f /data/ployer/docker-compose.yml logs -f ployer

# Stop Ployer
docker compose -f /data/ployer/docker-compose.yml down

# Restart Ployer
docker compose -f /data/ployer/docker-compose.yml restart ployer

# Open a shell inside the container
docker compose -f /data/ployer/docker-compose.yml exec ployer sh
```

---

## Deploying an App Through Ployer

1. **Add a server** — Ployer auto-registers the local server on first start
2. **Create an application** — provide a Git URL, branch, and build strategy
3. **Add the deploy key** — copy the public key from Ployer into your repository's deploy keys
4. **Deploy** — click Deploy or push to the configured branch via webhook

### Build strategies

| Strategy | Description |
|---|---|
| **Dockerfile** | Builds using a `Dockerfile` in your repo root |
| **Nixpacks** | Auto-detects language and builds without a Dockerfile |
| **Docker Compose** | Deploys using a `docker-compose.yml` in your repo |

### Setting up webhooks (auto-deploy on push)

1. Go to your application → **Webhooks** → **Configure**
2. Select your Git provider (GitHub or GitLab)
3. Copy the webhook URL and secret
4. Add it to your repository's webhook settings
5. Push to your configured branch — Ployer deploys automatically

---

## Architecture

```
Internet
    │
    ▼
 Caddy (80/443)          ← TLS termination, auto SSL
    │
    ▼
 Ployer (3001)           ← Rust/Axum API + SvelteKit frontend
    │
    ├── SQLite (/data)   ← Persistent database
    ├── Docker socket    ← Container management
    └── Caddy Admin API  ← Dynamic reverse proxy routes
```

- **Backend** — Rust (Axum), SQLite via sqlx, bollard for Docker
- **Frontend** — SvelteKit (static build, served by the Rust binary)
- **Proxy** — Caddy 2 (automatic HTTPS, dynamic routing)
- **Database** — SQLite with WAL mode (no separate database server needed)

---

## Troubleshooting

**Dashboard not loading after install**
```bash
docker compose -f /data/ployer/docker-compose.yml logs caddy
docker compose -f /data/ployer/docker-compose.yml logs ployer
```

**SSL certificate not issued**
- Make sure your domain's DNS A record points to the server before running the installer
- Port 80 must be open on your firewall (`ufw allow 80` and `ufw allow 443`)

**Cannot connect to Docker**
```bash
ls -la /var/run/docker.sock   # should exist
systemctl status docker        # should be active
```

**Database errors on startup**
- The `/data` volume is created automatically. If permissions are wrong:
```bash
docker compose -f /data/ployer/docker-compose.yml down
docker volume rm ployer_ployer-data
docker compose -f /data/ployer/docker-compose.yml up -d
```

---

## License

MIT — see [LICENSE](LICENSE)
