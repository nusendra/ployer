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

**Prerequisites:** Docker must be installed and running before installing Ployer.

> **Important:** Install on a **fresh, dedicated server**. Ployer owns ports 80 and 443 via Caddy. It will conflict with Nginx, Apache, Coolify, or any other reverse proxy already running on those ports.

---

## Quick Install

Point your domain's DNS `A record` to your server IP first, then run:

```bash
curl -fsSL https://ployer.nusendra.com/install.sh | sudo bash
```

> **Note:** `curl | bash` runs in non-interactive mode and will auto-detect your server IP. To use a custom domain with HTTPS, download and run the script directly:
> ```bash
> curl -fsSL https://ployer.nusendra.com/install.sh -o install.sh
> sudo bash install.sh
> ```

The installer will:
1. Detect your OS and architecture
2. Download the pre-built binary from GitHub Releases (no compilation needed)
3. Ask for your domain or IP address
4. Generate a secure JWT secret automatically
5. Install and configure Caddy as a reverse proxy
6. Set up systemd services for both Ployer and Caddy
7. Start everything and print the dashboard URL

**That's it.** HTTPS is provisioned automatically when you use a domain.

---

## Upgrading

Re-run the install script — it detects the current version and upgrades automatically:

```bash
curl -fsSL https://ployer.nusendra.com/install.sh | sudo bash
```

---

## Configuration

Config is stored in `/opt/ployer/ployer.env`. Edit it and restart Ployer to apply changes:

```bash
nano /opt/ployer/ployer.env
systemctl restart ployer
```

| Variable | Default | Description |
|---|---|---|
| `PLOYER_JWT_SECRET` | *(required)* | Secret key for signing JWT tokens. Use a long random string. |
| `PLOYER_BASE_DOMAIN` | `localhost` | Base domain for auto-generated app subdomains |
| `PLOYER_PUBLIC_URL` | `http://localhost` | Full public URL of the Ployer dashboard |
| `PLOYER_ALLOWED_ORIGINS` | `*` | CORS allowed origins. Lock down in production. |
| `PLOYER_DATABASE_URL` | `sqlite:///var/lib/ployer/ployer.db` | SQLite database path |
| `PLOYER_PORT` | `3001` | Internal API port |
| `PLOYER_CADDY_URL` | `http://localhost:2019` | Caddy Admin API URL |
| `PLOYER_DOCKER_SOCKET` | `/var/run/docker.sock` | Docker socket path |
| `LOG_FORMAT` | *(plain text)* | Set to `json` for structured JSON logging |

---

## Common Commands

```bash
# View live logs
journalctl -u ployer -f

# Stop / start / restart
systemctl stop ployer
systemctl start ployer
systemctl restart ployer

# Reset a locked-out password
ployer reset-password --email you@example.com --password newpassword123
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
 Caddy (80/443)          ← TLS termination, automatic SSL
    │
    ▼
 Ployer (3001)           ← Rust/Axum API + SvelteKit frontend (single binary)
    │
    ├── SQLite            ← Persistent database (/var/lib/ployer/)
    ├── Docker socket     ← Container management
    └── Caddy Admin API   ← Dynamic reverse proxy routes
```

- **Backend** — Rust (Axum), SQLite via sqlx, bollard for Docker
- **Frontend** — SvelteKit (static build, embedded in the binary)
- **Proxy** — Caddy 2 (automatic HTTPS, dynamic routing)
- **Database** — SQLite with WAL mode (no separate database server needed)
- **Process manager** — systemd

### Accessing Ployer over your LAN

In addition to your configured domain, the Ployer dashboard is reachable over plain HTTP on any host that resolves to the server — e.g. `http://192.168.x.x` or `http://hostname.local`. This is handy for local-network use when your domain isn't reachable (or you don't have one yet).

TLS is not used on this path because Let's Encrypt can't issue certs for private IPs or `.local` names; keep the public domain for anything sensitive.

---

## Troubleshooting

**Dashboard not loading after install**
```bash
journalctl -u ployer -f
journalctl -u caddy -f
```

**SSL certificate not issued**
- Make sure your domain's DNS A record points to the server before running the installer
- Ports 80 and 443 must be open (`ufw allow 80 && ufw allow 443`)

**Cannot connect to Docker**
```bash
ls -la /var/run/docker.sock   # should exist
systemctl status docker        # should be active
```

**Cloning a private repo fails with `authentication required but no callback set`**

This means no deploy key is registered for the application, so Ployer has no credentials to authenticate the SSH clone. Deploy keys are per-application — generating one for a different app does not help.

1. Open the application in Ployer and **generate a deploy key** (Settings → Deploy key).
2. Copy the public key and add it to your repository on GitHub: **Settings → Deploy keys → Add deploy key**.
3. Redeploy.

The deploy key must exist in Ployer **and** be added to the repository before the clone runs.

**Database errors on startup**
```bash
ls -la /var/lib/ployer/       # check permissions
chown -R root:root /var/lib/ployer
systemctl restart ployer
```

---

## License

MIT — see [LICENSE](LICENSE)
