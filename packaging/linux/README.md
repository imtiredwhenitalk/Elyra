# Linux packaging assets

This directory holds the distribution assets for the Linux x86_64 desktop build.
The release pipeline is `.github/workflows/linux-release.yml`; the full
installation and build guide is [`docs/en/linux.md`](../../docs/en/linux.md).

| File | Purpose |
| --- | --- |
| `PKGBUILD` | AUR-style recipe that downloads the published AppImage, verifies its SHA-256 file, and installs it into `/opt/elyra`. |
| `elyra.desktop` | Freedesktop launcher used by the PKGBUILD (`Name`, `Exec`, `Icon`, categories, `mailto` handling). |
| `elyra.png` | 256x256 hicolor icon copied from `apps/desktop/src-tauri/icons/128x128@2x.png`. |
| `copy-icon.sh` | Regenerates `elyra.png` from the Tauri icon set after an icon change. |

## Using the PKGBUILD locally

The default source is the stable asset published by the release workflow:

```sh
cd packaging/linux
makepkg -si
```

Use a specific, immutable release instead of the rolling `latest` asset:

```sh
ELYRA_APPIMAGE_URL=https://github.com/imtiredwhenitalk/Elyra/releases/download/v0.5.0/Elyra-0.5.0-x64.AppImage \
  makepkg -si
```

`prepare()` deletes nothing automatically: it reads `<asset-url>.sha256`, checks
that it contains a 64-character hex digest, and compares it with the downloaded
AppImage. A missing or malformed checksum fails the build unless
`ELYRA_ALLOW_UNVERIFIED=1` is set. That escape hatch is deliberately noisy and
should only be used for local testing while a release is being prepared. AUR
submissions should pin `sha256sums[0]` to a versioned asset rather than rely on
the `latest` redirect (see the comment at the top of `PKGBUILD`).

## Refreshing the icon

```sh
sh packaging/linux/copy-icon.sh
```

The script fails unless the copied PNG is 256x256. Update the pinned hash for
`elyra.png` in `PKGBUILD` when the icon changes. Desktop environments look up
`Icon=elyra` in `hicolor/256x256/apps`, which is where the package installs it.

## Desktop entry checks

```sh
desktop-file-validate packaging/linux/elyra.desktop
```

If the window does not group with its launcher, check the real class with
`xprop WM_CLASS` and update `StartupWMClass` in `elyra.desktop`.

## Release prerequisites

The workflow must publish `Elyra-latest-x64.AppImage` and
`Elyra-latest-x64.AppImage.sha256` to the GitHub Release before this PKGBUILD
works out of the box; this is enabled as of 0.5.0. See `docs/en/linux.md` for the
publishing switch (`ELYRA_LINUX_PUBLISH_RELEASE`) and for the alternative
installation methods.
