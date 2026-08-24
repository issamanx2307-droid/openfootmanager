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

: "${ALBION_SAVE:?ALBION_SAVE must be set}"
: "${ALBION_BACKUP_DIR:?ALBION_BACKUP_DIR must be set}"

if [[ ! -f "$ALBION_SAVE" ]]; then
  echo "Career SQLite file does not exist: $ALBION_SAVE" >&2
  exit 1
fi

install -d -m 750 "$ALBION_BACKUP_DIR"
timestamp="$(date -u +%Y%m%dT%H%M%SZ)"
target="$ALBION_BACKUP_DIR/career-$timestamp.db"
temporary="$target.partial"

# The SQLite online backup API keeps this snapshot consistent while the
# authoritative server is writing. Oracle Linux provides this as `sqlite3`.
if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "sqlite3 is required for consistent live backups" >&2
  exit 1
fi

sqlite3 "$ALBION_SAVE" ".backup '$temporary'"
mv -- "$temporary" "$target"
echo "Created Albion career backup: $target"
