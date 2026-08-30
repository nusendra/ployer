# Dashboard Domain

Ployer installs onto whatever address it can reach the server by. Give it an IP
and it serves the dashboard on `<server-ip>.nip.io` — a free wildcard-DNS
service that resolves any `1.2.3.4.nip.io` name back to `1.2.3.4`, so you get a
working HTTPS URL without owning a domain.

That default is meant to be temporary. **Settings → Dashboard Domain** moves the
dashboard onto a domain or subdomain you own, and — if you've saved a Cloudflare
API token — creates the DNS record for you.

---

## Setup

### 1. (Optional) Save a Cloudflare API token

Skip this if you'd rather add the DNS record by hand.

Cloudflare dashboard → **My Profile → API Tokens → Create Token** → **Edit zone
DNS** template:

- **Permissions:** `Zone` → `DNS` → `Edit`
- **Zone Resources:** Include → Specific zone → `yourdomain.com`

Paste it into **Settings → Wildcard Domains (HTTPS)**. The same token serves both
features: DNS-01 wildcard certificates for app domains, and the `A` records for
this one.

### 2. Check the server IP

**Settings → Dashboard Domain** shows the public IPv4 Ployer detected on
startup. That's the address any DNS record must point at. Override it if the
detected value is wrong — behind NAT, on a floating IP, or on a multi-homed host.

### 3. Set the domain

Enter a bare hostname (`ployer.yourdomain.com` — no `https://`, no `*.`) and hit
**Set Domain**.

With a token configured and **Create the A record in Cloudflare** ticked, Ployer:

1. finds the zone that owns the hostname (longest suffix match, so
   `a.b.example.com` prefers a `b.example.com` zone),
2. creates or corrects an `A` record → your server IP, **DNS only (grey
   cloud)**,
3. rewrites the dashboard's Caddy site block and reloads Caddy.

Without a token it does step 3 and tells you exactly which record to add.

Once DNS resolves, `https://ployer.yourdomain.com` serves the dashboard and Caddy
issues a Let's Encrypt certificate for it.

---

## The nip.io address keeps working

Switching does **not** retire the original `<ip>.nip.io` address. Both hostnames
stay on the dashboard's site block:

```caddy
ployer.yourdomain.com, 3.144.143.144.nip.io {
    reverse_proxy localhost:3001
}
```

A domain that isn't resolving yet — or a typo — can't lock you out of the UI you'd
use to fix it. **Revert** moves the dashboard back to the nip.io address alone;
it leaves Cloudflare records untouched.

---

## How it works

### The base Caddyfile, not apps.caddy

App routes live in `/opt/ployer/apps.caddy` and are rewritten on every deploy
(see [wildcard-domains.md](wildcard-domains.md)). The dashboard's own hostname
lives one level up, in `/opt/ployer/Caddyfile`, which `install.sh` writes. Ployer
regenerates that file from the same template, preserving the `import` of
`apps.caddy`, and saves the previous version as `Caddyfile.bak`.

### Persisted to ployer.env

`PLOYER_BASE_DOMAIN`, `PLOYER_PUBLIC_URL` and `PLOYER_ALLOWED_ORIGINS` in
`/opt/ployer/ployer.env` are rewritten too — every other line, including the JWT
secret and Cloudflare token, is left byte-for-byte intact. This matters on
upgrades: the installer reads `PLOYER_BASE_DOMAIN` back and keeps it, so a
self-update won't drop you onto a re-detected nip.io address.

Caddy picks up the new hostname on reload, so the dashboard is reachable there
immediately. Restarting Ployer (`systemctl restart ployer`) additionally
refreshes the CORS allow-list from the new value.

### Why records are DNS-only

Ployer writes `proxied: false`. Caddy terminates TLS on your server, so routing
the hostname through Cloudflare's orange-cloud proxy would break both the HTTP-01
challenge for this domain and the DNS-01 wildcard flow the same token serves.

---

## Troubleshooting

**"No Cloudflare zone for '…' is visible to this token."**
The token is scoped to a different zone. Either widen it (Zone Resources →
include the right zone) or add the `A` record manually — the Caddy side was still
applied.

**"Could not determine this server's public IP."**
The startup probe couldn't reach `api.ipify.org` / `ifconfig.me` / `ipecho.net`.
Type the IP into **Server public IP** and retry.

**New domain shows the login page over HTTP but not HTTPS.**
Caddy hasn't finished issuing the certificate. Check DNS resolves to the server
(`dig +short ployer.yourdomain.com`) and watch `sudo journalctl -u caddy -f`.

**Certificate isn't issued.**
Confirm the record is **grey cloud** (DNS only) in Cloudflare. An orange-clouded
record proxies port 80 through Cloudflare and the HTTP-01 challenge never reaches
Caddy.

**Locked out after a typo.**
The nip.io address still works — open it and set the domain again. If the base
Caddyfile itself is broken, `sudo cp /opt/ployer/Caddyfile.bak
/opt/ployer/Caddyfile && sudo systemctl reload caddy` restores the previous one.

---

## Verifying

```bash
# DNS
dig +short ployer.yourdomain.com

# HTTPS + cert (tls=0 means the certificate verified)
curl -o /dev/null -w 'http=%{http_code} tls=%{ssl_verify_result}\n' https://ployer.yourdomain.com/

# What Caddy is serving the dashboard on
grep -A2 'reverse_proxy localhost:3001' /opt/ployer/Caddyfile
```
