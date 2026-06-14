use std::{
    sync::OnceLock,
    time::{Duration, Instant},
};

use jsonwebtoken::{decode, decode_header, jwk::Jwk, Algorithm, DecodingKey, Validation};
use reqwest::Client;
use serde::Deserialize;
use serde_json::Value;

use crate::errors::GatewayError;

const GOOGLE_JWKS_URL: &str = "https://www.googleapis.com/oauth2/v3/certs";
const GOOGLE_CHAT_ISSUER: &str = "chat@system.gserviceaccount.com";
const KEY_CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

static GOOGLE_CHAT_KEYS: OnceLock<tokio::sync::RwLock<Option<CachedKeys>>> = OnceLock::new();

#[derive(Debug, Clone, Deserialize)]
struct RawJwkSet {
    keys: Vec<Value>,
}

#[derive(Debug, Clone)]
struct CachedKeys {
    keys: RawJwkSet,
    fetched_at: Instant,
}

#[derive(Debug, Deserialize)]
struct GoogleChatClaims {
    iss: String,
    aud: String,
}

pub(crate) async fn verify_google_chat_request(
    client: &Client,
    authorization: Option<&str>,
    auth_audience: &str,
) -> Result<(), GatewayError> {
    let token = bearer_token(authorization)?;
    let header = decode_header(token).map_err(|_| GatewayError::Unauthorized)?;
    let key_id = header
        .kid
        .or(header.x5t)
        .ok_or(GatewayError::Unauthorized)?;
    let mut keys = google_chat_keys(client, false).await?;
    let mut key_value = keys
        .keys
        .iter()
        .find(|key| key_matches(key, &key_id))
        .cloned();
    if key_value.is_none() {
        keys = google_chat_keys(client, true).await?;
        key_value = keys
            .keys
            .iter()
            .find(|key| key_matches(key, &key_id))
            .cloned();
    }
    let key_value = key_value.ok_or(GatewayError::Unauthorized)?;
    let jwk: Jwk = serde_json::from_value(key_value).map_err(GatewayError::InvalidJson)?;
    let key = DecodingKey::from_jwk(&jwk).map_err(|_| GatewayError::Unauthorized)?;
    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[auth_audience]);
    validation.set_issuer(&[GOOGLE_CHAT_ISSUER]);
    validation.leeway = 300;
    validation.validate_nbf = true;
    let data = decode::<GoogleChatClaims>(token, &key, &validation)
        .map_err(|_| GatewayError::Unauthorized)?;
    if data.claims.iss != GOOGLE_CHAT_ISSUER || data.claims.aud != auth_audience {
        return Err(GatewayError::Unauthorized);
    }
    Ok(())
}

fn bearer_token(authorization: Option<&str>) -> Result<&str, GatewayError> {
    authorization
        .and_then(|value| value.trim().strip_prefix("Bearer "))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or(GatewayError::Unauthorized)
}

async fn google_chat_keys(client: &Client, force_refresh: bool) -> Result<RawJwkSet, GatewayError> {
    let cache = GOOGLE_CHAT_KEYS.get_or_init(|| tokio::sync::RwLock::new(None));
    if !force_refresh {
        if let Some(cached) = cache.read().await.as_ref() {
            if cached.fetched_at.elapsed() < KEY_CACHE_TTL {
                return Ok(cached.keys.clone());
            }
        }
    }
    let fresh = fetch_google_chat_keys(client).await?;
    *cache.write().await = Some(CachedKeys {
        keys: fresh.clone(),
        fetched_at: Instant::now(),
    });
    Ok(fresh)
}

async fn fetch_google_chat_keys(client: &Client) -> Result<RawJwkSet, GatewayError> {
    client
        .get(GOOGLE_JWKS_URL)
        .send()
        .await
        .map_err(GatewayError::Upstream)?
        .json()
        .await
        .map_err(GatewayError::Upstream)
}

fn key_matches(key: &Value, key_id: &str) -> bool {
    key.get("kid").and_then(Value::as_str) == Some(key_id)
        || key.get("x5t").and_then(Value::as_str) == Some(key_id)
}
