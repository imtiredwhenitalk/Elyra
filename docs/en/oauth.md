> 🌐 **English** · [Українська](../ua/oauth.md)

# OAuth 2.0 for mail providers

Elyra can sign in to Gmail and Outlook/Microsoft 365 with OAuth 2.0
**Authorization Code + PKCE** instead of an IMAP app password. The flow is a
native desktop flow: Elyra opens the system browser, listens on a temporary
loopback port, and never asks you to paste a client secret into the app.

> **Scope.** OAuth authorizes **IMAP sync** through `AUTHENTICATE XOAUTH2`.
> Outgoing SMTP still needs an app password or the Elyra relay. See
> [Limitations](#limitations).

<a id="how-it-works"></a>
## How it works

1. Elyra generates a random `state` and a PKCE `code_verifier` and derives a
   S256 `code_challenge` (RFC 7636).
2. It binds `127.0.0.1` on an ephemeral port and opens the provider's consent
   screen in the default browser.
3. The provider redirects to `http://127.0.0.1:<port>/callback` with `code` and
   `state`. Elyra rejects a callback whose `state` does not match.
4. The code is exchanged at the token endpoint with the original verifier. No
   client secret is used because Elyra is a public native client.
5. The account row is created or updated, and the token set is written to the
   OS keychain under `oauth-account-<account id>`. SQLite stores no tokens.
6. Access tokens are refreshed automatically before they expire. Refreshing
   with a revoked or expired refresh token fails with a clear reconnect hint.

<a id="configuration"></a>
## Configuration

Client IDs are public identifiers, but Elyra does not ship placeholder IDs.
Set one or both before building or starting the desktop app:

| Variable | When | Purpose |
| :--- | :--- | :--- |
| `ELYRA_GOOGLE_CLIENT_ID` | Build time or runtime | Gmail / Google OAuth client |
| `ELYRA_MICROSOFT_CLIENT_ID` | Build time or runtime | Outlook / Microsoft identity client |

Build-time values are embedded with `option_env!`; a runtime value overrides
them. The account modal reports a provider as available only when a client ID
**and a working OS keychain** are present. In the browser preview and on
non-Windows builds the OAuth buttons are hidden — the flow requires the Rust
core and the Windows Credential Manager backend.

Register a **Desktop app / Native** client with the provider and allow the
loopback redirect URI `http://127.0.0.1:<port>/callback`. Google and Microsoft
both permit a dynamically chosen loopback port for native clients. Requested
scopes:

| Provider | Scopes |
| :--- | :--- |
| Google | `openid email profile https://mail.google.com/` |
| Microsoft | `openid profile email offline_access https://outlook.office.com/IMAP.AccessAsUser.All` |

<a id="sign-in"></a>
## Sign in from the desktop app

1. Open **Settings → Mail accounts** and select Gmail or Outlook.
2. Enter the email address and save the account (or start OAuth directly from
   the **OAuth 2.0 sign-in (PKCE)** panel).
3. Click **Sign in with Google / Microsoft** and approve the consent screen in
   your browser.
4. Return to Elyra. The account row shows **OAuth connected**.
5. Use **Sync** to import the newest INBOX messages. The IMAP session
   authenticates with `AUTHENTICATE XOAUTH2`; local read/starred state is
   preserved exactly as for password accounts.

**Disconnect OAuth** deletes the keychain entry and switches the account back
to password authentication. Google revocation posts the stored refresh token
(or the access token if no refresh token exists) to the revocation endpoint on
a best-effort basis; a network failure does not block the local disconnect.
Removing the account also removes its OAuth token.

<a id="commands"></a>
## Tauri commands

| Command | Arguments | Returns |
| :--- | :--- | :--- |
| `oauth_providers` | — | Providers with `configured`, `scopes`, `docsUrl`; `configured` requires a client ID and keychain |
| `oauth_start` | `provider` (`google`/`microsoft`, also `gmail`/`outlook`), `email` | `OAuthSessionView` without secrets |
| `oauth_status` | `accountId` | Connection state, expiry, refresh-token flag |
| `oauth_revoke` | `accountId` | Disconnected view; deletes local tokens |

`OAuthSessionView` contains only `accountId`, `provider`, `email`, `connected`,
`expiresAt`, `hasRefreshToken`, and `needsAttention`. `connected` is false for
an expired session without a refresh token, even if a stale token set remains
until the next sync. Access and refresh tokens never cross the webview
boundary.

<a id="limitations"></a>
## Limitations

- **SMTP.** `send_email` with an OAuth account returns a clear error. Use the
  Elyra Cloud relay, or save an app password to switch the account back to
  password authentication. Tracking issue: `issues/open/003-oauth-smtp-xoauth2.md`.
- **No Elyra Cloud OAuth provider.** Elyra Cloud still uses
  register/login with Argon2 passwords and JWT access tokens. OAuth here is
  only for third-party mail providers.
- **Browser preview and non-Windows builds.** `oauth_*` commands are not
  implemented in `services/tauri.js`, and the OS keychain backend is
  Windows-only, so the buttons do not appear there. Use a password account.
- **Consent portability.** Changing the registered client ID or revoking
  consent requires signing in again.

<a id="troubleshooting"></a>
## Troubleshooting

| Symptom | Cause / action |
| :--- | :--- |
| OAuth panel is missing | Browser preview or non-Windows build: the flow requires Windows Credential Manager. Use a password account. |
| "… OAuth is not configured" | Set `ELYRA_GOOGLE_CLIENT_ID` or `ELYRA_MICROSOFT_CLIENT_ID` and restart/rebuild. |
| "OAuth state did not match" | The callback came from another tab or an old request. Retry sign-in; do not reuse browser tabs. |
| "OAuth sign-in timed out" | Nothing reached the loopback listener within 5 minutes. Check that the browser can reach `127.0.0.1`. |
| "The OAuth token endpoint returned HTTP …" | Wrong client ID, redirect URI, or PKCE verifier. Check the provider's client registration. |
| "OAuth session expired. Reconnect this account." | No usable refresh token. Sign in again. |
| IMAP works, SMTP fails | Expected: SMTP XOAUTH2 is not implemented. Use the Elyra relay, or save an app password to switch the account back to password authentication. |

<a id="related-pages"></a>
## Related pages

- [User guide → Mail accounts](user-guide.md#mail-accounts)
- [Configuration reference](reference/configuration.md)
- [Tauri command reference](reference/tauri-commands.md)
- [Security](security.md)
