#!/usr/bin/env bash
set -euo pipefail

ENV_FILE="${ALBION_ENV_FILE:-/etc/openfootballmanager/albion.env}"

if [[ ! -r "$ENV_FILE" ]]; then
  echo "Cannot read Albion environment file: $ENV_FILE" >&2
  exit 1
fi

set -a
# shellcheck disable=SC1090
source "$ENV_FILE"
set +a

required=(ALBION_BIND ALBION_JOIN_SECRET ALBION_SAVE ALBION_PUBLIC_URL ALBION_BACKUP_DIR)
for name in "${required[@]}"; do
  value="${!name:-}"
  if [[ -z "$value" || "$value" == *"replace-with-"* || "$value" == *"example.com"* ]]; then
    echo "$name is missing or still uses an example value" >&2
    exit 1
  fi
done

if [[ ! -f "$ALBION_SAVE" ]]; then
  echo "Career SQLite file does not exist: $ALBION_SAVE" >&2
  exit 1
fi

if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "sqlite3 is required for online career backups" >&2
  exit 1
fi

if ! command -v caddy >/dev/null 2>&1; then
  echo "caddy is required for the documented HTTPS/WebSocket proxy" >&2
  exit 1
fi

install -d -m 750 "$ALBION_BACKUP_DIR"
printf 'Oracle preflight passed for %s\n' "$ALBION_PUBLIC_URL"
