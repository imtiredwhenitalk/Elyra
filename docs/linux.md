# Linux

Elyra 0.3.0 ships a 64-bit Linux desktop build from the same Tauri 2 source as
the Windows app. The `Release Linux` workflow (`.github/workflows/linux-release.yml`)
builds an **AppImage** and a **Debian package** for `x86_64` on every `v*` tag and
on manual dispatch, with stable asset names and SHA-256 checksums. An AUR-style
[`PKGBUILD`](../packaging/linux/PKGBUILD) is included in the repository for Arch
Linux. A Flatpak is **not shipped**; the investigation and the remaining work are
[documented below](#flatpak-investigation).

> **Configuration note.** The workflow passes `--bundles appimage deb` to the
> Tauri CLI, so it builds Linux packages even if the shared
> `apps/desktop/src-tauri/tauri.conf.json` still lists only Windows targets. For
> a plain local `npm run build` to emit Linux bundles, `bundle.targets` must
> include `appimage` and `deb` (see [Build from source](#build-from-source)).

## Install

All assets are attached to the [latest GitHub Release](https://github.com/imtiredwhenitalk/Elyra/releases/latest)
when Linux publishing is enabled for the repository; otherwise they are
available as the `elyra-linux-x64` artifact on the workflow run. See
[Release pipeline](#release-pipeline).

| Distribution | Recommended method | Command |
| :--- | :--- | :--- |
| Debian, Ubuntu, Linux Mint, Pop!_OS | `.deb` | `sudo apt install ./Elyra-latest-x64.deb` |
| Any glibc-based x86_64 distribution | AppImage | `chmod +x Elyra-latest-x64.AppImage && ./Elyra-latest-x64.AppImage` |
| Arch, Manjaro, EndeavourOS | PKGBUILD | `makepkg -si` from `packaging/linux` |

### Verify a download

Every published asset has a `<name>.sha256` file written by the release job:

```sh
sha256sum -c Elyra-latest-x64.AppImage.sha256
sha256sum -c Elyra-latest-x64.deb.sha256
```

The checksum file contains `<64 hex digits>  <asset name>`, so `sha256sum -c`
must run in the same directory as the asset. A mismatch means the download is
incomplete or was replaced; do not run it.

### AppImage

```sh
# Download the stable asset and its checksum from the latest release.
curl -fL -O https://github.com/imtiredwhenitalk/Elyra/releases/latest/download/Elyra-latest-x64.AppImage
curl -fL -O https://github.com/imtiredwhenitalk/Elyra/releases/latest/download/Elyra-latest-x64.AppImage.sha256
sha256sum -c Elyra-latest-x64.AppImage.sha256
chmod +x Elyra-latest-x64.AppImage
./Elyra-latest-x64.AppImage
```

The AppImage is portable: it needs no installation and writes its data to the
usual XDG locations. Files placed in a writable directory run with relative
paths resolved against the AppImage itself. If FUSE 2 is unavailable, run it
without a mounted loop device:

```sh
./Elyra-latest-x64.AppImage --appimage-extract-and-run
```

For permanent desktop integration, use the `.deb` package or the PKGBUILD; both
install a `.desktop` entry and a hicolor icon. Optional tools such as
AppImageLauncher can also register a downloaded AppImage.

### Debian and Ubuntu

```sh
sudo apt install ./Elyra-latest-x64.deb
```

`apt` resolves and installs the WebKitGTK 4.1 and GTK 3 runtime dependencies
declared in the package. The binary is installed under `/usr/bin` and the
launcher appears as **Elyra** in the application menu.

### Arch Linux (PKGBUILD)

The repository includes an AUR-style recipe at
[`packaging/linux/PKGBUILD`](../packaging/linux/PKGBUILD):

```sh
git clone https://github.com/imtiredwhenitalk/Elyra.git
cd Elyra/packaging/linux
makepkg -si
```

It downloads the stable AppImage plus its `.sha256` file, verifies the digest in
`prepare()`, installs the AppImage to `/opt/elyra/elyra.AppImage`, creates the
`/usr/bin/elyra` symlink, and installs the desktop entry and 256x256 icon.

The `latest` asset changes with every release, so `sha256sums[0]` is `SKIP` and
the published checksum is fetched at build time. This is verified, but it is
not a pinned trust anchor. For an immutable build or an AUR submission, point
at a versioned asset instead:

```sh
ELYRA_APPIMAGE_URL=https://github.com/imtiredwhenitalk/Elyra/releases/download/v0.3.0/Elyra-0.3.0-x64.AppImage \
  makepkg -si
```

Then move the real digest into `sha256sums[0]`. `ELYRA_ALLOW_UNVERIFIED=1`
skips verification for local experiments only; a missing or malformed checksum
otherwise fails the build. See [`packaging/linux/README.md`](../packaging/linux/README.md)
for details.

## Dependencies

### Build dependencies

On Debian or Ubuntu (Tauri 2 prerequisites, including WebKitGTK **4.1**):

```sh
sudo apt update
sudo apt install -y \
  build-essential pkg-config curl wget file \
  libwebkit2gtk-4.1-dev libjavascriptcoregtk-4.1-dev libsoup-3.0-dev \
  libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
  libxdo-dev libssl-dev dpkg-dev libfuse2
```

On Arch: `sudo pacman -S --needed base-devel webkit2gtk-4.1 libsoup3 gtk3 libappindicator-gtk3 librsvg curl wget file`.

### Runtime dependencies

- The `.deb` declares its WebKitGTK 4.1, GTK 3, and appindicator dependencies;
  `apt` installs them for you.
- An AppImage bundles most libraries but still needs a system WebKitGTK 4.1 /
  GTK 3 stack and glibc from a compatible base system. On Ubuntu 24.04 install
  `libfuse2` to mount AppImages without `--appimage-extract-and-run`.
- The PKGBUILD declares the equivalent Arch runtime dependencies. `fuse2` is
  optional and only needed if you do not use the extract-and-run fallback.
- SQLite is compiled into the binary (`rusqlite` `bundled` feature); no system
  SQLite package is required.

### glibc baseline

The workflow uses `ubuntu-latest`, currently Ubuntu 24.04. Tauri's own guidance
is to build on the oldest base you intend to support: binaries built there
require that runner's glibc version, so an AppImage or `.deb` from this workflow
may report `version 'GLIBC_2.39' not found` on Ubuntu 22.04, Debian 12, or older
distributions. Those users should build from source on their own distribution
(or a 22.04 container). Switching the workflow to an older runner image is an
option if broader binary compatibility becomes a requirement.

## Build from source

Requirements: Node.js 20+, Rust stable, the build dependencies above, and Git.

```sh
git clone https://github.com/imtiredwhenitalk/Elyra.git
cd Elyra
npm ci --prefix apps/desktop
cd apps/desktop
npm run check
npm run build -- --bundles appimage deb
```

The `--bundles` flag overrides `bundle.targets` for this invocation. To make
plain `npm run build` produce Linux packages, add the Linux targets to
`apps/desktop/src-tauri/tauri.conf.json`:

```json
"targets": ["msi", "nsis", "appimage", "deb"]
```

Bundles are written to:

```text
apps/desktop/src-tauri/target/release/bundle/appimage/
apps/desktop/src-tauri/target/release/bundle/deb/
```

Set `ELYRA_DEFAULT_API_URL` before the build to bake a production API base into
the app, the same as the Windows release. Set `TAURI_SIGNING_PRIVATE_KEY` and
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD` only if updater artifacts are enabled; the
Linux workflow disables `createUpdaterArtifacts` when the signing key is absent
so unsigned builds still succeed.

No local Rust toolchain is needed to follow the release pipeline; the Linux
build itself is validated in GitHub Actions.

## Desktop entries and icons

| Asset | Location | Notes |
| :--- | :--- | :--- |
| Launcher | `packaging/linux/elyra.desktop` | `Name=Elyra`, `Exec=/opt/elyra/elyra.AppImage %F`, `Icon=elyra`, `Categories=Office;Email;Network;`, `mailto` handler. |
| Icon | `packaging/linux/elyra.png` | 256x256 PNG copied from `apps/desktop/src-tauri/icons/128x128@2x.png`. |
| Refresh script | `packaging/linux/copy-icon.sh` | Re-copies the icon and fails unless it is 256x256; update the pinned hash in `PKGBUILD` afterwards. |

Freedesktop hicolor lookup expects a 256x256 raster icon at
`usr/share/icons/hicolor/256x256/apps/elyra.png`, which the PKGBUILD installs
so the desktop entry's `Icon=elyra` resolves. The Tauri bundle config also
lists `32x32.png`, `128x128.png`, and `128x128@2x.png`; the last one is the
256x256 source used here. Validate a modified launcher with
`desktop-file-validate packaging/linux/elyra.desktop`. If the running window
does not group with its launcher, compare `xprop WM_CLASS` with
`StartupWMClass=Elyra` and adjust the desktop file.

## Release pipeline

| File | Purpose |
| :--- | :--- |
| `Elyra-latest-x64.AppImage` | Stable AppImage used by the docs and PKGBUILD. |
| `Elyra-latest-x64.deb` | Stable Debian package. |
| `Elyra-<version>-x64.AppImage` | Versioned copy of the same build. |
| `Elyra-<version>-x64.deb` | Versioned copy of the same build. |
| `<name>.sha256` | SHA-256 digest of each asset above. |

`<version>` is read from `apps/desktop/src-tauri/tauri.conf.json`, so the asset
names always match the app version. The build job uploads all of them to the
`elyra-linux-x64` GitHub Actions artifact with read-only `contents` permission.
To attach them to the same GitHub Release as the Windows installers, set the
repository variable `ELYRA_LINUX_PUBLISH_RELEASE` to `true`; the opt-in publish
job then gets `contents: write` and merges the files into the release for the
tag (or the version-derived tag on manual dispatch). Until then, the release
page only contains the Windows assets and the Linux files are available from
the workflow run.

## Flatpak investigation

**Decision: not shipped in 0.3.0.** AppImage and `.deb` cover the current Linux
requests with the existing Tauri bundler and release workflow; adding Flatpak
would require a separate sandbox build, a runtime dependency decision, and a
Flathub submission owner. This is a packaging gap, not a runtime limitation.

What would be needed:

1. A `com.github.imtiredwhenitalk.Elyra.yml` manifest using `flatpak-builder`
   with the GNOME or Freedesktop SDK/runtime that provides WebKitGTK 4.1 and
   libsoup 3 (for example `org.gnome.Sdk//46`), plus the Rust toolchain
   extension or a tarball of the source with vendored Cargo dependencies.
2. The app must be built inside the sandbox from source. A Tauri AppImage is
   not a good Flatpak source: its FUSE-based runtime, bundled loader, and
   `Exec` path do not fit the Flatpak sandbox. Use the Tauri source build with
   the Linux targets instead.
3. Sandbox permissions for IMAP, SMTP, and HTTPS network access; the
   `org.freedesktop.secrets` session bus for keychain-backed secrets; XDG data
   and config directories for the local SQLite database; desktop portals,
   Wayland/X11, and IPC for the webview and window integration.
4. A `.desktop` entry named after the Flatpak application ID with
   `Exec=elyra`, the 256x256 hicolor icon, a metainfo/appdata XML file with
   content rating, summaries, and screenshots, and a license file.
5. A release job that runs `flatpak-builder`, exports a single `.flatpak`
   bundle, and smoke-tests `flatpak run`, then a Flathub submission and review.
   Verify the WebKitGTK 4.1 runtime stays available across supported runtime
   branches before committing to the format.

Revisit this when a maintainer owns the Flathub submission and the CI can build
and test a sandbox build on every tag.

## Updates and uninstall

- **AppImage:** download the new asset, re-check the `.sha256` file, replace the
  old file, and keep your existing XDG data directory. Optional AppImage
  updater tools must be configured separately.
- **Debian:** `sudo apt install ./Elyra-latest-x64.deb` upgrades in place;
  `sudo apt remove elyra` uninstalls the package.
- **Arch:** `pacman -Suy elyra` after the PKGBUILD is updated;
  `sudo pacman -Rns elyra` removes the package.
- Local mail, settings, and scan results are not deleted by uninstalling. Open
  **Settings → Diagnostics** in the app to see the exact data path; remove it
  manually only after backing up anything you need.

## Troubleshooting

| Symptom | Cause / fix |
| :--- | :--- |
| `dlopen(): error loading libfuse.so.2` | Install `libfuse2` (Debian/Ubuntu) or `fuse2` (Arch), or run `./Elyra.AppImage --appimage-extract-and-run`. |
| `version 'GLIBC_2.39' not found` | The binary was built on a newer runner than your distribution. Build from source on an older base (see [glibc baseline](#glibc-baseline)). |
| `pkg-config could not find webkit2gtk-4.1` | Install `libwebkit2gtk-4.1-dev` (Ubuntu 22.04+) / `webkit2gtk-4.1` (Arch). The 4.0 package is a different API and will not work. |
| `Can't detect any appindicator library` | Install `libayatana-appindicator3-dev` (build) and its runtime package, or `libappindicator-gtk3` on Arch. |
| Blank window after launch | Check WebKitGTK and icon packages, then run from a terminal for webview errors. Software rendering may be needed on remote desktops. |
| Elyra is missing from the application menu | Install the `.deb` or PKGBUILD rather than a bare AppImage; run `desktop-file-validate` and check that `elyra.png` is in `hicolor/256x256/apps`. |
| Keychain or secret storage fails on Linux | The current keyring integration is Windows-only in the Cargo features; secrets may fall back to environment variables or be unavailable. Track this as a separate Linux hardening item before relying on cloud sign-in. |
| Checksum mismatch | Re-download the asset and confirm the `.sha256` file came from the same release. Never bypass a real mismatch. |

## Related pages

- [Development](development.md) — prerequisites, commands, and the release process.
- [Troubleshooting](troubleshooting.md) — general application issues.
- [Configuration reference](reference/configuration.md) — environment variables.
- [`packaging/linux/README.md`](../packaging/linux/README.md) — packaging asset details.
