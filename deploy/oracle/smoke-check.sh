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

: "${ALBION_PUBLIC_URL:?ALBION_PUBLIC_URL must be set}"

curl --fail --silent --show-error --output /dev/null "$ALBION_PUBLIC_URL/healthz"
curl --fail --silent --show-error --output /dev/null "$ALBION_PUBLIC_URL/readyz"
curl --fail --silent --show-error "$ALBION_PUBLIC_URL/version"
printf '\nAlbion HTTP probes passed. Complete the two-manager join/reconnect check from the desktop clients.\n'
