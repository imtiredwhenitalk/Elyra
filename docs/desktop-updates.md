# Desktop updates (Windows)

Elyra's Windows desktop app can check for, verify, download and install its own
updates. The updater is deliberately conservative: it only trusts artifacts
signed with Elyra's Ed25519 update key, it never installs an unverified
installer, and a failed update leaves the currently installed version in place.

## What ships today

- Supported update target: **Windows x64**.
- Check endpoint:
  `https://github.com/imtiredwhenitalk/Elyra/releases/latest/download/latest.json`
- Accepted installer keys in `latest.json`: `windows-x86_64-msi`,
  `windows-x86_64-nsis`, and the generic `windows-x86_64` fallback.
- The desktop config declares `msi` and `nsis` as the Windows bundle targets;
  the Linux release workflow explicitly builds `appimage` and `deb` with
  `--bundles appimage deb`. Linux bundles are not consumed by the Windows
  updater.

The application version shown in **Settings → Updates** comes from
`apps/desktop/src-tauri/tauri.conf.json`.

## User controls

Open **Settings → Updates** to:

- see the installed and latest announced versions, release date and notes;
- run an on-demand check with **Check now**;
- choose whether Elyra checks the signed feed at startup;
- choose whether Elyra shows a notification when an automatic check finds an
  update;
- watch download and verification progress;
- install the signed update with **Install update**.

Both preferences are stored in the local SQLite `settings` table under
`updates_auto_check` and `updates_notify_available`. They are enabled by
default. Turning off automatic checks does not disable manual checks.

On Windows, Tauri exits Elyra when the signed installer starts. The installer
uses passive UI by default, and the default configuration installs into the
current user context for NSIS.

## Trust model

The updater public key is a compile-time Rust constant in
`apps/desktop/src-tauri/src/updates.rs`, registered with
`tauri_plugin_updater::Builder::new().pubkey(UPDATE_PUBLIC_KEY)`. The plugin
configuration in `tauri.conf.json` keeps an empty `pubkey` placeholder because
Tauri's plugin config schema requires the field; the Rust value always replaces
it at runtime.

Public key (safe to share):

```text
dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDQyREE1QzczREYzNDdCQkEKUldTNmV6VGZjMXphUXV3aEJQcU9IQVArb1NZTEdaQ0FVOUYrQ056LzJVODYydHh0aVdZR0ZudEEK
```

The matching private key must **never** be committed. Anyone with that key can
publish updates that existing installations will trust.

Artifact flow:

1. The release workflow signs the copied MSI and NSIS installers with the
   private key and `--app-version <version>`.
2. Each `.sig` file contains the raw base64 signature text, not a URL or path.
3. `latest.json` embeds those signature strings and absolute HTTPS URLs for the
   release assets.
4. On download, Tauri verifies the installer bytes against the embedded public
   key. `requireSignedVersion` is enabled, so the signed version must also match
   the version announced by `latest.json`. `download_and_install_update` also
   calls `install`, which verifies the bytes again before invoking the platform
   installer.
5. If the manifest, signature, download or verification fails, Elyra reports an
   error and does not replace any application files.

Tauri's updater cannot be configured to skip signature verification. Do not
add `dangerousInsecureTransportProtocol`, `dangerousAcceptInvalidCerts` or
`dangerousAcceptInvalidHostnames` to the updater configuration.

## Publishing a signed release

1. Generate a key pair once (skip if the repository already has one):

   ```powershell
   npm.cmd run tauri -- signer generate -- -w .tauri/elyra.key
   ```

   Keep the `.pub` output handy and store the private key outside the repo.

2. In the GitHub repository, create these Actions secrets:

   - `TAURI_SIGNING_PRIVATE_KEY` — the **full contents** of the private key
     file, including its `untrusted comment:` line. Literal `\n` sequences are
     accepted and normalized by the workflow.
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — only if the private key was created
     with a password.

   These secrets are required for automatic updates. They are never written to
   disk by the workflow.

3. Update the app version in `apps/desktop/src-tauri/tauri.conf.json` and the
   desktop `package.json` together, then push a `v*` tag or run the
   **Release Windows** workflow manually.

4. The **Release Windows** workflow builds the installers, signs them, writes
   SHA-256 checksums, generates `latest.json`, and publishes all `dist/Elyra-*`
   assets on the GitHub release.

5. Confirm that the release contains `Elyra-latest-x64.msi`,
   `Elyra-latest-x64-setup.exe`, both `.sig` files, their `.sha256` files and
   `latest.json`. Then open **Settings → Updates** in an installed build and
   choose **Check now**.

If `TAURI_SIGNING_PRIVATE_KEY` is absent, the workflow still builds and
publishes normal installers, but it warns and skips signing and `latest.json`.
That release is safe to distribute, but existing installs will keep reporting
the last signed version until a signed release is published.

## Key rotation

Treat the private key as permanent infrastructure. Replacing it means every
already-installed copy of Elyra still trusts the old public key, so plan a
transition release before rotating. If the private key is lost or suspected
compromised, rotate it, publish a new signed release, and require users to
install that bridge release manually.

## Troubleshooting

- **“Could not check for updates”** — confirm GitHub Releases is reachable,
  that `latest.json` exists on the latest release, and that its JSON is valid.
- **“Update download or signature verification failed”** — check that the
  `.sig` content in `latest.json` matches the exact installer URL, that the
  version was signed with `--app-version`, and that the public key in
  `updates.rs` matches the private key used in CI.
- **No update is offered** — the announced version must be greater than the
  running version, and the manifest must contain a complete entry for the
  current Windows target.
- **Browser preview** — the web preview cannot download installers. It
  simulates an up-to-date result and stores update preferences in
  `localStorage` so the Settings UI can still be exercised.
