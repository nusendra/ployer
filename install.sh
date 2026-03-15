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
  wait_for_apt
  curl -fsSL https://get.docker.com | sh \
    || error "Docker installation failed. Install manually: https://docs.docker.com/engine/install/"

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
  [[ $waited -gt 0 ]] && log "Apt lock released"
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
  curl -fsSL "https://api.github.com/repos/${PLOYER_REPO}/releases" \
    | grep '"tag_name"' | head -1 | cut -d'"' -f4
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

  install -m 755 "${extracted}/ployer" "$PLOYER_BIN"
  log "Binary installed: ${PLOYER_BIN}"

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

  echo ""
  echo -e "  ${BOLD}Where will Ployer be accessible?${NC}"
  echo -e "  ${YELLOW}→ Domain (e.g. ployer.yourdomain.com) — gets automatic HTTPS${NC}"
  echo -e "  ${YELLOW}→ IP address — auto-converted to sslip.io for free HTTPS + subdomains${NC}"
  echo ""

  if [[ -t 0 ]]; then
    read -rp "  Enter domain or IP [default: ${server_ip}]: " DOMAIN
  else
    warn "Non-interactive mode (curl | bash). Using server IP: ${server_ip}"
    warn "Re-run 'bash install.sh' to set a custom domain."
    DOMAIN=""
  fi
  DOMAIN="${DOMAIN:-$server_ip}"

  # Convert bare IP to sslip.io for working subdomains + free HTTPS
  if [[ "$DOMAIN" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    local sslip_domain
    sslip_domain=$(echo "$DOMAIN" | tr '.' '-')
    DOMAIN="${sslip_domain}.sslip.io"
    info "IP detected — using ${DOMAIN} for free HTTPS and subdomains"
  fi

  PUBLIC_URL="https://${DOMAIN}"
  log "Dashboard will be at: ${PUBLIC_URL}"
}

write_config() {
  local env_file="${PLOYER_DIR}/ployer.env"

  # Preserve existing JWT secret on upgrades
  local jwt_secret=""
  if [[ -f "$env_file" ]]; then
    jwt_secret=$(grep "^PLOYER_JWT_SECRET=" "$env_file" 2>/dev/null | cut -d'=' -f2- || true)
  fi
  [[ -z "$jwt_secret" ]] && jwt_secret=$(generate_secret)

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
FRONTEND_DIR=${PLOYER_DIR}/public
EOF

  chmod 600 "$env_file"
  log "Config written: ${env_file}"
}

# ── Caddy (reverse proxy) ─────────────────────

install_caddy() {
  if command -v caddy &>/dev/null; then
    log "Caddy already installed: $(caddy version | head -1)"
    return
  fi

  step "Installing Caddy"

  local caddy_arch="amd64"
  [[ "$BINARY_ARCH" == "arm64" ]] && caddy_arch="arm64"

  info "Fetching latest Caddy release..."
  local caddy_version
  caddy_version=$(curl -fsSL https://api.github.com/repos/caddyserver/caddy/releases/latest \
    | grep '"tag_name"' | cut -d'"' -f4)
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

  cat > "$caddyfile" <<EOF
${DOMAIN} {
    reverse_proxy localhost:3001
}

:2019 {
    bind 127.0.0.1
}
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
  if [[ "$DOMAIN" =~ \.sslip\.io$ ]]; then
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
