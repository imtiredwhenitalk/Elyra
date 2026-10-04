# Elyra infrastructure

Docker provides the production website, authenticated Rust API, and shared services:

- Nginx serves the public website and proxies `/api` to the Rust API.
- The Rust API owns account metadata and sync cursors.
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

Set `POSTGRES_PASSWORD`, `REDIS_PASSWORD`, and `ELYRA_JWT_SECRET` in the deployment environment, then run:

```sh
docker compose -f infrastructure/deployment/docker-compose.prod.yml up -d --build
```

Only the Nginx website port is published. The API, PostgreSQL, and Redis stay
on the private `elyra` network. The website is available on `WEB_PORT`
(default `8080`).

## Release artifacts

Build the standalone Windows desktop installer:

```powershell
npm.cmd run release:desktop
```

MSI and NSIS artifacts are written to
`apps/desktop/src-tauri/target/release/bundle/`. Build the optional Windows
loader with:

```powershell
$env:ELYRA_DOWNLOAD_URL = "https://your-domain.example/downloads/Elyra.msi"
npm.cmd run launcher:build
```

The loader is written to `target/release/elyra-launcher.exe`, downloads the MSI
over HTTPS, and starts `msiexec`.
