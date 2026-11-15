#!/usr/bin/env bash
# Build and start the Elyra production stack.
#
# The script is intentionally small so it can run both manually and from the
# GitHub Actions deploy workflow. It never stores secrets: everything comes
# from infrastructure/deployment/.env.prod (see .env.prod.example).
set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
DEPLOY_DIR="$ROOT_DIR/infrastructure/deployment"
COMPOSE_FILE="$DEPLOY_DIR/docker-compose.prod.yml"
ENV_FILE="${ELYRA_ENV_FILE:-$DEPLOY_DIR/.env.prod}"

usage() {
  cat <<'EOF'
Usage: ./infrastructure/deployment/deploy.sh [options]

Options:
  --tls        Enable the Caddy HTTPS profile (ports 80/443).
  --no-build   Reuse existing images instead of rebuilding the API and website.
  --backup     Dump PostgreSQL to backups/ before deploying.
  --check      Validate the Compose configuration and exit.
  -h, --help   Show this help.

Environment:
  ELYRA_ENV_FILE   Path to the production env file.
                   Defaults to infrastructure/deployment/.env.prod.
  ELYRA_ENABLE_TLS Set to 1 to enable Caddy without passing --tls.
EOF
}

read_env() {
  local key="$1"
  [[ -f "$ENV_FILE" ]] || return 0
  sed -n "s/^[[:space:]]*${key}[[:space:]]*=[[:space:]]*//p" "$ENV_FILE" \
    | tail -n 1 \
    | sed 's/[[:space:]]*$//'
}

TLS="${ELYRA_ENABLE_TLS:-$(read_env ELYRA_ENABLE_TLS)}"
TLS="${TLS:-0}"
DO_BUILD=1
DO_BACKUP=0
CHECK_ONLY=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --tls) TLS=1 ;;
    --no-build) DO_BUILD=0 ;;
    --backup) DO_BACKUP=1 ;;
    --check) CHECK_ONLY=1 ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 64 ;;
  esac
  shift
done

if ! command -v docker >/dev/null 2>&1; then
  echo "docker is not installed or not on PATH." >&2
  exit 69
fi
if ! docker compose version >/dev/null 2>&1; then
  echo "The Docker Compose plugin is required (docker compose)." >&2
  exit 69
fi
if [[ ! -f "$ENV_FILE" ]]; then
  echo "Missing production env file: $ENV_FILE" >&2
  echo "Create it with:" >&2
  echo "  cp $DEPLOY_DIR/.env.prod.example $ENV_FILE" >&2
  echo "then replace the placeholder secrets." >&2
  exit 78
fi

PROFILE_ARGS=()
if [[ "$TLS" == "1" ]]; then
  PROFILE_ARGS=(--profile tls)
fi

compose() {
  docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" "${PROFILE_ARGS[@]}" "$@"
}

echo "Validating Compose configuration…"
compose config --quiet

if [[ "$CHECK_ONLY" == "1" ]]; then
  echo "Compose configuration is valid."
  exit 0
fi

if [[ "$DO_BACKUP" == "1" ]]; then
  if compose ps --status running --services 2>/dev/null | grep -qx postgres; then
    mkdir -p "$ROOT_DIR/backups"
    backup="$ROOT_DIR/backups/elyra-$(date +%Y%m%d-%H%M%S).sql.gz"
    echo "Backing up PostgreSQL to $backup…"
    compose exec -T postgres sh -c 'pg_dump -U "$POSTGRES_USER" "$POSTGRES_DB"' | gzip > "$backup"
    echo "Backup written."
  else
    echo "PostgreSQL is not running; skipping the pre-deploy backup."
  fi
fi

if [[ "$DO_BUILD" == "1" ]]; then
  echo "Building the API and website images…"
  compose build --pull
fi

echo "Starting the production stack…"
compose up -d --remove-orphans

WEB_PORT="$(read_env WEB_PORT)"
WEB_PORT="${WEB_PORT:-8080}"
WEB_BIND="$(read_env WEB_BIND)"
WEB_BIND="${WEB_BIND:-0.0.0.0}"
HEALTH_HOST="$WEB_BIND"
if [[ "$HEALTH_HOST" == "0.0.0.0" || -z "$HEALTH_HOST" ]]; then
  HEALTH_HOST="127.0.0.1"
fi

echo "Waiting for the API health check…"
if command -v curl >/dev/null 2>&1; then
  for _ in $(seq 1 30); do
    if curl -fsS "http://${HEALTH_HOST}:${WEB_PORT}/health" >/dev/null 2>&1; then
      echo "API is healthy."
      break
    fi
    sleep 2
  done

  if ! curl -fsS "http://${HEALTH_HOST}:${WEB_PORT}/health" >/dev/null 2>&1; then
    echo "The API did not become healthy in time. Recent logs:" >&2
    compose logs --tail=80 api >&2 || true
    exit 1
  fi
else
  echo "curl is not installed; skipping the HTTP health check."
  compose ps
fi

DOMAIN="$(read_env ELYRA_DOMAIN)"
if [[ "$TLS" == "1" && -n "$DOMAIN" ]]; then
  echo "Website: https://${DOMAIN}"
  echo "API:     https://${DOMAIN}/api"
else
  echo "Website: http://${HEALTH_HOST}:${WEB_PORT}"
  echo "API:     http://${HEALTH_HOST}:${WEB_PORT}/api"
fi
echo "Deployment complete."
