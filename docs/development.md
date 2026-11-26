# Development

This page explains how to build, test, and release Elyra from source. It assumes
you are working in the repository root.

## Prerequisites

| Tool | Version | Needed for |
| --- | --- | --- |
| Node.js | 20 or newer | Desktop package scripts, JavaScript checks, Tauri CLI |
| npm | Bundled with Node | Installing desktop dependencies |
| Rust | Stable toolchain | Desktop core, server, crypto, launcher |
| Cargo | Bundled with Rust | Workspace builds and tests |
| Tauri 2 prerequisites | See below | Desktop build on Windows |
| Docker | Engine 24+ with Compose plugin | Server development and deployment |
| PostgreSQL | 16 (or Docker) | Running the server locally |
| Git | Any recent version | Source control |

### Windows Tauri prerequisites

Install:

- Microsoft C++ Build Tools with the **Desktop development with C++** workload.
- WebView2 Runtime (included in current Windows 10/11).
- Rust stable for the `x86_64-pc-windows-msvc` target.

Verify:

```powershell
rustc --version
cargo --version
node --version
npm.cmd --version
```

### Linux server prerequisites

For server-only work, Rust stable and PostgreSQL are enough. Docker is
recommended because the development stack provides PostgreSQL, Redis, and
Mailpit.

## Get the source

```powershell
git clone https://github.com/imtiredwhenitalk/Elyra.git
cd Elyra
```

The repository is a Cargo workspace with four members:

- `packages/crypto`
- `server`
- `apps/launcher`
- `apps/desktop/src-tauri`

The desktop frontend is not a workspace member; it is static files.

## Install desktop dependencies

```powershell
npm.cmd install --prefix apps/desktop
```

The desktop package has only the Tauri CLI as a dev dependency and
`@tauri-apps/api` as a runtime dependency. There is no bundler, React, or Vite.

## Common commands

From the repository root:

| Command | Purpose |
| --- | --- |
| `npm.cmd run check` | Run `node --check` on the desktop entry point |
| `npm.cmd run build` | Build the Windows MSI and NSIS bundles |
| `npm.cmd run tauri dev` | Start the desktop app in development mode |
| `npm.cmd run server:check` | `cargo check -p elyra-server` |
| `npm.cmd run crypto:test` | `cargo test -p elyra-crypto` |
| `npm.cmd run launcher:build` | Build the release launcher |
| `npm.cmd run release:desktop` | Alias for the desktop bundle build |
| `npm.cmd run release:all` | Check JS, build desktop, build launcher |

From `apps/desktop`:

```powershell
npm.cmd run check
npm.cmd run tauri dev
npm.cmd run build
```

### JavaScript validation

`npm.cmd run check` only checks `src/main.js`. For a complete syntax pass over
every desktop and website script, run:

```powershell
Get-ChildItem apps\desktop\src,web -Recurse -Filter *.js |
  ForEach-Object { node --check $_.FullName }
```

The desktop frontend is loaded directly from `apps/desktop/src` via
`frontendDist`. Changes to HTML, CSS, or JS appear after a reload; no bundler
watch step is required.

### Rust validation

```powershell
cargo test -p elyra-crypto -p elyra-server -p elyra-launcher
cargo check -p tauri-app
```

The CI workflow runs the tests on Linux and builds the desktop app on Windows.
A full local desktop build requires the Windows Tauri prerequisites.

### Server validation

```powershell
cargo check -p elyra-server
cargo test -p elyra-server
```

### Automated tests

The server test suite has pure and database-backed layers across three locations:

| Layer | Location | Without PostgreSQL |
| --- | --- | --- |
| Pure unit tests | `server/src/auth.rs`, `server/src/mailer.rs`, `server/src/api.rs` | Run normally |
| API integration tests | `server/src/api.rs` (`#[cfg(test)]`) | Database cases skip with a message |
| Schema integration tests | `server/tests/database_integration.rs` | Skips with a message |

The database-backed cases cover registration/login, bearer-token security, sync
cursor round-trips, notification privacy and read ownership, disabled-relay
fail-closed behavior, schema idempotency, email uniqueness, cursor upserts, and
cascade deletes. No test sends real SMTP traffic.

A plain `cargo test -p elyra-server` therefore works anywhere. To run the full
suite, point the tests at a disposable database:

```sh
ELYRA_TEST_DATABASE_URL=postgres://elyra:elyra@127.0.0.1:5432/elyra_test \
  cargo test -p elyra-server
```

PowerShell equivalent:

```powershell
$env:ELYRA_TEST_DATABASE_URL = "postgres://elyra:elyra@127.0.0.1:5432/elyra_test"
cargo test -p elyra-server
```

`ELYRA_TEST_DATABASE_URL` is an explicit opt-in: a bad URL or failed migration
then fails the test run. `DATABASE_URL` is used as a fallback only when it is
reachable; otherwise the database cases print a skip message. The `Rust` CI
workflow already runs `cargo test -p elyra-crypto -p elyra-server -p
elyra-launcher` on Linux, so pure tests are enforced automatically. A CI service
container can set `ELYRA_TEST_DATABASE_URL` to enforce the database cases too;
the workflow file does not need to change to pick up new tests.

### Testing procedures

Local server smoke test after `cargo run -p elyra-server` (see
[Run the server locally](#run-the-server-locally)):

```sh
curl -fsS http://127.0.0.1:8787/health
curl -sS -X POST http://127.0.0.1:8787/v1/auth/register \
  -H 'Content-Type: application/json' \
  -d '{"email":"dev@example.test","password":"local-dev-passphrase","display_name":"Dev"}'
# Use the returned access_token as: -H "Authorization: Bearer <token>"
```

For production deployments, follow the
[production verification checklist](deploy/VERIFY.md) instead: it includes
pre-deploy checks, HTTPS verification, restart persistence, backup/restore
drill, and sign-off.

### Formatting and linting

```powershell
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
```

`cargo fmt` is expected before committing Rust changes. Clippy is not enforced
by CI in 0.4.0, but it is recommended.

## Run the desktop app

### Desktop mode

```powershell
npm.cmd run tauri dev
```

This starts the Tauri window with the Rust core. SQLite, keychain, IMAP, SMTP,
cloud calls, and crypto commands are live.

### Browser preview

Open `apps/desktop/src/index.html` in a browser, or serve the directory:

```powershell
npx --yes serve apps/desktop/src
```

`services/tauri.js` detects the missing Tauri runtime and falls back to
`localStorage` for mailbox data and settings. The preview is useful for layout
work but does not exercise Rust, SQLite, keychain, IMAP, SMTP, or crypto. Some
actions intentionally show a preview notice.

## Run the server locally

### With Docker

```sh
cp infrastructure/docker/.env.example infrastructure/docker/.env
./infrastructure/build/program.sh check
./infrastructure/build/program.sh up
```

This starts PostgreSQL, Redis, and Mailpit. Mailpit's web UI is at
`http://localhost:8025`.

Stop with:

```sh
./infrastructure/build/program.sh down
```

### Without Docker

Start PostgreSQL 16 yourself, then:

```sh
cp server/.env.example server/.env
# edit DATABASE_URL and ELYRA_JWT_SECRET
cargo run -p elyra-server
```

The server applies its schema on startup and listens on
`ELYRA_SERVER_ADDR` (default `127.0.0.1:8787`).

### Point the desktop at the local server

In the app: **Settings → Elyra Cloud → Server URL**, enter
`http://127.0.0.1:8787`, save, then register or sign in. Alternatively set
`ELYRA_API_URL=http://127.0.0.1:8787` before launching.

## Repository conventions

### Rust

- Keep privileged work in Rust. Do not move mailbox, secret, or network logic
  into the webview.
- Use parameterized SQL. Never format user input into a query string.
- Return `Result<T, String>` from Tauri commands and surface actionable errors.
- Use `#[tauri::command(async)]` for blocking network or long-running work so it
  runs off the main thread.
- Keep secrets out of logs, errors, and serialized webview payloads.
- Match the existing module layout and naming.

### JavaScript

- Use the existing `invoke` wrapper in `services/tauri.js`.
- Escape user or provider content with `escapeHtml` before inserting HTML.
- Keep the UI functional in browser preview where practical, but do not fake
  backend security results.
- Prefer small functions and the existing state/render pattern.
- Use the inline SVG sprite (`icon(name)`) instead of image icons.

### Documentation

- Update `docs/` when behavior changes.
- Add an entry to `changes.md` with the `[200 OK]` marker used by the project.
- Update `task.md` for roadmap status.
- Keep `README.md` and `ai.md` consistent with reality.

### Architecture changes

The project rules in [`ai.md`](../ai.md) require explicit permission before
changing the architecture. Adding a feature inside the existing boundaries is
allowed; adding a service, database, framework, or trust boundary is not.
Record additions in `changes.md`.

## Adding a feature

A typical vertical slice:

1. **Data** — add a column or table in the relevant Rust module with an
   idempotent migration (`ensure_column` or `CREATE TABLE IF NOT EXISTS`).
2. **Domain** — implement the query or mutation in Rust with validation.
3. **Command** — add `#[tauri::command]` and register it in `lib.rs`.
4. **Bridge** — expose it through `services/tauri.js` if the UI uses a wrapper.
5. **UI** — add the control, state, rendering, and error handling.
6. **Docs** — update the command reference, user guide, and architecture if
   needed.
7. **Validation** — run JavaScript checks, Rust checks/tests, and a manual smoke
   test.
8. **Log** — add a `changes.md` entry and update `task.md`.

### Adding a Tauri command

```rust
#[tauri::command]
pub fn example_command(app: AppHandle, value: String) -> Result<String, String> {
    // validate, access SQLite/keychain/network, return a safe result
    Ok(value)
}
```

Register it:

```rust
.invoke_handler(tauri::generate_handler![
    // ...
    module::example_command,
])
```

Document it in
[`docs/reference/tauri-commands.md`](reference/tauri-commands.md).

### Adding a server route

Add the route in `server/src/api.rs`, validate every input, use the existing
`ApiError` variants, and document the payload and error codes in
[Server and API](server.md).

## Release process

### 1. Update versions

Update the version in:

- `Cargo.toml` → `[workspace.package] version`
- `apps/desktop/package.json`
- `package.json`
- `apps/desktop/src-tauri/tauri.conf.json`

`release.yml` reads the version from `tauri.conf.json` for the release title and
tag fallback.

### 2. Update the logs

- Add shipped items to `task.md` with `[200 OK]`.
- Add a detailed `changes.md` entry with validation notes.
- Update `README.md`, `docs/`, and `web/` release text.

### 3. Validate

```powershell
npm.cmd run check
cargo test -p elyra-crypto -p elyra-server -p elyra-launcher
cargo check -p tauri-app
npm.cmd run build
```

Also run the JavaScript syntax pass over all `.js` files and a manual smoke test
of the desktop app: onboarding, local mailbox, compose/draft, account add/test,
sync, settings persistence, Security Lab, and diagnostics.

### 4. Tag and push

```powershell
git tag v0.4.0
git push origin main --tags
```

The `Release Windows` workflow builds the MSI, NSIS setup, and launcher, renames
them to stable asset names, writes checksums, and publishes a GitHub Release.
When `TAURI_SIGNING_PRIVATE_KEY` is configured it also signs both installers and
publishes `latest.json` for in-app updates; see
[Desktop updates](desktop-updates.md) for the signing setup and verification
steps.

Expected assets:

- `Elyra-latest-x64.msi`
- `Elyra-latest-x64-setup.exe`
- `Elyra-Launcher-x64.exe`
- `Elyra-latest-x64.msi.sha256`
- `Elyra-latest-x64-setup.exe.sha256`
- `Elyra-latest-x64.msi.sig` and `Elyra-latest-x64-setup.exe.sig` (signed releases)
- `latest.json` (signed releases)
- `Elyra-latest-x64.AppImage`
- `Elyra-latest-x64.deb`
- `Elyra-latest-x64.AppImage.sha256`
- `Elyra-latest-x64.deb.sha256`

Linux packaging, runtime dependencies, source builds, and Flatpak status are in
the [Linux installation guide](linux.md).

### 5. Optional: bake the production API URL

Set the repository variable `ELYRA_DEFAULT_API_URL` to the public API base, for
example `https://mail.example.com/api`, before tagging. The release workflow
passes it to the desktop build so new installs default to your server.

## CI workflows

| Workflow | Trigger | What it does |
| --- | --- | --- |
| `Rust` | Push/PR to `main` | Tests crypto, server, launcher on Linux; installs npm deps and builds the desktop app on Windows |
| `Release Windows` | Tag `v*` or manual | Builds installers and launcher, publishes a GitHub Release |
| `Release Linux` | Tag `v*` or manual | Builds AppImage + .deb with checksums, uploads `elyra-linux-x64`; optional release publishing via `ELYRA_LINUX_PUBLISH_RELEASE` |
| `Deploy GitHub Pages` | Push to `main` or manual | Publishes `web/` to GitHub Pages |
| `Deploy server` | Push to `main` touching server/web/infrastructure, or manual | Opt-in SSH deploy gated by `ELYRA_DEPLOY_ENABLED` |

The deploy workflow runs `git reset --hard origin/main` and
`deploy.sh --backup` on the server. It is disabled unless the repository variable
`ELYRA_DEPLOY_ENABLED` is exactly `true`.

## Dependency notes

- The desktop uses `rusqlite` with the `bundled` feature, so SQLite is compiled
  into the binary. No system SQLite is required.
- `reqwest` and `lettre` use rustls, avoiding an OpenSSL dependency.
- `imap` uses `native-tls` for platform TLS.
- `keyring` is only included for Windows targets with the `windows-native`
  feature.
- `Cargo.lock` may lag behind workspace dependency changes. CI and Docker do not
  use `--locked`, so Cargo resolves and updates the lockfile during the build.
  If you need reproducible builds, regenerate the lockfile with
  `cargo update -w` and commit it.

## Debugging

| Symptom | Approach |
| --- | --- |
| Blank desktop window | Check the Tauri dev console; verify `frontendDist` and `index.html` paths |
| Command not found in webview | Confirm it is registered in `lib.rs` and spelled exactly in `invoke` |
| SQLite error | Check the database path in Diagnostics; close other processes; inspect pragmas |
| Keychain unavailable | Verify the Windows Credential Manager service and the `windows-native` feature |
| IMAP login fails | Use an app password, verify host/port/security, check provider access settings |
| Cloud calls fail | Check `ELYRA_API_URL`, server URL in settings, `/health`, and the bearer token expiry |
| Release build missing assets | Inspect the `Collect release assets` step and bundle output paths |

Useful local checks:

```powershell
Get-ChildItem apps\desktop\src-tauri\target\release\bundle -Recurse -File
Get-Content apps\desktop\src-tauri\tauri.conf.json
```

## Project layout

```text
Elyra/
├── .github/workflows/        CI, release, Pages, server deploy
├── apps/
│   ├── desktop/
│   │   ├── src/              static HTML/CSS/JS frontend
│   │   ├── src-tauri/        Rust desktop core and Tauri config
│   │   ├── services/         sendemail.rs compiled into the desktop crate
│   │   └── package.json
│   └── launcher/             portable installer/updater CLI
├── packages/
│   ├── crypto/               Rust AES-256-GCM primitives
│   ├── types/                shared TypeScript contracts
│   └── ui/                   framework-neutral UI primitives
├── server/                   Axum API, PostgreSQL schema, auth, relay
├── web/                      static marketing/download site
├── infrastructure/           Docker, deploy script, Caddyfile, Nginx
├── docs/                     this documentation set
├── ai.md                     project rules and intended stack
├── task.md                   feature checklist
└── changes.md                implementation log
```

## Related pages

- [Architecture](architecture.md) — how the pieces fit together.
- [Tauri command reference](reference/tauri-commands.md) — command surface.
- [Configuration reference](reference/configuration.md) — environment variables.
- [Deployment](deployment.md) — production operations.
- [Production verification](deploy/VERIFY.md) — deployment checklist and smoke tests.
- [Troubleshooting](troubleshooting.md) — build and runtime fixes.
