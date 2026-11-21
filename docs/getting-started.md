# Getting started

This page takes you from a clean Windows machine to a working Elyra inbox.

Elyra 0.3.0 is a Windows desktop application. It runs entirely on your computer
unless you explicitly connect Elyra Cloud. No account is required to try the
local mailbox.

## 1. Check the requirements

| Requirement | Details |
| --- | --- |
| Operating system | Windows 10 or Windows 11, 64-bit |
| Architecture | x64 (`x86_64`) |
| Disk | Roughly 100 MB for the application plus your local mailbox database |
| Network | Optional. Needed for IMAP sync, Elyra Cloud, updates, and the server mail relay |
| Administrator rights | Not required for the per-user NSIS installer. The MSI may request elevation because it is a per-machine package |

macOS bundles are not published. Linux `x86_64` AppImage and `.deb` packages are
built by a separate release workflow; see [Linux](linux.md) for installation and
known limitations. Only the Windows delivery path has launcher support.

## 2. Choose an installer

Download from the [latest release](https://github.com/imtiredwhenitalk/Elyra/releases/latest).

| Asset | Best for | Behavior |
| --- | --- | --- |
| `Elyra-latest-x64.msi` | Managed machines, standard Windows Installer deployment | Per-machine install, may request administrator approval |
| `Elyra-latest-x64-setup.exe` | Normal personal installation | NSIS setup with a per-user install mode |
| `Elyra-Launcher-x64.exe` | Users who want the smallest download and automatic updates | Downloads the MSI with resume/retry, verifies it, installs it, and opens Elyra |
| `Elyra-latest-x64.msi.sha256` | Integrity verification | SHA-256 digest of the MSI |

All release assets use stable file names so links keep working between
versions. The version is visible in **Settings → Diagnostics** and in the title
of the GitHub release.

### Verify a download manually

In PowerShell:

```powershell
Get-FileHash .\Elyra-latest-x64.msi -Algorithm SHA256
Get-Content .\Elyra-latest-x64.msi.sha256
```

The two hashes must match. If they do not, delete the file and download it
again. Do not run an installer whose checksum does not match.

## 3. Install

### Option A: NSIS setup (recommended for most users)

1. Run `Elyra-latest-x64-setup.exe`.
2. Follow the wizard. A per-user installation does not need administrator
   rights.
3. Launch Elyra from the Start menu or the desktop shortcut.

### Option B: MSI package

1. Run `Elyra-latest-x64.msi`.
2. Approve the Windows Installer elevation prompt if it appears.
3. Launch Elyra from the Start menu.

### Option C: portable launcher

The launcher is a single executable. Running it with no arguments downloads
`Elyra-latest-x64.msi` from the latest GitHub release, validates the MSI magic
bytes, verifies the published `.sha256` sidecar, runs Windows Installer, and
opens the app. Downloads are written to a `.part` file, resume after an
interrupted transfer, retry temporary failures, and are only promoted to the
final MSI name after verification. An already-verified cached MSI is reused.

```powershell
.\Elyra-Launcher-x64.exe
```

Useful commands and flags:

| Command | Purpose |
| --- | --- |
| `Elyra-Launcher-x64.exe` | Install or update, then launch |
| `Elyra-Launcher-x64.exe install` | Same as no arguments |
| `Elyra-Launcher-x64.exe update` | Alias for install |
| `Elyra-Launcher-x64.exe download` | Download and verify the installer without installing |
| `Elyra-Launcher-x64.exe launch` | Open an already installed Elyra |
| `Elyra-Launcher-x64.exe info` | Show versions, paths, install record, and download folder |
| `Elyra-Launcher-x64.exe cleanup` | Delete stale downloaded installers |
| `Elyra-Launcher-x64.exe uninstall` | Remove Elyra but keep the local mailbox database |
| `Elyra-Launcher-x64.exe --msi <path>` | Install a local MSI instead of downloading |
| `Elyra-Launcher-x64.exe --url <url>` | Download from a custom HTTPS URL |
| `Elyra-Launcher-x64.exe --sha256 <digest>` | Require a specific checksum |
| `Elyra-Launcher-x64.exe --require-checksum` | Refuse to install when no checksum is published |
| `Elyra-Launcher-x64.exe --retries <n>` | Download attempts, 0–10 (default: 3) |
| `Elyra-Launcher-x64.exe --download-only` | Alias for the download command |
| `Elyra-Launcher-x64.exe --dry-run` | Validate and report without changing anything |
| `Elyra-Launcher-x64.exe --silent` | Quiet Windows Installer mode |
| `Elyra-Launcher-x64.exe --no-launch` | Install without opening the app |
| `Elyra-Launcher-x64.exe --cleanup` | Remove the downloaded MSI after installing |
| `Elyra-Launcher-x64.exe --allow-insecure-http` | Permit a plain HTTP mirror (not recommended) |
| `Elyra-Launcher-x64.exe --version` | Print the launcher version |
| `Elyra-Launcher-x64.exe help` | Show the built-in help |

The launcher writes downloads to `%TEMP%\elyra-installer\` and keeps an MSI
install log there at `install.log`. When installing from a URL, it tries to
fetch `<url>.sha256` automatically before downloading, so a bad mirror is
rejected early. If no checksum is published, it prints the computed hash and
relies on the MSI signature and Windows Installer validation; pass
`--require-checksum` to make a checksum mandatory. Downloads are HTTPS-only by
default; `--allow-insecure-http` is for trusted local or isolated mirrors.

## 4. First launch

On first launch Elyra creates its local database and seeds a small demo
mailbox so the interface is never empty.

The onboarding modal offers three paths:

1. **Create an account** — register against an Elyra server. Enter the server
   URL if it is not the default. The password must be at least 12 characters.
2. **I already have an account** — sign in with an existing Elyra account.
3. **Continue offline** — skip cloud registration. The local mailbox, demo
   messages, themes, drafts, and security lab remain available. IMAP accounts
   can still be added later.

“Continue offline” stores a flag in the webview's `localStorage`
(`elyra-onboarding-skipped`) so the modal does not reappear on every start. You
can connect later from **Settings → Elyra Cloud → Connect account**.

### Server URL

The default for local development is `http://127.0.0.1:8787`. For a deployed
server, enter the public API base **including** `/api` and **without** a trailing
slash, for example:

```text
https://mail.example.com/api
```

The resolution order is documented in
[Configuration reference → Desktop API URL](reference/configuration.md#desktop-api-url).
To verify a self-hosted server after deployment, use the
[production verification checklist](deploy/VERIFY.md).

## 5. A five-minute tour

1. **Inbox** — the seeded demo messages appear in the message list. Use the
   quick filter to switch between All, Unread, Starred, and Attachments.
2. **Open a message** — click a row. The reading pane shows the body, metadata,
   and attachment cards. Star, reply, forward, mark unread, archive, mark spam,
   or move to trash from the toolbar.
3. **Compose** — click **Compose**, choose a sending account if you have one,
   and send. Without a mail account, Elyra tries the environment SMTP settings
   and then the Elyra Cloud relay when you are signed in.
4. **Connect a real mailbox** — open **Settings → Mail accounts → Add account**.
   Pick a provider preset, enter the address and app password, then use
   **Test connection** and **Sync**.
5. **Make it yours** — change the theme, accent, density, signature, and display
   name in **Settings → Appearance** and **Settings → Identity**.

## 6. Connect a mailbox

Elyra 0.3.0 supports IMAP/SMTP accounts with TLS or STARTTLS. OAuth provider
login is the next milestone.

| Provider | Preset | Notes |
| --- | --- | --- |
| Gmail | `imap.gmail.com`, `smtp.gmail.com` | Requires 2-Step Verification and an App Password |
| Outlook / Microsoft 365 | `outlook.office365.com`, `smtp.office365.com` | Use an app password if the tenant requires it |
| iCloud Mail | `imap.mail.me.com`, `smtp.mail.me.com` | Use an app-specific password |
| Yahoo Mail | `imap.mail.yahoo.com`, `smtp.mail.yahoo.com` | Generate an app password |
| Fastmail | `imap.fastmail.com`, `smtp.fastmail.com` | App password recommended |
| Proton Mail | `127.0.0.1` | Requires Proton Mail Bridge running locally |
| Custom | Any host | Enter IMAP and SMTP servers, ports, and security mode |

Passwords are stored in the Windows Credential Manager, not in SQLite. See
[User guide → Mail accounts](user-guide.md#mail-accounts) for the full workflow.

## 7. Where your data lives

| Data | Location |
| --- | --- |
| Local mailbox database | `%LOCALAPPDATA%\com.elyra.desktop\elyra.sqlite3` |
| SQLite journal/WAL files | Same directory |
| Cloud session and IMAP passwords | Windows Credential Manager, service `com.elyra.desktop` |
| UI preferences | SQLite `settings` table, with a `localStorage` fallback in browser preview |
| Installer downloads | `%TEMP%\elyra-installer\` |

The exact database path is always shown in **Settings → Diagnostics**. The same
panel reports the SQLite version, database size, keychain availability, and
mailbox counts.

### Back up

Close Elyra, then copy `elyra.sqlite3` together with any `-wal` and `-shm` files
to a safe location. The database contains message metadata and bodies pulled
from IMAP, plus local settings. It does **not** contain keychain passwords. A
backup without the keychain cannot send or sync until passwords are re-entered.

### Reset

- **Settings → Data → Reset demo mailbox** restores the seeded demo messages.
- Removing the SQLite file starts from a completely fresh state. Close Elyra
  first. This also removes local drafts and cached messages.
- Use **Settings → Elyra Cloud → Sign out** to delete the cloud session from the
  keychain without touching the mailbox.

## 8. Update

- **Launcher users:** run `Elyra-Launcher-x64.exe` again. It downloads the
  newest MSI and installs it over the existing version.
- **Installer users:** download the newest asset from the releases page and run
  it. Windows Installer performs an upgrade.
- **Launcher command:** `Elyra-Launcher-x64.exe update`.

Your local mailbox and keychain entries survive upgrades. Release notes are on
the GitHub releases page and in the in-app **Changes** tab.

## 9. Uninstall

Use Windows **Settings → Apps → Elyra → Uninstall**, or run:

```powershell
.\Elyra-Launcher-x64.exe uninstall
```

Uninstalling removes the application but deliberately keeps the local mailbox
database so an accidental uninstall does not destroy mail. The launcher can do
the full cleanup for you:

| Flag | Purpose |
| --- | --- |
| `--remove-data` (or `--purge`) | Delete `%LOCALAPPDATA%\com.elyra.desktop` and Elyra entries in Windows Credential Manager |
| `--backup-data <folder>` | Copy database files to a folder before `--remove-data` deletes them |
| `--product-code <GUID>` | Remove a specific registered Windows Installer product |
| `--dry-run` | Show what would be removed without changing anything |

```powershell
.\Elyra-Launcher-x64.exe uninstall --backup-data C:\Elyra-backup --remove-data
```

Without `--remove-data`, you can still delete local data manually:

```text
%LOCALAPPDATA%\com.elyra.desktop
```

Credential Manager entries created by `keyring-rs` contain the
`com.elyra.desktop` marker; the launcher removes those automatically only with
`--remove-data`.

## Next steps

- Learn every screen in the [User guide](user-guide.md).
- Understand the security model in [Security](security.md).
- Run your own server with [Deployment](deployment.md).
- If something fails, start with [Troubleshooting](troubleshooting.md).
