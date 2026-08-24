#!/usr/bin/env bash
set -euo pipefail

backup_source="${1:?Usage: $0 /path/to/career-backup.db /path/to/restored-career.db}"
restore_target="${2:?Usage: $0 /path/to/career-backup.db /path/to/restored-career.db}"

if ! command -v sqlite3 >/dev/null 2>&1; then
  echo "sqlite3 is required for a restore test" >&2
  exit 1
fi

if [[ ! -r "$backup_source" ]]; then
  echo "Backup is not readable: $backup_source" >&2
  exit 1
fi

if [[ -e "$restore_target" ]]; then
  echo "Restore target already exists: $restore_target" >&2
  exit 1
fi

restore_directory="$(dirname "$restore_target")"
if [[ ! -d "$restore_directory" ]]; then
  echo "Restore target directory does not exist: $restore_directory" >&2
  exit 1
fi

cp -- "$backup_source" "$restore_target"

integrity="$(sqlite3 "$restore_target" "PRAGMA integrity_check;")"
if [[ "$integrity" != "ok" ]]; then
  echo "Restore integrity check failed for $restore_target: $integrity" >&2
  exit 1
fi

echo "Restore test passed: $restore_target"
