# Oracle deployment preparation

Project Albion can run as a single-career authoritative server on an Oracle
Cloud VM. This is the first deployment target; it is intentionally private and
supports the current two-manager career model rather than public matchmaking.

## Build the standalone server

On a Linux build machine, from `src-tauri`:

```bash
cargo build --release -p albion_server
```

Copy `target/release/albion_server` to the VM. Keep the SQLite career file on a
persistent disk, for example `/srv/openfootballmanager/career.db`.

## Environment

Create `/etc/openfootballmanager/albion.env` and keep it readable only by the
service account:

```text
ALBION_BIND=127.0.0.1:38421
ALBION_JOIN_SECRET=replace-with-a-long-private-code
ALBION_SAVE=/srv/openfootballmanager/career.db
# Optional when a browser/web shell or a hosted Tauri origin calls the API.
ALBION_CORS_ORIGINS=https://play.example.com,tauri://localhost
```

Bind to loopback when Caddy/Nginx terminates HTTPS. Use
`ALBION_BIND=0.0.0.0:38421` only when the VM is intentionally exposing the
service directly through its firewall.

## systemd service

```ini
[Unit]
Description=OpenFoot Manager Albion authoritative server
After=network-online.target
Wants=network-online.target

[Service]
User=ofm
Group=ofm
WorkingDirectory=/srv/openfootballmanager
EnvironmentFile=/etc/openfootballmanager/albion.env
ExecStart=/srv/openfootballmanager/albion_server
Restart=on-failure
RestartSec=3
NoNewPrivileges=true
PrivateTmp=true

[Install]
WantedBy=multi-user.target
```

After installing the unit:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now openfootballmanager-albion
curl -i http://127.0.0.1:38421/healthz
curl -i http://127.0.0.1:38421/readyz
```

Both probes should return HTTP `204`. The WebSocket `/ws` endpoint must be
proxied by the TLS reverse proxy with upgrade headers preserved.

## First Oracle smoke test

1. Upload one known-good career SQLite file.
2. Start the service and verify `/healthz`, `/readyz` and `/version`.
3. Point the Thai and English desktop clients at the public HTTPS URL.
4. Join with both manager slots, pass Ready, reconnect once, and restart the
   service to verify the persisted revision and claims.
5. Schedule a backup of `career.db` before replacing the file.

The hosted server is not yet an account service. Treat the join secret and
career URL as private until account authentication and multi-career tenancy are
implemented.
