# Wildcard & Multitenant Domains

Ployer can serve an entire wildcard domain — `*.yourdomain.com` — from a single
app, over HTTPS. This turns one deployment into a multitenant host: every
subdomain (`acme.yourdomain.com`, `123.yourdomain.com`, `demo.yourdomain.com`, …)
routes to the same container, and your app decides what to render per hostname.

This guide covers what it does, how to set it up, and how it works under the
hood.

> Moving the **dashboard itself** onto your own domain is a separate, simpler
> flow — see [dashboard-domain.md](dashboard-domain.md). It reuses the same
> Cloudflare token configured below, and creates that domain's `A` record for
> you. App domains still need their records added by hand (below).

---

## Why wildcard domains are their own feature

A normal custom domain is a single hostname: one DNS record, one Let's Encrypt
certificate issued over the standard HTTP-01 challenge. That challenge validates
one exact hostname at a time, so it can't cover `*.yourdomain.com`.

Wildcard certificates can **only** be issued via the **DNS-01** challenge, where
the ACME client proves control of the domain by creating a `_acme-challenge` TXT
record. That requires programmatic access to your DNS provider. Ployer uses
**Cloudflare** for this: you give it a scoped API token, and Caddy creates and
cleans up the TXT record automatically.

There's also a rate-limit angle. Ployer serves IP-based installs over shared
wildcard-DNS services like `nip.io` and `sslip.io`. Those share a single
registered domain across the whole internet, so per-host Let's Encrypt issuance
hits rate limits fast. Ployer deliberately keeps `nip.io`/`sslip.io` subdomains
on plain HTTP and reserves DNS-01 HTTPS for real domains you own.

| | Single hostname | Wildcard (`*.domain`) |
|---|---|---|
| ACME challenge | HTTP-01 | **DNS-01 (needs DNS API)** |
| DNS records | one A record | `A domain` + `A *.domain` |
| Certificate | per hostname | one cert covers all subdomains |
| Good for | a marketing site | multitenant apps, per-customer subdomains |

---

## Setup

You need a real domain whose DNS is on **Cloudflare**.

### 1. Create a Cloudflare API token

Cloudflare dashboard → **My Profile → API Tokens → Create Token** → use the
**Edit zone DNS** template.

- **Permissions:** `Zone` → `DNS` → `Edit`
- **Zone Resources:** Include → Specific zone → `yourdomain.com`

Copy the token. It only needs DNS edit rights on that one zone, and you can
revoke it at any time.

### 2. Add DNS records

In Cloudflare, both records **DNS only (grey cloud)** so Caddy terminates TLS:

| Type | Name | Content |
|------|------|---------|
| A | `yourdomain.com` | `<your server IP>` |
| A | `*.yourdomain.com` | `<your server IP>` |

### 3. Save the token in Ployer

**Settings → Wildcard Domains (HTTPS)** → paste the token → **Save**. You should
see “✓ Token configured”.

If the running Caddy doesn't have the Cloudflare DNS plugin, the page shows a
warning with a one-line command to install a plugin-enabled Caddy build. Run it
once on the server:

```bash
sudo CF_API_TOKEN=<token> bash -c 'curl -fsSL https://ployer.nusendra.com/install.sh | bash'
```

The installer keeps your existing config and swaps in a Caddy build that includes
`github.com/caddy-dns/cloudflare`.

### 4. Add the wildcard domain to your app

Your app → **Domains** → enter the bare hostname `yourdomain.com` (no `https://`,
no `*.`), tick **Wildcard**, and **Add Domain**. An existing domain can be
switched with the **Enable Wildcard** button.

### 5. Point your app at the domain and redeploy

If your app maps subdomains to tenants, set its tenant-domain environment
variable (whatever your app reads) to `yourdomain.com`, then **redeploy**. The
redeploy writes the HTTPS wildcard route and Caddy issues the certificate over
DNS-01.

Within a few seconds:

```
https://yourdomain.com        → your app
https://anything.yourdomain.com → your app (same container)
```

---

## How it works

### Caddy routes live in `apps.caddy`

Ployer keeps app routes in `/opt/ployer/apps.caddy`, imported by the main
`Caddyfile`. Each route is written as an upsertable block with a marker:

```caddy
# ployer-route: yourdomain.com
*.yourdomain.com, yourdomain.com {
    tls {
        dns cloudflare <token>
    }
    reverse_proxy localhost:<app-host-port>
}
```

For plain HTTP routes (nip.io) the block instead uses an `http://` prefix and no
`tls` directive.

### Ports are refreshed on every deploy

App containers are published on **ephemeral host ports** that change on every
deploy. That means a route written once would point at a dead port after the next
redeploy and return 502. Ployer solves this by **re-persisting every one of an
app's domains on each deploy** with the freshly-resolved host port. The
`# ployer-route:` marker makes each block replaceable regardless of its shape, so
the upsert is clean.

### The token

The Cloudflare token is stored in Ployer's settings database (never returned by
the API — the UI only reports whether one is set) and is written **literally**
into the `apps.caddy` block. Writing it literally means a `caddy reload` applies
it with no service restart and no dependency on an environment file. On startup
the token is seeded from the database, falling back to the `CF_API_TOKEN`
environment variable for install-time configuration.

> **Note:** because the token is embedded in `apps.caddy`, it is stored in
> plaintext on the server (same trust level as `/opt/ployer/ployer.env`). Use a
> minimally-scoped token (`Zone:DNS:Edit` on one zone) and rotate it if needed.

### TLS mode selection

For each domain, Ployer picks the TLS mode automatically:

- `*.nip.io` / `*.sslip.io` → plain HTTP (avoids Let's Encrypt rate limits)
- any other domain, with a Cloudflare token configured → HTTPS via DNS-01
- any other domain, no token → plain HTTP

---

## Troubleshooting

**App subdomain redirects to the Ployer login page.**
No route exists for that host, so Caddy falls through to the dashboard catch-all.
Confirm the app's block is present in `/opt/ployer/apps.caddy` and that you've
redeployed since adding the domain.

**502 after a redeploy.**
A stale route pointing at an old ephemeral port. Redeploy — the route is
rewritten with the current port. (Fixed for good by the per-deploy refresh.)

**`405 Method Not Allowed` when toggling wildcard or removing a domain.**
The domain was stored as a full URL (e.g. `https://yourdomain.com`) and its
slashes broke the REST path. Enter domains as bare hostnames; Ployer now
normalizes input on add (strips scheme, path, port, leading `*.`, trailing dot).

**Settings still warns "Caddy is missing the Cloudflare DNS plugin".**
The running Caddy is a stock build. Run the installer with `CF_API_TOKEN` set to
pull a plugin build, then restart Caddy.

**Certificate isn't issued.**
Check that `*.yourdomain.com` and `yourdomain.com` both resolve to the server
(`dig +short yourdomain.com`), that DNS is grey-clouded (DNS only) in Cloudflare,
and that the token has `Zone:DNS:Edit` on the zone. Watch the Caddy logs:
`sudo journalctl -u caddy -f`.

---

## Verifying

```bash
# DNS
dig +short yourdomain.com 123.yourdomain.com

# HTTPS + cert (tls=0 means the certificate verified)
curl -o /dev/null -w 'http=%{http_code} tls=%{ssl_verify_result}\n' https://123.yourdomain.com/

# Issued certificates on the server
sudo ls /root/.local/share/caddy/certificates/*/wildcard_.yourdomain.com/
```
