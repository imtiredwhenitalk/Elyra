#!/usr/bin/env bash
set -Eeuo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
COMPOSE_FILE="$ROOT_DIR/infrastructure/docker/docker-compose.yaml"
ENV_FILE="$ROOT_DIR/infrastructure/docker/.env"

compose() {
  if [[ -f "$ENV_FILE" ]]; then
    docker compose --env-file "$ENV_FILE" -f "$COMPOSE_FILE" "$@"
  else
    docker compose -f "$COMPOSE_FILE" "$@"
  fi
}

usage() {
  cat <<'EOF'
Usage: ./infrastructure/build/program.sh <command>

Commands:
  up       Start PostgreSQL, Redis and Mailpit
  down     Stop and remove development containers
  logs     Follow service logs
  status   Show service status
  check    Validate all Compose files
  backup   Dump the development PostgreSQL database
EOF
}

command="${1:-}"
case "$command" in
  up) compose --profile dev up -d ;;
  down) compose --profile dev down ;;
  logs) compose --profile dev logs -f ;;
  status) compose --profile dev ps ;;
  check)
    compose config --quiet
    docker compose -f "$ROOT_DIR/infrastructure/docker/docker-build.yaml" config --quiet
    POSTGRES_PASSWORD="${POSTGRES_PASSWORD:-validation-only}" REDIS_PASSWORD="${REDIS_PASSWORD:-validation-only}" \
      docker compose -f "$ROOT_DIR/infrastructure/deployment/docker-compose.prod.yml" config --quiet
    echo "Compose configuration is valid."
    ;;
  backup)
    mkdir -p "$ROOT_DIR/backups"
    compose exec -T postgres sh -c 'pg_dump -U "$POSTGRES_USER" "$POSTGRES_DB"' > "$ROOT_DIR/backups/elyra-$(date +%Y%m%d-%H%M%S).sql"
    echo "Database backup written to backups/."
    ;;
  *) usage; exit 64 ;;
esac
