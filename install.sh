#!/usr/bin/env bash
set -euo pipefail

# ─────────────────────────────────────────────
# Ployer — One-line installer
# Usage: curl -fsSL https://ployer.nusendra.com/install.sh | sudo bash
# ─────────────────────────────────────────────

PLOYER_REPO="nusendra/ployer"
PLOYER_DIR="/opt/ployer"
PLOYER_DATA_DIR="/var/lib/ployer"
PLOYER_BIN="/usr/local/bin/ployer"
PLOYER_SERVICE="/etc/systemd/system/ployer.service"

# ── Colors ────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

log()   { echo -e "${GREEN}[✓]${NC} $*"; }
info()  { echo -e "${BLUE}[→]${NC} $*"; }
warn()  { echo -e "${YELLOW}[!]${NC} $*"; }
error() { echo -e "${RED}[✗]${NC} $*" >&2; exit 1; }
step()  { echo -e "\n${BOLD}${BLUE}── $* ${NC}"; }

banner() {
  echo -e "${BLUE}${BOLD}"
  cat << 'EOF'
  ____  _
 |  _ \| | ___  _   _  ___ _ __
 | |_) | |/ _ \| | | |/ _ \ '__|
 |  __/| | (_) | |_| |  __/ |
 |_|   |_|\___/ \__, |\___|_|
                |___/
EOF
  echo -e "${NC}"
  echo -e "  ${BOLD}Lightweight self-hosting PaaS${NC}"
  echo -e "  ${BLUE}https://github.com/${PLOYER_REPO}${NC}"
  echo ""
}

# ── Preflight ─────────────────────────────────

check_root() {
  [[ $EUID -eq 0 ]] || error "Run as root: sudo bash install.sh"
}

check_os() {
  [[ "$(uname -s)" == "Linux" ]] || error "Only Linux is supported."

  ARCH=$(uname -m)
  case "$ARCH" in
    x86_64)        BINARY_ARCH="x86_64" ;;
    aarch64|arm64) BINARY_ARCH="arm64" ;;
    *) error "Unsupported architecture: ${ARCH}" ;;
  esac

  if [[ -f /etc/os-release ]]; then
    source /etc/os-release
    OS_ID="${ID:-unknown}"
    log "OS: ${PRETTY_NAME:-$OS_ID}"
  else
    OS_ID="unknown"
  fi
}

install_docker() {
  step "Installing Docker"
  info "Docker not found — installing via official script..."

  # Use Docker's official convenience script (supports Ubuntu, Debian, CentOS, Fedora, etc.)
  # Download to a temp file — do NOT pipe to sh here, because this installer itself is
  # being piped into bash (curl | bash), and a nested pipe would steal stdin mid-script.
  wait_for_apt
  local docker_script
  docker_script=$(mktemp)
  curl -fsSL https://get.docker.com -o "$docker_script" \
    || error "Failed to download Docker install script."
  sh "$docker_script" || error "Docker installation failed. Install manually: https://docs.docker.com/engine/install/"
  rm -f "$docker_script"

  # Enable and start Docker
  systemctl enable docker --now
  log "Docker installed and started"
}

check_docker() {
  if ! command -v docker &>/dev/null; then
    install_docker
  fi

  if ! docker info &>/dev/null; then
    info "Starting Docker..."
    systemctl start docker || error "Docker is installed but could not be started. Try: systemctl start docker"
  fi

  log "Docker: $(docker --version | awk '{print $3}' | tr -d ',')"
}

# ── Package helpers ───────────────────────────

wait_for_apt() {
  local locks=(
    /var/lib/dpkg/lock-frontend
    /var/lib/dpkg/lock
    /var/lib/apt/lists/lock
    /var/cache/apt/archives/lock
  )
  local waited=0
  while fuser "${locks[@]}" &>/dev/null 2>&1; do
    if [[ $waited -eq 0 ]]; then
      info "Waiting for apt lock (unattended-upgrades is running)..."
    fi
    sleep 3
    waited=$((waited + 3))
    if [[ $waited -ge 120 ]]; then
      warn "Apt lock held for 2 minutes. Killing unattended-upgrades..."
      systemctl stop unattended-upgrades 2>/dev/null || true
      kill -9 "$(fuser /var/lib/dpkg/lock-frontend 2>/dev/null)" 2>/dev/null || true
      sleep 2
      break
    fi
  done
  if [[ $waited -gt 0 ]]; then log "Apt lock released"; fi
}

install_packages() {
  case "$OS_ID" in
    ubuntu|debian|linuxmint|pop)
      wait_for_apt
      apt-get update -qq && apt-get install -y -qq "$@" ;;
    centos|rhel|rocky|almalinux)
      yum install -y -q "$@" 2>/dev/null || dnf install -y -q "$@" ;;
    fedora)
      dnf install -y -q "$@" ;;
    alpine)
      apk add --no-cache -q "$@" ;;
    *)
      warn "Unknown distro. Trying apt-get..." && apt-get install -y -qq "$@" || true ;;
  esac
}

# ── Fetch latest release ──────────────────────

get_latest_version() {
  # Extract all tag names, sort by version (handles alpha.9 < alpha.10 < alpha.11),
  # and take the highest — GitHub API does not guarantee chronological ordering.
  #
  # `grep -o` pulls out each "tag_name":"..." pair individually so this works
  # whether the API returns pretty-printed JSON (one field per line) or compact
  # single-line JSON. A plain `grep '"tag_name"' | cut -f4` breaks on compact
  # JSON — the whole array is one line, so cut returns the first release's URL.
  curl -fsSL "https://api.github.com/repos/${PLOYER_REPO}/releases" \
    | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' \
    | cut -d'"' -f4 | sort -V | tail -1
}

download_release() {
  local version="$1"
  local asset="ployer-${version}-ployer-linux-${BINARY_ARCH}.tar.gz"
  local url="https://github.com/${PLOYER_REPO}/releases/download/${version}/${asset}"
  local tmpdir
  tmpdir=$(mktemp -d)

  info "Downloading ${asset}..."
  curl -fsSL --progress-bar "$url" -o "${tmpdir}/${asset}" \
    || error "Failed to download release. Check: https://github.com/${PLOYER_REPO}/releases"

  info "Extracting..."
  tar -xzf "${tmpdir}/${asset}" -C "$tmpdir"

  local extracted="${tmpdir}/ployer-${version}-ployer-linux-${BINARY_ARCH}"

  # Validate the tarball BEFORE touching the installed copy. A release missing
  # the binary or a built frontend would otherwise leave ployer running but
  # serving a 404 dashboard (empty public/). Abort and keep the old version up.
  local missing=""
  [[ -f "${extracted}/ployer" ]]            || missing+=" ployer"
  [[ -f "${extracted}/public/index.html" ]] || missing+=" public/index.html"
  [[ -d "${extracted}/migrations" ]]        || missing+=" migrations"
  if [[ -n "$missing" ]]; then
    rm -rf "$tmpdir"
    warn "Release ${version} is incomplete (missing:${missing})."
    # main() stops ployer before calling us on upgrade — bring the old one back.
    systemctl start ployer 2>/dev/null || true
    error "Aborting update. Existing version left running. Re-cut the release with a built frontend."
  fi

  install -m 755 "${extracted}/ployer" "$PLOYER_BIN"
  log "Binary installed: ${PLOYER_BIN}"

  # Replace frontend wholesale so stale files from prior versions are removed.
  rm -rf "${PLOYER_DIR}/public"
  mkdir -p "${PLOYER_DIR}/public" "${PLOYER_DIR}/migrations"
  cp -r "${extracted}/public/." "${PLOYER_DIR}/public/"
  cp -r "${extracted}/migrations/." "${PLOYER_DIR}/migrations/"

  rm -rf "$tmpdir"
  log "Release ${version} extracted to ${PLOYER_DIR}"
}

# ── Configure ─────────────────────────────────

generate_secret() {
  command -v openssl &>/dev/null \
    && openssl rand -hex 32 \
    || cat /dev/urandom | tr -dc 'a-zA-Z0-9' | fold -w 64 | head -n 1
}

get_server_ip() {
  curl -fsSL --max-time 3 https://api.ipify.org 2>/dev/null \
    || curl -fsSL --max-time 3 https://ipecho.net/plain 2>/dev/null \
    || hostname -I 2>/dev/null | awk '{print $1}' \
    || echo "127.0.0.1"
}

prompt_config() {
  local server_ip
  server_ip=$(get_server_ip)

  # Preserve previously-configured domain on upgrades. Self-update from the
  # dashboard runs this script non-interactively via `curl | bash`, so without
  # this we'd silently overwrite the user's real domain with an auto-detected
  # IP/nip.io on every update.
  local existing_domain=""
  if [[ -f "${PLOYER_DIR}/ployer.env" ]]; then
    existing_domain=$(grep "^PLOYER_BASE_DOMAIN=" "${PLOYER_DIR}/ployer.env" 2>/dev/null | cut -d'=' -f2- || true)
  fi

  echo ""
  echo -e "  ${BOLD}Where will Ployer be accessible?${NC}"
  echo -e "  ${YELLOW}→ Domain (e.g. ployer.yourdomain.com) — gets automatic HTTPS${NC}"
  echo -e "  ${YELLOW}→ IP address — auto-converted to nip.io for free HTTPS + subdomains${NC}"
  echo ""

  local default_domain="${existing_domain:-$server_ip}"

  if [[ -t 0 ]]; then
    read -rp "  Enter domain or IP [default: ${default_domain}]: " DOMAIN
  elif [[ -n "$existing_domain" ]]; then
    info "Non-interactive mode — keeping existing domain: ${existing_domain}"
    DOMAIN="$existing_domain"
  else
    warn "Non-interactive mode (curl | bash). Using server IP: ${server_ip}"
    warn "Re-run 'bash install.sh' to set a custom domain."
    DOMAIN=""
  fi
  DOMAIN="${DOMAIN:-$default_domain}"

  # Convert bare IP to nip.io for working subdomains + free HTTPS
  if [[ "$DOMAIN" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    DOMAIN="${DOMAIN}.nip.io"
    info "IP detected — using ${DOMAIN} for free HTTPS and subdomains"
  fi

  PUBLIC_URL="https://${DOMAIN}"
  log "Dashboard will be at: ${PUBLIC_URL}"
}

# Resolve the Cloudflare API token from the install environment or a previous
# install. Enables HTTPS wildcard certs (DNS-01) for real custom domains.
resolve_cf_token() {
  if [[ -n "${CF_API_TOKEN:-}" ]]; then
    printf '%s' "${CF_API_TOKEN}"
    return
  fi
  if [[ -f "${PLOYER_DIR}/ployer.env" ]]; then
    grep "^CF_API_TOKEN=" "${PLOYER_DIR}/ployer.env" 2>/dev/null | cut -d'=' -f2- || true
  fi
}

write_config() {
  local env_file="${PLOYER_DIR}/ployer.env"

  # Preserve existing JWT secret on upgrades
  local jwt_secret=""
  if [[ -f "$env_file" ]]; then
    jwt_secret=$(grep "^PLOYER_JWT_SECRET=" "$env_file" 2>/dev/null | cut -d'=' -f2- || true)
  fi
  [[ -z "$jwt_secret" ]] && jwt_secret=$(generate_secret)

  # Preserve the Cloudflare token across upgrades (read before rewriting).
  local cf_token
  cf_token=$(resolve_cf_token)

  cat > "$env_file" <<EOF
PLOYER_HOST=0.0.0.0
PLOYER_PORT=3001
PLOYER_BASE_DOMAIN=${DOMAIN}
PLOYER_PUBLIC_URL=${PUBLIC_URL}
PLOYER_ALLOWED_ORIGINS=${PUBLIC_URL}
PLOYER_DATABASE_URL=sqlite://${PLOYER_DATA_DIR}/ployer.db?mode=rwc
PLOYER_JWT_SECRET=${jwt_secret}
PLOYER_TOKEN_EXPIRY_HOURS=24
PLOYER_DOCKER_SOCKET=/var/run/docker.sock
PLOYER_CADDY_URL=http://localhost:2019
PLOYER_CADDYFILE=${PLOYER_DIR}/Caddyfile
FRONTEND_DIR=${PLOYER_DIR}/public
EOF

  # Cloudflare token for HTTPS wildcard custom domains (optional).
  [[ -n "$cf_token" ]] && echo "CF_API_TOKEN=${cf_token}" >> "$env_file"

  chmod 600 "$env_file"
  log "Config written: ${env_file}"
}

# ── Caddy (reverse proxy) ─────────────────────

install_caddy() {
  local caddy_arch="amd64"
  [[ "$BINARY_ARCH" == "arm64" ]] && caddy_arch="arm64"

  # HTTPS wildcard custom domains need the Cloudflare DNS provider compiled in.
  local need_cloudflare=0
  [[ -n "$(resolve_cf_token)" ]] && need_cloudflare=1

  if command -v caddy &>/dev/null; then
    # Already fine unless we need the Cloudflare plugin and it's missing.
    if [[ "$need_cloudflare" == "0" ]] || caddy list-modules 2>/dev/null | grep -q 'dns.providers.cloudflare'; then
      log "Caddy already installed: $(caddy version | head -1)"
      return
    fi
    info "Existing Caddy lacks the Cloudflare DNS plugin — installing a plugin build..."
  fi

  step "Installing Caddy"

  if [[ "$need_cloudflare" == "1" ]]; then
    # Prebuilt binary with the Cloudflare DNS provider — no Go toolchain needed.
    local caddy_url="https://caddyserver.com/api/download?os=linux&arch=${caddy_arch}&p=github.com/caddy-dns/cloudflare"
    info "Downloading Caddy with Cloudflare DNS plugin..."
    curl -fsSL "$caddy_url" -o /usr/local/bin/caddy \
      || error "Failed to download Caddy (cloudflare) from ${caddy_url}"
    chmod +x /usr/local/bin/caddy
    log "Caddy (with Cloudflare DNS plugin) installed"
    return
  fi

  info "Fetching latest Caddy release..."
  local caddy_version
  caddy_version=$(curl -fsSL https://api.github.com/repos/caddyserver/caddy/releases/latest \
    | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' | cut -d'"' -f4)
  [[ -n "$caddy_version" ]] || error "Could not determine latest Caddy version."

  local caddy_url="https://github.com/caddyserver/caddy/releases/download/${caddy_version}/caddy_${caddy_version#v}_linux_${caddy_arch}.tar.gz"
  info "Downloading Caddy ${caddy_version}..."
  curl -fsSL "$caddy_url" | tar -xz -C /usr/local/bin caddy \
    || error "Failed to download Caddy from ${caddy_url}"
  chmod +x /usr/local/bin/caddy

  log "Caddy ${caddy_version} installed"
}

write_caddyfile() {
  local caddyfile="${PLOYER_DIR}/Caddyfile"

  # Create apps.caddy only if it does not exist yet. Existing app routes must
  # survive self-updates (which re-run this installer); truncating here would
  # wipe every deployed app's route and fall back to the dashboard catch-all.
  [ -f "${PLOYER_DIR}/apps.caddy" ] || : > "${PLOYER_DIR}/apps.caddy"

  cat > "$caddyfile" <<EOF
{
    # Disable Caddy's catch-all HTTP→HTTPS redirect so app subdomains
    # served with http:// prefix are not silently upgraded to HTTPS.
    auto_https disable_redirects
}

${DOMAIN} {
    reverse_proxy localhost:3001
}

# LAN / direct-IP access (HTTP only — no cert possible for private IPs).
# Catches any host not matched above (e.g. http://192.168.x.x, http://hostname.local).
http:// {
    reverse_proxy localhost:3001
}

import ${PLOYER_DIR}/apps.caddy
EOF

  log "Caddyfile written: ${caddyfile}"
}

# ── Systemd services ──────────────────────────

write_ployer_service() {
  cat > "$PLOYER_SERVICE" <<EOF
[Unit]
Description=Ployer — Self-hosting PaaS
After=network.target docker.service
Requires=docker.service

[Service]
Type=simple
User=root
WorkingDirectory=${PLOYER_DIR}
EnvironmentFile=${PLOYER_DIR}/ployer.env
ExecStart=${PLOYER_BIN}
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=ployer

[Install]
WantedBy=multi-user.target
EOF
  log "Systemd service: ${PLOYER_SERVICE}"
}

write_caddy_service() {
  # If caddy was installed via package manager it already has a service
  if systemctl list-unit-files caddy.service &>/dev/null 2>&1 | grep -q caddy; then
    mkdir -p /etc/caddy
    cp "${PLOYER_DIR}/Caddyfile" /etc/caddy/Caddyfile
  else
    cat > /etc/systemd/system/caddy.service <<EOF
[Unit]
Description=Caddy
After=network.target

[Service]
Type=simple
User=root
# Loads CF_API_TOKEN (if set) so Caddy can resolve {env.CF_API_TOKEN} for
# Cloudflare DNS-01 wildcard certs. Optional — the leading '-' tolerates absence.
EnvironmentFile=-${PLOYER_DIR}/ployer.env
ExecStart=/usr/local/bin/caddy run --config ${PLOYER_DIR}/Caddyfile
ExecReload=/usr/local/bin/caddy reload --config ${PLOYER_DIR}/Caddyfile
Restart=on-failure
RestartSec=5
StandardOutput=journal
StandardError=journal
SyslogIdentifier=caddy

[Install]
WantedBy=multi-user.target
EOF
  fi
  log "Caddy service configured"
}

open_firewall() {
  if command -v ufw &>/dev/null && ufw status | grep -q "Status: active"; then
    ufw allow 80/tcp  >/dev/null 2>&1 || true
    ufw allow 443/tcp >/dev/null 2>&1 || true
    log "Firewall: ports 80 and 443 opened"
  fi
}

start_services() {
  step "Starting services"
  open_firewall
  systemctl daemon-reload

  systemctl enable caddy --now
  systemctl reload caddy 2>/dev/null || systemctl restart caddy
  log "Caddy started"

  systemctl enable ployer --now
  log "Ployer started"
}

# ── Health check ──────────────────────────────

wait_healthy() {
  step "Waiting for Ployer to be ready"
  local retries=20
  for i in $(seq 1 $retries); do
    if curl -sf http://localhost:3001/api/v1/health &>/dev/null; then
      log "Ployer is healthy"
      return
    fi
    echo -n "."
    sleep 2
  done
  echo ""
  warn "Health check timed out. Check logs: journalctl -u ployer -f"
}

# ── Success ───────────────────────────────────

print_success() {
  echo ""
  echo -e "${GREEN}${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${GREEN}${BOLD}  Ployer installed successfully!${NC}"
  echo -e "${GREEN}${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""
  echo -e "  ${BOLD}Dashboard:${NC}  ${BLUE}${PUBLIC_URL}${NC}"
  echo -e "  ${BOLD}Config:${NC}     ${PLOYER_DIR}/ployer.env"
  echo -e "  ${BOLD}Data:${NC}       ${PLOYER_DATA_DIR}/"
  echo ""
  echo -e "  ${BOLD}Commands:${NC}"
  echo -e "    Logs:     ${YELLOW}journalctl -u ployer -f${NC}"
  echo -e "    Stop:     ${YELLOW}systemctl stop ployer${NC}"
  echo -e "    Restart:  ${YELLOW}systemctl restart ployer${NC}"
  echo -e "    Upgrade:  ${YELLOW}curl -fsSL https://ployer.nusendra.com/install.sh | sudo bash${NC}"
  echo ""
  if [[ "$DOMAIN" =~ \.nip\.io$ ]]; then
    echo -e "  ${YELLOW}Tip: For a permanent URL, point your own domain to this server and re-run the installer.${NC}"
    echo ""
  fi
}

# ── Main ──────────────────────────────────────

main() {
  banner
  check_root
  check_os
  check_docker

  step "Fetching latest release"
  PLOYER_VERSION=$(get_latest_version)
  [[ -n "$PLOYER_VERSION" ]] || error "Could not determine latest version. Check your internet connection."
  log "Version: ${PLOYER_VERSION}"

  # Upgrade detection
  if [[ -f "$PLOYER_BIN" ]]; then
    CURRENT_VERSION=$(ployer --version 2>/dev/null | awk '{print $2}' || echo "unknown")
    if [[ "$CURRENT_VERSION" == "$PLOYER_VERSION" ]]; then
      log "Already on latest version (${PLOYER_VERSION}). Nothing to do."
      exit 0
    fi
    info "Upgrading ${CURRENT_VERSION} → ${PLOYER_VERSION}"
    systemctl stop ployer 2>/dev/null || true
  fi

  mkdir -p "$PLOYER_DIR" "$PLOYER_DATA_DIR"

  step "Downloading Ployer ${PLOYER_VERSION}"
  download_release "$PLOYER_VERSION"

  prompt_config
  write_config

  step "Setting up Caddy (reverse proxy)"
  install_caddy
  write_caddyfile
  write_caddy_service

  step "Setting up systemd service"
  write_ployer_service

  start_services
  wait_healthy
  print_success
}

main "$@"
