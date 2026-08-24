# Oracle deployment preparation

Project Albion can run as a single-career authoritative server on an Oracle
Cloud VM. This is the first deployment target; it is intentionally private and
supports the current two-manager career model rather than public matchmaking.

## Deployment bundle

The tracked [`deploy/oracle`](../../deploy/oracle) directory is the source of
truth for deployment artifacts:

- `albion.env.example` — environment template;
- `openfootballmanager-albion.service` — systemd unit;
- `Caddyfile` — TLS and WebSocket reverse proxy;
- `backup-career.sh` — timestamped SQLite backup;
- `smoke-check.sh` — public HTTP probe;
- `build-on-oracle.sh` — builds for the VM's actual CPU architecture.

Copy these files to the Oracle VM and replace every example hostname, join
secret and path before enabling the service.

Install the SQLite command-line utility on the VM before scheduling backups;
the backup script uses SQLite's online backup API rather than copying a live
database file directly.

## Build the standalone server

On a Linux build machine, from `src-tauri`:

```bash
cargo build --release -p albion_server
```

For an Oracle ARM VM, build on the VM itself so Cargo targets its native CPU:

```bash
sudo -u ofm /srv/openfootballmanager/source/deploy/oracle/build-on-oracle.sh
```

Alternatively build the binary with the matching Linux target elsewhere. Keep
the SQLite career file on a persistent disk, for example
`/srv/openfootballmanager/data/career.db`.

## Environment

Copy `deploy/oracle/albion.env.example` to `/etc/openfootballmanager/albion.env`
and keep it readable only by the service account. The bundled systemd unit and
Caddyfile use the same paths.

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

Install `deploy/oracle/openfootballmanager-albion.service` as
`/etc/systemd/system/openfootballmanager-albion.service`.

After installing the unit:

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now openfootballmanager-albion
curl -i http://127.0.0.1:38421/healthz
curl -i http://127.0.0.1:38421/readyz
```

Both probes should return HTTP `204`. The WebSocket `/ws` endpoint must be
proxied by the TLS reverse proxy with upgrade headers preserved. Caddy's
`reverse_proxy` does this automatically; install its tracked Caddyfile as
`/etc/caddy/Caddyfile` after replacing the hostname.

Schedule `deploy/oracle/backup-career.sh` with systemd timer or cron, then run
it once and confirm a new timestamped database appears in `ALBION_BACKUP_DIR`.

## First Oracle smoke test

1. Upload one known-good career SQLite file.
2. Start the service and verify `/healthz`, `/readyz` and `/version`.
3. Point the Thai and English desktop clients at the public HTTPS URL.
4. Join with both manager slots, pass Ready, reconnect once, and restart the
   service to verify the persisted revision and claims.
5. Run `smoke-check.sh`, then verify a timestamped backup before replacing the
   career database.

The hosted server is not yet an account service. Treat the join secret and
career URL as private until account authentication and multi-career tenancy are
implemented.
