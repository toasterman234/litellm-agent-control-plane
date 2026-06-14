mod config;
mod flow_state;
mod token;

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    response::Redirect,
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{
    db::mcp_servers::repository,
    errors::GatewayError,
    proxy::{auth::master_key::require_any_gateway_key, credential_crypto, state::AppState},
};

use config::{oauth_client_value, oauth_resource, oauth_scopes, required, required_server_url};
use flow_state::{redirect_target, safe_redirect_after, SignedOAuthState};
use token::{credential_from_token, exchange_code, store_oauth_credential};

pub(in crate::http::mcp_registry) use token::resolve_oauth_bearer_token;

#[derive(Debug, Deserialize)]
pub struct StartOAuthRequest {
    pub redirect_after: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct StartOAuthResponse {
    pub authorization_url: String,
    pub redirect_uri: String,
}

#[derive(Debug, Deserialize)]
pub struct OAuthCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

pub async fn start_oauth(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Path(server_id): Path<String>,
    Json(input): Json<StartOAuthRequest>,
) -> Result<Json<StartOAuthResponse>, GatewayError> {
    require_any_gateway_key(&headers, &state)?;

    let pool = state.db.as_ref().ok_or(GatewayError::MissingDatabase)?;
    let server = repository::get(pool, &server_id)
        .await?
        .ok_or_else(|| GatewayError::NotFound(format!("MCP server not found: {server_id}")))?;
    let enc_key =
        credential_crypto::encryption_key(state.config.general_settings.master_key.as_deref())?;
    let client_id = oauth_client_value(&server, &enc_key, &["oauth_client_id", "client_id"])
        .ok_or_else(|| {
            GatewayError::InvalidConfig("MCP OAuth client_id is not configured".to_owned())
        })?;
    let authorization_url = required_server_url(
        server.authorization_url.as_deref(),
        "MCP OAuth authorization_url is not configured",
    )?;
    let scopes = oauth_scopes(&server)?;
    let resource = oauth_resource(&server);
    let redirect_uri = format!("{}/v1/mcp/oauth/callback", flow_state::origin(&headers));
    let signed_state = SignedOAuthState {
        server_id,
        user_id: super::caller_user_id(&headers, &state),
        redirect_after: input.redirect_after,
        redirect_uri: redirect_uri.clone(),
        nonce: uuid::Uuid::new_v4().simple().to_string(),
        iat_ms: flow_state::now_ms(),
    };
    let authorization_url = flow_state::authorization_url(
        authorization_url,
        &client_id,
        &redirect_uri,
        &scopes,
        resource.as_deref(),
        &flow_state::encode_state(&signed_state, &enc_key)?,
    )?;

    Ok(Json(StartOAuthResponse {
        authorization_url,
        redirect_uri,
    }))
}

pub async fn oauth_callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<OAuthCallbackQuery>,
) -> Result<Redirect, GatewayError> {
    let pool = state.db.as_ref().ok_or(GatewayError::MissingDatabase)?;
    let enc_key =
        credential_crypto::encryption_key(state.config.general_settings.master_key.as_deref())?;
    let signed_state = flow_state::decode_state(
        required(query.state.as_deref(), "missing OAuth state")?,
        &enc_key,
    )?;
    let redirect_after = safe_redirect_after(signed_state.redirect_after.as_deref());

    if let Some(error) = query.error {
        let message = query.error_description.unwrap_or(error);
        return Ok(Redirect::to(&redirect_target(
            &redirect_after,
            "failed",
            &signed_state.server_id,
            Some(&message),
        )));
    }

    let code = required(query.code.as_deref(), "missing OAuth code")?;
    let server = repository::get(pool, &signed_state.server_id)
        .await?
        .ok_or_else(|| {
            GatewayError::NotFound(format!("MCP server not found: {}", signed_state.server_id))
        })?;
    let token = exchange_code(&state, &server, &enc_key, code, &signed_state.redirect_uri).await?;
    let credential = credential_from_token(token, None)?;
    store_oauth_credential(
        pool,
        &state,
        &signed_state.server_id,
        &signed_state.user_id,
        &credential,
    )
    .await?;

    Ok(Redirect::to(&redirect_target(
        &redirect_after,
        "connected",
        &signed_state.server_id,
        None,
    )))
}
