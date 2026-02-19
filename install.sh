#!/usr/bin/env bash
set -euo pipefail

# ─────────────────────────────────────────────
# Ployer — One-line installer
# Usage: curl -fsSL https://raw.githubusercontent.com/nusendra/ployer/main/install.sh | sudo bash
# ─────────────────────────────────────────────

PLOYER_VERSION="main"
PLOYER_DIR="/data/ployer"
GITHUB_REPO="https://github.com/nusendra/ployer.git"
GITHUB_RAW="https://raw.githubusercontent.com/nusendra/ployer/${PLOYER_VERSION}"

# ── Colors ────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

log()     { echo -e "${GREEN}[✓]${NC} $*"; }
info()    { echo -e "${BLUE}[→]${NC} $*"; }
warn()    { echo -e "${YELLOW}[!]${NC} $*"; }
error()   { echo -e "${RED}[✗]${NC} $*" >&2; exit 1; }
step()    { echo -e "\n${BOLD}${BLUE}── $* ${NC}"; }

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
  echo -e "  ${BLUE}https://github.com/nusendra/ployer${NC}"
  echo ""
}

# ── Preflight checks ──────────────────────────

check_root() {
  if [[ $EUID -ne 0 ]]; then
    error "This installer must be run as root. Try: sudo bash install.sh"
  fi
}

check_os() {
  if [[ "$(uname -s)" != "Linux" ]]; then
    error "Ployer installer only supports Linux. Got: $(uname -s)"
  fi

  ARCH=$(uname -m)
  if [[ "$ARCH" != "x86_64" && "$ARCH" != "aarch64" && "$ARCH" != "arm64" ]]; then
    warn "Untested architecture: ${ARCH}. Proceeding anyway."
  fi

  # Detect distro
  if [[ -f /etc/os-release ]]; then
    # shellcheck source=/dev/null
    source /etc/os-release
    OS_ID="${ID:-unknown}"
    OS_VERSION="${VERSION_ID:-unknown}"
    log "Detected OS: ${PRETTY_NAME:-$OS_ID $OS_VERSION}"
  else
    warn "Could not detect OS. Proceeding anyway."
    OS_ID="unknown"
  fi
}

check_requirements() {
  local missing=()
  for cmd in curl git; do
    if ! command -v "$cmd" &>/dev/null; then
      missing+=("$cmd")
    fi
  done

  if [[ ${#missing[@]} -gt 0 ]]; then
    info "Installing missing tools: ${missing[*]}"
    install_packages "${missing[@]}"
  fi
}

# ── Package manager helpers ───────────────────

install_packages() {
  case "$OS_ID" in
    ubuntu|debian|linuxmint|pop)
      apt-get update -qq
      apt-get install -y -qq "$@"
      ;;
    centos|rhel|fedora|rocky|almalinux)
      yum install -y -q "$@" 2>/dev/null || dnf install -y -q "$@"
      ;;
    alpine)
      apk add --no-cache -q "$@"
      ;;
    *)
      warn "Unknown distro '${OS_ID}'. Trying apt-get..."
      apt-get update -qq && apt-get install -y -qq "$@" || true
      ;;
  esac
}

# ── Docker installation ───────────────────────

install_docker() {
  if command -v docker &>/dev/null; then
    DOCKER_VERSION=$(docker --version | awk '{print $3}' | tr -d ',')
    log "Docker already installed: ${DOCKER_VERSION}"
    return
  fi

  step "Installing Docker"
  case "$OS_ID" in
    ubuntu|debian|linuxmint|pop)
      install_packages ca-certificates gnupg lsb-release
      install -m 0755 -d /etc/apt/keyrings
      curl -fsSL https://download.docker.com/linux/${OS_ID}/gpg \
        | gpg --dearmor -o /etc/apt/keyrings/docker.gpg
      chmod a+r /etc/apt/keyrings/docker.gpg
      echo \
        "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] \
        https://download.docker.com/linux/${OS_ID} \
        $(lsb_release -cs) stable" \
        > /etc/apt/sources.list.d/docker.list
      apt-get update -qq
      apt-get install -y -qq docker-ce docker-ce-cli containerd.io docker-compose-plugin
      ;;
    centos|rhel|rocky|almalinux)
      install_packages yum-utils
      yum-config-manager --add-repo https://download.docker.com/linux/centos/docker-ce.repo
      yum install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
      ;;
    fedora)
      dnf install -y dnf-plugins-core
      dnf config-manager --add-repo https://download.docker.com/linux/fedora/docker-ce.repo
      dnf install -y docker-ce docker-ce-cli containerd.io docker-compose-plugin
      ;;
    *)
      info "Trying generic Docker install script..."
      curl -fsSL https://get.docker.com | bash
      ;;
  esac

  systemctl enable docker --now
  log "Docker installed and started"
}

check_docker_running() {
  if ! docker info &>/dev/null; then
    info "Starting Docker daemon..."
    systemctl start docker
    sleep 2
    docker info &>/dev/null || error "Docker is not running. Start it with: systemctl start docker"
  fi
}

# ── Generate secrets ──────────────────────────

generate_secret() {
  if command -v openssl &>/dev/null; then
    openssl rand -hex 32
  else
    cat /dev/urandom | tr -dc 'a-zA-Z0-9' | fold -w 64 | head -n 1
  fi
}

# ── Get server IP ─────────────────────────────

get_server_ip() {
  # Try multiple methods to get the public IP
  local ip=""
  ip=$(curl -fsSL --max-time 3 https://api.ipify.org 2>/dev/null) \
    || ip=$(curl -fsSL --max-time 3 https://ipecho.net/plain 2>/dev/null) \
    || ip=$(hostname -I 2>/dev/null | awk '{print $1}') \
    || ip="127.0.0.1"
  echo "$ip"
}

# ── Interactive prompts ───────────────────────

prompt_domain() {
  local server_ip
  server_ip=$(get_server_ip)

  echo ""
  echo -e "  ${BOLD}Domain / IP for Ployer dashboard:${NC}"
  echo -e "  ${YELLOW}→ Use a domain if you want HTTPS (e.g. ployer.yourdomain.com)${NC}"
  echo -e "  ${YELLOW}→ Use the server IP for quick testing (HTTP only)${NC}"
  echo ""

  # When piped through curl | bash, stdin is not a terminal — skip prompt and use server IP
  if [[ -t 0 ]]; then
    read -rp "  Enter domain or IP [default: ${server_ip}]: " DOMAIN
  else
    warn "Non-interactive mode detected (curl | bash). Using server IP: ${server_ip}"
    warn "To use a custom domain, run: bash install.sh"
    DOMAIN=""
  fi
  DOMAIN="${DOMAIN:-$server_ip}"

  # Determine if it looks like an IP address
  if [[ "$DOMAIN" =~ ^[0-9]+\.[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    USE_HTTPS=false
    PLOYER_PUBLIC_URL="http://${DOMAIN}"
  else
    USE_HTTPS=true
    PLOYER_PUBLIC_URL="https://${DOMAIN}"
  fi

  log "Dashboard will be available at: ${PLOYER_PUBLIC_URL}"
}

# ── Install / upgrade ─────────────────────────

clone_or_update() {
  if [[ -d "${PLOYER_DIR}/.git" ]]; then
    step "Updating existing Ployer installation"
    git -C "$PLOYER_DIR" fetch --quiet origin
    git -C "$PLOYER_DIR" reset --hard "origin/${PLOYER_VERSION}" --quiet
    log "Updated to latest ${PLOYER_VERSION}"
  else
    step "Downloading Ployer"
    mkdir -p "$(dirname "$PLOYER_DIR")"
    git clone --quiet --depth 1 --branch "$PLOYER_VERSION" "$GITHUB_REPO" "$PLOYER_DIR" \
      || git clone --quiet --depth 1 "$GITHUB_REPO" "$PLOYER_DIR"
    log "Cloned to ${PLOYER_DIR}"
  fi
}

write_env() {
  local env_file="${PLOYER_DIR}/.env"

  # Preserve existing JWT secret on upgrades
  local jwt_secret=""
  if [[ -f "$env_file" ]]; then
    jwt_secret=$(grep "^PLOYER_JWT_SECRET=" "$env_file" 2>/dev/null | cut -d= -f2- || true)
  fi
  if [[ -z "$jwt_secret" ]]; then
    jwt_secret=$(generate_secret)
  fi

  cat > "$env_file" <<EOF
# Generated by Ployer installer — $(date -u '+%Y-%m-%d %H:%M UTC')
# Edit this file to change configuration, then run: docker compose up -d

PLOYER_JWT_SECRET=${jwt_secret}
PLOYER_BASE_DOMAIN=${DOMAIN}
PLOYER_PUBLIC_URL=${PLOYER_PUBLIC_URL}
PLOYER_ALLOWED_ORIGINS=${PLOYER_PUBLIC_URL}
EOF

  chmod 600 "$env_file"
  log "Configuration written to ${env_file}"
}

write_caddyfile() {
  local caddyfile="${PLOYER_DIR}/Caddyfile"

  if [[ "$USE_HTTPS" == "true" ]]; then
    cat > "$caddyfile" <<EOF
${DOMAIN} {
    reverse_proxy ployer:3001

    header_up X-Real-IP {remote_host}
    header_up X-Forwarded-Proto {scheme}

    @websocket {
        header Connection *Upgrade*
        header Upgrade    websocket
    }
    reverse_proxy @websocket ployer:3001

    log {
        output file /data/access.log
        format json
    }
}

:2019 {
    bind 0.0.0.0
}
EOF
  else
    # IP-based: HTTP only, no TLS
    cat > "$caddyfile" <<EOF
:80 {
    reverse_proxy ployer:3001

    header_up X-Real-IP {remote_host}
    header_up X-Forwarded-Proto http

    @websocket {
        header Connection *Upgrade*
        header Upgrade    websocket
    }
    reverse_proxy @websocket ployer:3001
}

:2019 {
    bind 0.0.0.0
}
EOF
  fi

  log "Caddyfile written to ${caddyfile}"
}

spinner() {
  local pid=$1
  local msg=$2
  local frames=('⠋' '⠙' '⠹' '⠸' '⠼' '⠴' '⠦' '⠧' '⠇' '⠏')
  local i=0
  while kill -0 "$pid" 2>/dev/null; do
    printf "\r  ${BLUE}%s${NC} %s" "${frames[$((i % ${#frames[@]}))]}" "$msg"
    sleep 0.1
    ((i++))
  done
  printf "\r"
}

start_services() {
  step "Building and starting Ployer"
  cd "$PLOYER_DIR"

  # Build in background and show spinner
  info "Building Docker image (this may take 5-15 minutes on first run)..."
  docker compose build 2>&1 | while IFS= read -r line; do
    # Show stage transitions so user knows progress
    if [[ "$line" =~ ^"#"[0-9]+\ "[".*"]" ]] || [[ "$line" =~ "Step " ]] || [[ "$line" =~ "=>" ]]; then
      stage=$(echo "$line" | sed 's/^#[0-9]* //' | cut -c1-70)
      printf "\r  ${BLUE}→${NC} %-70s\n" "$stage"
    fi
  done || true

  log "Image built"

  docker compose up -d --remove-orphans
  log "Services started"
}

wait_for_healthy() {
  step "Waiting for Ployer to be ready"
  local retries=30
  local url="http://localhost:3001/api/v1/health"

  # Find the mapped port or use the container network
  for i in $(seq 1 $retries); do
    if docker compose exec -T ployer curl -sf http://localhost:3001/api/v1/health &>/dev/null; then
      log "Ployer is healthy"
      return
    fi
    echo -n "."
    sleep 2
  done
  echo ""
  warn "Health check timed out — Ployer may still be starting. Check: docker compose logs ployer"
}

print_success() {
  echo ""
  echo -e "${GREEN}${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo -e "${GREEN}${BOLD}  Ployer installed successfully!${NC}"
  echo -e "${GREEN}${BOLD}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
  echo ""
  echo -e "  ${BOLD}Dashboard:${NC}     ${BLUE}${PLOYER_PUBLIC_URL}${NC}"
  echo -e "  ${BOLD}Install dir:${NC}   ${PLOYER_DIR}"
  echo -e "  ${BOLD}Config:${NC}        ${PLOYER_DIR}/.env"
  echo ""
  echo -e "  ${BOLD}Useful commands:${NC}"
  echo -e "    View logs:    ${YELLOW}docker compose -f ${PLOYER_DIR}/docker-compose.yml logs -f${NC}"
  echo -e "    Stop:         ${YELLOW}docker compose -f ${PLOYER_DIR}/docker-compose.yml down${NC}"
  echo -e "    Upgrade:      ${YELLOW}curl -fsSL https://raw.githubusercontent.com/nusendra/ployer/main/install.sh | sudo bash${NC}"
  echo -e "    Reset pass:   ${YELLOW}docker compose -f ${PLOYER_DIR}/docker-compose.yml exec ployer ./ployer reset-password --email you@example.com --password newpass${NC}"
  echo ""
  if [[ "$USE_HTTPS" == "false" ]]; then
    echo -e "  ${YELLOW}Note: Running over HTTP. Point a domain to this server and re-run the${NC}"
    echo -e "  ${YELLOW}installer to get automatic HTTPS via Let's Encrypt.${NC}"
    echo ""
  fi
}

# ── Main ──────────────────────────────────────

main() {
  banner
  check_root
  check_os
  check_requirements
  install_docker
  check_docker_running
  prompt_domain
  clone_or_update
  write_env
  write_caddyfile
  start_services
  wait_for_healthy
  print_success
}

main "$@"
