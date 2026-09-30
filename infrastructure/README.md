# Elyra infrastructure

The repository does not contain a server implementation yet, so Docker currently provides the shared services needed by the future API and sync layers:

- PostgreSQL 16 for server data.
- Redis 7 for sessions, queues, and realtime coordination.
- Mailpit in the development profile for safe local SMTP testing.

## Development

1. Copy `infrastructure/docker/.env.example` to `infrastructure/docker/.env`.
2. Run `./infrastructure/build/program.sh check`.
3. Run `./infrastructure/build/program.sh up`.
4. Open Mailpit at `http://localhost:8025`.
5. Stop services with `./infrastructure/build/program.sh down`.

The desktop application remains a native Tauri build. Use `./infrastructure/build/site.sh check` and `./infrastructure/build/site.sh build` for it.

## Production

Set `POSTGRES_PASSWORD` and `REDIS_PASSWORD` in the deployment environment, then run:

```sh
docker compose -f infrastructure/deployment/docker-compose.prod.yml up -d
```

Production services are intentionally not published to host ports. Put the future API/reverse proxy on the `elyra` network and expose only the API entry point.
