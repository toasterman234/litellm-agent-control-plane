use serde::{Deserialize, Serialize};

use crate::{
    db::{credentials, mcp_servers::schema::McpServerRow},
    errors::GatewayError,
    proxy::{credential_crypto, state::AppState},
};

use super::flow_state::now_ms;

const REFRESH_SKEW_MS: i64 = 60 * 1000;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(super) struct StoredOAuthCredential {
    access_token: String,
    refresh_token: Option<String>,
    expires_at_ms: Option<i64>,
    token_type: Option<String>,
    scope: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct OAuthTokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    token_type: Option<String>,
    scope: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

pub(in crate::http::mcp_registry) async fn resolve_oauth_bearer_token(
    state: &AppState,
    pool: &sqlx::PgPool,
    server: &McpServerRow,
    user_id: &str,
    enc_key: &str,
) -> Result<Option<String>, GatewayError> {
    let Some(raw) = read_user_credential(pool, &server.server_id, user_id, enc_key).await? else {
        return Ok(None);
    };
    let Ok(mut credential) = serde_json::from_str::<StoredOAuthCredential>(&raw) else {
        return Ok(Some(raw));
    };
    if token_is_fresh(credential.expires_at_ms) {
        return Ok(Some(credential.access_token));
    }

    let Some(refresh_token) = credential.refresh_token.clone() else {
        return Ok(Some(credential.access_token));
    };
    let Some(context) = refresh_context(server, enc_key) else {
        return Ok(Some(credential.access_token));
    };
    let token = refresh_access_token(
        state,
        context.token_url,
        &context.client_id,
        &context.client_secret,
        &refresh_token,
        context.resource.as_deref(),
    )
    .await?;
    credential = credential_from_token(token, credential.refresh_token.as_deref())?;
    store_oauth_credential(pool, state, &server.server_id, user_id, &credential).await?;
    Ok(Some(credential.access_token))
}

pub(super) async fn exchange_code(
    state: &AppState,
    server: &McpServerRow,
    enc_key: &str,
    code: &str,
    redirect_uri: &str,
) -> Result<OAuthTokenResponse, GatewayError> {
    let token_url = required_server_url(
        server.token_url.as_deref(),
        "MCP OAuth token_url is not configured",
    )?;
    let client_id = oauth_client_value(server, enc_key, &["oauth_client_id", "client_id"])
        .ok_or_else(|| {
            GatewayError::InvalidConfig("MCP OAuth client_id is not configured".to_owned())
        })?;
    let client_secret =
        oauth_client_value(server, enc_key, &["oauth_client_secret", "client_secret"]).ok_or_else(
            || GatewayError::InvalidConfig("MCP OAuth client_secret is not configured".to_owned()),
        )?;
    let mut form = vec![
        ("grant_type", "authorization_code".to_owned()),
        ("code", code.to_owned()),
        ("redirect_uri", redirect_uri.to_owned()),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ];
    if let Some(resource) = oauth_resource(server) {
        form.push(("resource", resource));
    }
    token_request(state, token_url, form).await
}

pub(super) fn credential_from_token(
    token: OAuthTokenResponse,
    existing_refresh_token: Option<&str>,
) -> Result<StoredOAuthCredential, GatewayError> {
    let access_token = token
        .access_token
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            GatewayError::InvalidConfig("OAuth token response omitted access_token".to_owned())
        })?;
    Ok(StoredOAuthCredential {
        access_token,
        refresh_token: token
            .refresh_token
            .filter(|value| !value.trim().is_empty())
            .or_else(|| existing_refresh_token.map(str::to_owned)),
        expires_at_ms: token.expires_in.map(|seconds| now_ms() + seconds * 1000),
        token_type: token.token_type,
        scope: token.scope,
    })
}

pub(super) async fn store_oauth_credential(
    pool: &sqlx::PgPool,
    state: &AppState,
    server_id: &str,
    user_id: &str,
    credential: &StoredOAuthCredential,
) -> Result<(), GatewayError> {
    let enc_key =
        credential_crypto::encryption_key(state.config.general_settings.master_key.as_deref())?;
    let raw = serde_json::to_string(credential).map_err(GatewayError::InvalidJson)?;
    let encrypted = credential_crypto::encrypt_value(&raw, &enc_key)?;
    let key_name = format!("mcp_user:{server_id}:{user_id}");
    credentials::upsert_vault_key(
        pool,
        &key_name,
        "personal",
        Some(user_id),
        &encrypted,
        user_id,
    )
    .await
}

pub(super) fn oauth_scopes(server: &McpServerRow) -> Result<Vec<String>, GatewayError> {
    let scopes = server
        .mcp_info
        .pointer("/oauth/scopes")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|value| value.as_str())
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if scopes.is_empty() {
        return Err(GatewayError::InvalidConfig(
            "MCP OAuth scopes are not configured in mcp_info.oauth.scopes".to_owned(),
        ));
    }
    Ok(scopes)
}

pub(super) fn oauth_resource(server: &McpServerRow) -> Option<String> {
    server
        .mcp_info
        .pointer("/oauth/resource")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

pub(super) fn oauth_client_value(
    server: &McpServerRow,
    enc_key: &str,
    names: &[&str],
) -> Option<String> {
    names.iter().find_map(|name| {
        server
            .credentials
            .get(*name)
            .and_then(|value| value.as_str())
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|raw| {
                credential_crypto::decrypt_value(raw, enc_key).unwrap_or_else(|_| raw.to_owned())
            })
    })
}

pub(super) fn required_server_url<'a>(
    value: Option<&'a str>,
    message: &str,
) -> Result<&'a str, GatewayError> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| GatewayError::InvalidConfig(message.to_owned()))
}

pub(super) fn required<'a>(value: Option<&'a str>, message: &str) -> Result<&'a str, GatewayError> {
    value
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| GatewayError::InvalidJsonMessage(message.to_owned()))
}

struct RefreshContext<'a> {
    token_url: &'a str,
    client_id: String,
    client_secret: String,
    resource: Option<String>,
}

fn refresh_context<'a>(server: &'a McpServerRow, enc_key: &str) -> Option<RefreshContext<'a>> {
    let token_url = server.token_url.as_deref()?.trim();
    if token_url.is_empty() {
        return None;
    }
    Some(RefreshContext {
        token_url,
        client_id: oauth_client_value(server, enc_key, &["oauth_client_id", "client_id"])?,
        client_secret: oauth_client_value(
            server,
            enc_key,
            &["oauth_client_secret", "client_secret"],
        )?,
        resource: oauth_resource(server),
    })
}

async fn refresh_access_token(
    state: &AppState,
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    refresh_token: &str,
    resource: Option<&str>,
) -> Result<OAuthTokenResponse, GatewayError> {
    let mut form = vec![
        ("grant_type", "refresh_token".to_owned()),
        ("refresh_token", refresh_token.to_owned()),
        ("client_id", client_id.to_owned()),
        ("client_secret", client_secret.to_owned()),
    ];
    if let Some(resource) = resource {
        form.push(("resource", resource.to_owned()));
    }
    token_request(state, token_url, form).await
}

async fn token_request(
    state: &AppState,
    token_url: &str,
    form: Vec<(&str, String)>,
) -> Result<OAuthTokenResponse, GatewayError> {
    let res = state
        .http
        .post(token_url)
        .form(&form)
        .send()
        .await
        .map_err(GatewayError::Upstream)?;
    let status = res.status().as_u16();
    let text = res.text().await.map_err(GatewayError::Upstream)?;
    if status >= 400 {
        return Err(GatewayError::UpstreamHttp(status, text));
    }
    let token =
        serde_json::from_str::<OAuthTokenResponse>(&text).map_err(GatewayError::InvalidJson)?;
    if let Some(error) = token.error.as_deref() {
        let detail = token.error_description.as_deref().unwrap_or(error);
        return Err(GatewayError::UpstreamHttp(status, detail.to_owned()));
    }
    Ok(token)
}

async fn read_user_credential(
    pool: &sqlx::PgPool,
    server_id: &str,
    user_id: &str,
    enc_key: &str,
) -> Result<Option<String>, GatewayError> {
    let key_name = format!("mcp_user:{server_id}:{user_id}");
    let Some(row) = credentials::get_personal_by_name(pool, &key_name, user_id).await? else {
        return Ok(None);
    };
    let Some(encrypted) = row
        .credential_values
        .get("value")
        .and_then(|value| value.as_str())
    else {
        return Ok(None);
    };
    Ok(Some(credential_crypto::decrypt_value(encrypted, enc_key)?))
}

fn token_is_fresh(expires_at_ms: Option<i64>) -> bool {
    expires_at_ms.is_none_or(|expires_at| expires_at > now_ms() + REFRESH_SKEW_MS)
}
