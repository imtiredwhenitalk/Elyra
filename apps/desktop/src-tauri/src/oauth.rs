//! OAuth 2.0 Authorization Code + PKCE for third-party mail providers.
//!
//! The desktop client acts as a **public native client**: no client secret is
//! used or stored. A temporary loopback listener receives the authorization
//! callback, the code is exchanged with PKCE, and the resulting tokens live in
//! the OS keychain under `oauth-account-<id>`. Neither access nor refresh
//! tokens are ever returned to the webview.
//!
//! Client IDs are public values. They can be baked in at build time with
//! `ELYRA_GOOGLE_CLIENT_ID` / `ELYRA_MICROSOFT_CLIENT_ID`, or supplied at
//! runtime for local development. Without a client ID the commands return a
//! clear error instead of starting a flow that cannot complete.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use rand::RngCore;
use reqwest::blocking::Client;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;
use url::Url;

use crate::{secrets, security, storage};

pub(crate) const GOOGLE: &str = "google";
pub(crate) const MICROSOFT: &str = "microsoft";

const CALLBACK_PATH: &str = "/callback";
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);
const REFRESH_SKEW_SECS: i64 = 60;

struct OAuthProvider {
    id: &'static str,
    label: &'static str,
    authorize_url: &'static str,
    token_url: &'static str,
    revoke_url: Option<&'static str>,
    scopes: &'static str,
    client_id_var: &'static str,
    docs_url: &'static str,
    imap_host: &'static str,
    smtp_host: &'static str,
}

const PROVIDERS: &[OAuthProvider] = &[
    OAuthProvider {
        id: GOOGLE,
        label: "Google / Gmail",
        authorize_url: "https://accounts.google.com/o/oauth2/v2/auth",
        token_url: "https://oauth2.googleapis.com/token",
        revoke_url: Some("https://oauth2.googleapis.com/revoke"),
        scopes: "openid email profile https://mail.google.com/",
        client_id_var: "ELYRA_GOOGLE_CLIENT_ID",
        docs_url: "https://developers.google.com/identity/protocols/oauth2/native-app",
        imap_host: "imap.gmail.com",
        smtp_host: "smtp.gmail.com",
    },
    OAuthProvider {
        id: MICROSOFT,
        label: "Microsoft / Outlook",
        authorize_url: "https://login.microsoftonline.com/common/oauth2/v2.0/authorize",
        token_url: "https://login.microsoftonline.com/common/oauth2/v2.0/token",
        revoke_url: None,
        scopes:
            "openid profile email offline_access https://outlook.office.com/IMAP.AccessAsUser.All",
        client_id_var: "ELYRA_MICROSOFT_CLIENT_ID",
        docs_url:
            "https://learn.microsoft.com/azure/active-directory/develop/v2-oauth2-auth-code-flow",
        imap_host: "outlook.office365.com",
        smtp_host: "smtp.office365.com",
    },
];

fn preset(provider: &str) -> Result<&'static OAuthProvider, String> {
    // `gmail` / `outlook` are the ids used by the account form; accept them as
    // aliases so the account row keeps the id the UI already understands.
    let id = match provider {
        "gmail" => GOOGLE,
        "outlook" => MICROSOFT,
        other => other,
    };
    PROVIDERS
        .iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| "Unsupported OAuth provider".to_string())
}

fn account_provider_id(provider: &OAuthProvider) -> &'static str {
    match provider.id {
        GOOGLE => "gmail",
        MICROSOFT => "outlook",
        _ => provider.id,
    }
}

fn build_client_id(provider: &str) -> Option<String> {
    let (variable, embedded) = match provider {
        GOOGLE => (
            "ELYRA_GOOGLE_CLIENT_ID",
            option_env!("ELYRA_GOOGLE_CLIENT_ID"),
        ),
        MICROSOFT => (
            "ELYRA_MICROSOFT_CLIENT_ID",
            option_env!("ELYRA_MICROSOFT_CLIENT_ID"),
        ),
        _ => return None,
    };
    std::env::var(variable)
        .ok()
        .or_else(|| embedded.map(str::to_string))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn require_client_id(provider: &OAuthProvider) -> Result<String, String> {
    build_client_id(provider.id).ok_or_else(|| {
        format!(
            "{} OAuth is not configured. Set {} when building or starting Elyra.",
            provider.label, provider.client_id_var
        )
    })
}

fn random_token() -> String {
    let mut bytes = [0_u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn pkce_challenge(verifier: &str) -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()))
}

fn timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs() as i64)
        .unwrap_or_default()
}

fn expires_at_from(seconds: i64) -> i64 {
    timestamp().saturating_add(seconds.max(0))
}

fn rfc3339(seconds: i64) -> Option<String> {
    chrono::DateTime::<chrono::Utc>::from_timestamp(seconds, 0).map(|value| value.to_rfc3339())
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthProviderView {
    pub id: String,
    pub label: String,
    pub configured: bool,
    pub scopes: String,
    pub docs_url: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthSessionView {
    pub account_id: i64,
    pub provider: String,
    pub email: String,
    pub connected: bool,
    pub expires_at: Option<String>,
    pub has_refresh_token: bool,
    pub needs_attention: bool,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    expires_in: i64,
    #[serde(default)]
    scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredTokens {
    provider: String,
    access_token: String,
    refresh_token: Option<String>,
    expires_at: i64,
    scope: Option<String>,
}

fn read_tokens(account_id: i64) -> Result<Option<StoredTokens>, String> {
    let raw = secrets::get_secret(&secrets::oauth_user(account_id))?;
    match raw {
        None => Ok(None),
        Some(raw) => serde_json::from_str(&raw).map(Some).map_err(|_| {
            "The stored OAuth session is unreadable; reconnect the account".to_string()
        }),
    }
}

fn write_tokens(account_id: i64, tokens: &StoredTokens) -> Result<(), String> {
    let raw = serde_json::to_string(tokens)
        .map_err(|error| format!("Could not encode the OAuth session: {error}"))?;
    secrets::set_secret(&secrets::oauth_user(account_id), &raw)
}

fn emit_event(app: &AppHandle, event: &str, outcome: &str, detail: &str) {
    if let Ok(database) = storage::connection(app) {
        let _ = security::record_event(&database, event, outcome, detail);
    }
}

fn authorize_url(
    provider: &OAuthProvider,
    client_id: &str,
    redirect_uri: &str,
    state: &str,
    challenge: &str,
) -> Result<Url, String> {
    let mut url = Url::parse(provider.authorize_url)
        .map_err(|error| format!("Invalid OAuth authorization URL: {error}"))?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", provider.scopes)
        .append_pair("state", state)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256");
    if provider.id == GOOGLE {
        url.query_pairs_mut()
            .append_pair("access_type", "offline")
            .append_pair("prompt", "consent");
    } else {
        url.query_pairs_mut()
            .append_pair("prompt", "select_account");
    }
    Ok(url)
}

fn respond(stream: &mut TcpStream, status: &str, title: &str, message: &str) {
    let body = format!(
        "<!doctype html><html lang=\"en\"><meta charset=\"utf-8\">\
         <title>{title}</title><style>body{{font:16px system-ui;max-width:34rem;\
         margin:4rem auto;padding:0 1rem;line-height:1.5}}</style>\
         <h1>{title}</h1><p>{message}</p><p>You can close this window and return to Elyra.</p></html>"
    );
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\n\
         Content-Length: {}\r\nCache-Control: no-store\r\n\
         Content-Security-Policy: default-src 'none'\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.flush();
}

/// Wait for the provider's redirect on the loopback listener.
fn receive_code(listener: &TcpListener, expected_state: &str) -> Result<String, String> {
    listener
        .set_nonblocking(true)
        .map_err(|error| format!("Could not prepare the OAuth callback: {error}"))?;
    let deadline = Instant::now() + CALLBACK_TIMEOUT;
    loop {
        if Instant::now() >= deadline {
            return Err("OAuth sign-in timed out. Start the connection again.".to_string());
        }
        match listener.accept() {
            Ok((mut stream, _)) => {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
                let mut buffer = [0_u8; 8192];
                let mut request = String::new();
                loop {
                    let read = stream.read(&mut buffer).unwrap_or(0);
                    if read == 0 {
                        break;
                    }
                    request.push_str(&String::from_utf8_lossy(&buffer[..read]));
                    if request.contains("\r\n\r\n") || request.len() >= buffer.len() {
                        break;
                    }
                }
                let target = request
                    .lines()
                    .next()
                    .and_then(|line| line.split_whitespace().nth(1))
                    .unwrap_or("");
                if !target.starts_with(CALLBACK_PATH) {
                    respond(
                        &mut stream,
                        "404 Not Found",
                        "Not found",
                        "Unexpected callback path.",
                    );
                    continue;
                }
                let callback = Url::parse(&format!("http://127.0.0.1{target}"))
                    .map_err(|_| "The OAuth callback URL was malformed".to_string())?;
                let pairs: HashMap<String, String> = callback
                    .query_pairs()
                    .map(|(key, value)| (key.into_owned(), value.into_owned()))
                    .collect();
                if let Some(error) = pairs.get("error") {
                    respond(
                        &mut stream,
                        "400 Bad Request",
                        "Sign-in failed",
                        "The provider did not authorize this request.",
                    );
                    return Err(format!("OAuth provider rejected the request: {error}"));
                }
                if pairs.get("state").map(String::as_str) != Some(expected_state) {
                    respond(
                        &mut stream,
                        "400 Bad Request",
                        "Sign-in failed",
                        "The request could not be verified. Please try again.",
                    );
                    return Err("OAuth state did not match the original request".to_string());
                }
                let Some(code) = pairs.get("code").filter(|code| !code.is_empty()).cloned() else {
                    respond(
                        &mut stream,
                        "400 Bad Request",
                        "Sign-in failed",
                        "The provider did not return an authorization code.",
                    );
                    return Err(
                        "The OAuth callback did not contain an authorization code".to_string()
                    );
                };
                respond(
                    &mut stream,
                    "200 OK",
                    "Connected",
                    "Elyra can now access this mailbox.",
                );
                return Ok(code);
            }
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(200));
            }
            Err(error) => return Err(format!("OAuth callback failed: {error}")),
        }
    }
}

fn token_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|error| format!("Could not initialize the OAuth HTTP client: {error}"))
}

fn request_tokens(
    provider: &OAuthProvider,
    params: &[(&str, &str)],
) -> Result<TokenResponse, String> {
    let response = token_client()?
        .post(provider.token_url)
        .form(params)
        .send()
        .map_err(|error| format!("Could not reach the OAuth token endpoint: {error}"))?;
    let status = response.status();
    if !status.is_success() {
        let hint = if status.as_u16() == 400 || status.as_u16() == 401 {
            ". Reconnect this account"
        } else {
            ""
        };
        return Err(format!(
            "The OAuth token endpoint returned HTTP {}{hint}",
            status.as_u16()
        ));
    }
    response
        .json::<TokenResponse>()
        .map_err(|_| "The OAuth token response could not be read".to_string())
}

fn exchange_code(
    provider: &OAuthProvider,
    client_id: &str,
    code: &str,
    redirect_uri: &str,
    verifier: &str,
) -> Result<TokenResponse, String> {
    request_tokens(
        provider,
        &[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("client_id", client_id),
            ("redirect_uri", redirect_uri),
            ("code_verifier", verifier),
        ],
    )
}

fn refresh_tokens(
    provider: &OAuthProvider,
    client_id: &str,
    refresh_token: &str,
) -> Result<TokenResponse, String> {
    request_tokens(
        provider,
        &[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("client_id", client_id),
        ],
    )
}

fn upsert_account(app: &AppHandle, provider: &OAuthProvider, email: &str) -> Result<i64, String> {
    crate::providers::initialize(app)?;
    let database = storage::connection(app)?;
    database
        .query_row(
            "INSERT INTO accounts
                (provider, email, display_name, username, imap_host, imap_port,
                 imap_security, smtp_host, smtp_port, smtp_security, auth_method, updated_at)
             VALUES (?1, ?2, ?2, ?2, ?3, 993, 'tls', ?4, 465, 'tls', 'oauth2', ?5)
             ON CONFLICT(email) DO UPDATE SET
                provider = excluded.provider,
                username = excluded.username,
                imap_host = excluded.imap_host,
                imap_port = excluded.imap_port,
                imap_security = excluded.imap_security,
                smtp_host = excluded.smtp_host,
                smtp_port = excluded.smtp_port,
                smtp_security = excluded.smtp_security,
                auth_method = 'oauth2',
                updated_at = excluded.updated_at
             RETURNING id",
            params![
                account_provider_id(provider),
                email,
                provider.imap_host,
                provider.smtp_host,
                crate::providers::now_string()
            ],
            |row| row.get(0),
        )
        .map_err(|error| format!("Could not save the OAuth account: {error}"))
}

fn session_view(
    _app: &AppHandle,
    account_id: i64,
    provider: &str,
    email: String,
    tokens: Option<&StoredTokens>,
) -> OAuthSessionView {
    let needs_attention = tokens
        .map(|value| {
            value.expires_at <= timestamp()
                && value
                    .refresh_token
                    .as_deref()
                    .unwrap_or_default()
                    .is_empty()
        })
        .unwrap_or(false);
    OAuthSessionView {
        account_id,
        provider: provider.to_string(),
        email,
        connected: tokens.is_some() && !needs_attention,
        expires_at: tokens.and_then(|value| rfc3339(value.expires_at)),
        has_refresh_token: tokens
            .map(|value| value.refresh_token.is_some())
            .unwrap_or(false),
        needs_attention,
    }
}

#[tauri::command]
pub fn oauth_providers() -> Vec<OAuthProviderView> {
    // Never offer a flow that cannot finish: tokens need a working OS keychain.
    let keychain_ready = secrets::available();
    PROVIDERS
        .iter()
        .map(|provider| OAuthProviderView {
            id: provider.id.to_string(),
            label: provider.label.to_string(),
            configured: keychain_ready && build_client_id(provider.id).is_some(),
            scopes: provider.scopes.to_string(),
            docs_url: provider.docs_url.to_string(),
        })
        .collect()
}

#[tauri::command(async)]
pub fn oauth_start(
    app: AppHandle,
    provider: String,
    email: String,
) -> Result<OAuthSessionView, String> {
    let provider_config = preset(&provider)?;
    let email = email.trim().to_ascii_lowercase();
    if !email.contains('@') || email.len() > 320 {
        return Err("Enter a valid email address for this mailbox".to_string());
    }
    if !secrets::available() {
        return Err(
            "OAuth sign-in requires the OS keychain. Use a password account in this build."
                .to_string(),
        );
    }
    let client_id = require_client_id(provider_config)?;

    let listener = TcpListener::bind("127.0.0.1:0")
        .map_err(|error| format!("Could not start the OAuth callback listener: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| format!("Could not read the callback port: {error}"))?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{port}{CALLBACK_PATH}");

    let state = random_token();
    let verifier = random_token();
    let url = authorize_url(
        provider_config,
        &client_id,
        &redirect_uri,
        &state,
        &pkce_challenge(&verifier),
    )?;
    app.opener()
        .open_url(url.as_str(), None::<&str>)
        .map_err(|error| format!("Could not open the browser for OAuth sign-in: {error}"))?;

    let code = receive_code(&listener, &state)?;
    let tokens = exchange_code(provider_config, &client_id, &code, &redirect_uri, &verifier)?;
    let account_id = upsert_account(&app, provider_config, &email)?;
    let stored = StoredTokens {
        provider: provider_config.id.to_string(),
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        expires_at: expires_at_from(tokens.expires_in),
        scope: tokens.scope,
    };
    write_tokens(account_id, &stored)?;
    // The account may have had an IMAP password before it was converted to
    // OAuth. Once the token set is safely stored, remove that stale secret.
    let _ = secrets::delete_secret(&secrets::account_user(account_id));
    emit_event(
        &app,
        "oauth.connect",
        "success",
        &format!("{} authorized {}", provider_config.label, email),
    );
    Ok(session_view(
        &app,
        account_id,
        account_provider_id(provider_config),
        email,
        Some(&stored),
    ))
}

#[tauri::command]
pub fn oauth_status(app: AppHandle, account_id: i64) -> Result<OAuthSessionView, String> {
    let database = storage::connection(&app)?;
    let (provider, email): (String, String) = database
        .query_row(
            "SELECT provider, email FROM accounts WHERE id = ?1",
            params![account_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| format!("Email account not found: {error}"))?;
    let tokens = read_tokens(account_id)?;
    Ok(session_view(
        &app,
        account_id,
        &provider,
        email,
        tokens.as_ref(),
    ))
}

#[tauri::command(async)]
pub fn oauth_revoke(app: AppHandle, account_id: i64) -> Result<OAuthSessionView, String> {
    let database = storage::connection(&app)?;
    let (provider, email): (String, String) = database
        .query_row(
            "SELECT provider, email FROM accounts WHERE id = ?1",
            params![account_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|error| format!("Email account not found: {error}"))?;
    let tokens = read_tokens(account_id)?;

    // Remote revocation is best-effort: losing the network must not prevent a
    // local disconnect. Microsoft does not expose a public revocation endpoint.
    let mut remote_error = None;
    if let Some(tokens) = tokens.as_ref() {
        // Stored tokens use canonical provider ids (`google`/`microsoft`),
        // while the account row may use presets (`gmail`/`outlook`). `preset`
        // accepts both aliases and keeps revocation working after an upgrade.
        if let Ok(config) = preset(&tokens.provider) {
            if let Some(endpoint) = config.revoke_url {
                // Google accepts either token; prefer the refresh token so a
                // revoked grant cannot be resurrected from a stored refresh.
                let token = tokens
                    .refresh_token
                    .as_deref()
                    .unwrap_or(tokens.access_token.as_str());
                let result = token_client().and_then(|client| {
                    let response = client
                        .post(endpoint)
                        .form(&[("token", token)])
                        .send()
                        .map_err(|error| error.to_string())?;
                    let status = response.status();
                    if status.is_success() {
                        Ok(())
                    } else {
                        Err(format!("HTTP {}", status.as_u16()))
                    }
                });
                if let Err(error) = result {
                    remote_error = Some(error);
                }
            }
        }
    }

    secrets::delete_secret(&secrets::oauth_user(account_id))?;
    database
        .execute(
            "UPDATE accounts SET auth_method = 'password', updated_at = ?2 WHERE id = ?1",
            params![account_id, crate::providers::now_string()],
        )
        .map_err(|error| error.to_string())?;
    emit_event(
        &app,
        "oauth.revoke",
        if remote_error.is_none() {
            "success"
        } else {
            "warning"
        },
        &format!("OAuth access removed for {email}"),
    );
    let mut view = session_view(&app, account_id, &provider, email, None);
    if let Some(error) = remote_error {
        view.needs_attention = true;
    }
    Ok(view)
}

/// Resolve a usable access token for IMAP/SMTP, refreshing it when needed.
pub(crate) fn access_token(_app: &AppHandle, account_id: i64) -> Result<String, String> {
    let Some(mut tokens) = read_tokens(account_id)? else {
        return Err("This account is not connected with OAuth. Reconnect it.".to_string());
    };
    if tokens.expires_at > timestamp() + REFRESH_SKEW_SECS {
        return Ok(tokens.access_token);
    }
    let Some(refresh_token) = tokens.refresh_token.clone() else {
        return Err("The OAuth session expired. Reconnect this account.".to_string());
    };
    let provider = preset(&tokens.provider)?;
    let client_id = require_client_id(provider)?;
    let refreshed = refresh_tokens(provider, &client_id, &refresh_token)?;
    tokens.access_token = refreshed.access_token;
    if let Some(refresh_token) = refreshed.refresh_token {
        tokens.refresh_token = Some(refresh_token);
    }
    tokens.scope = refreshed.scope.or(tokens.scope);
    tokens.expires_at = expires_at_from(refreshed.expires_in);
    write_tokens(account_id, &tokens)?;
    Ok(tokens.access_token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pkce_challenge_matches_rfc_7636_vector() {
        let verifier = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
        assert_eq!(
            pkce_challenge(verifier),
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn preset_accepts_account_provider_aliases() {
        assert_eq!(preset("gmail").map(|value| value.id), Ok(GOOGLE));
        assert_eq!(preset("google").map(|value| value.id), Ok(GOOGLE));
        assert_eq!(preset("outlook").map(|value| value.id), Ok(MICROSOFT));
        assert_eq!(preset("microsoft").map(|value| value.id), Ok(MICROSOFT));
    }

    #[test]
    fn random_state_and_verifier_are_distinct_url_safe_values() {
        let first = random_token();
        let second = random_token();
        assert_ne!(first, second);
        for value in [&first, &second] {
            assert_eq!(value.len(), 43);
            assert!(value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_'));
        }
    }

    #[test]
    fn authorization_url_carries_pkce_and_state() {
        let url = authorize_url(
            preset(GOOGLE).expect("google preset"),
            "client.apps.googleusercontent.com",
            "http://127.0.0.1:53123/callback",
            "state-value",
            "challenge-value",
        )
        .expect("authorization URL");
        let pairs: HashMap<_, _> = url.query_pairs().into_owned().collect();
        assert_eq!(pairs.get("response_type").map(String::as_str), Some("code"));
        assert_eq!(pairs.get("state").map(String::as_str), Some("state-value"));
        assert_eq!(
            pairs.get("code_challenge_method").map(String::as_str),
            Some("S256")
        );
        assert_eq!(
            pairs.get("access_type").map(String::as_str),
            Some("offline")
        );
        assert!(pairs
            .get("scope")
            .map(|value| value.contains("https://mail.google.com/"))
            .unwrap_or(false));
    }

    #[test]
    fn refresh_and_expiry_helpers_are_monotonic() {
        let expiry = expires_at_from(3600);
        assert!(expiry > timestamp() + 3500);
        assert_eq!(rfc3339(expiry).map(|value| value.is_empty()), Some(false));
        assert_eq!(session_fields_expire_without_refresh(expiry, None), false);
        assert_eq!(
            session_fields_expire_without_refresh(timestamp() - 10, Some("refresh".to_string())),
            false
        );
    }

    fn session_fields_expire_without_refresh(expiry: i64, refresh: Option<String>) -> bool {
        expiry <= timestamp() && refresh.unwrap_or_default().is_empty()
    }

    #[test]
    fn loopback_callback_accepts_matching_state_and_rejects_other_state() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let address = listener.local_addr().expect("local address");
        let sender = thread::spawn(move || {
            let mut stream = TcpStream::connect(address).expect("connect");
            let _ = stream.write_all(
                b"GET /callback?code=the-code&state=expected HTTP/1.1\r\nHost: localhost\r\n\r\n",
            );
            let mut response = [0_u8; 256];
            let _ = stream.read(&mut response);
        });
        assert_eq!(
            receive_code(&listener, "expected").expect("code"),
            "the-code"
        );
        sender.join().expect("sender thread");

        let listener = TcpListener::bind("127.0.0.1:0").expect("bind loopback");
        let address = listener.local_addr().expect("local address");
        let sender = thread::spawn(move || {
            let mut stream = TcpStream::connect(address).expect("connect");
            let _ = stream.write_all(
                b"GET /callback?code=the-code&state=attacker HTTP/1.1\r\nHost: localhost\r\n\r\n",
            );
            let mut response = [0_u8; 256];
            let _ = stream.read(&mut response);
        });
        assert!(receive_code(&listener, "expected").is_err());
        sender.join().expect("sender thread");
    }
}
