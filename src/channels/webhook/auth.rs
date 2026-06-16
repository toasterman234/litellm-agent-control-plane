use axum::http::HeaderMap;

use crate::errors::GatewayError;

pub(super) fn verify_webhook_secret(headers: &HeaderMap, secret: &str) -> Result<(), GatewayError> {
    let secret = secret.trim();
    if secret.is_empty() {
        return Err(GatewayError::InvalidConfig(
            "webhook secret is empty".to_owned(),
        ));
    }
    if authorization_matches_secret(headers, secret) {
        return Ok(());
    }
    Err(GatewayError::Unauthorized)
}

fn authorization_matches_secret(headers: &HeaderMap, secret: &str) -> bool {
    headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .map(authorization_token)
        .is_some_and(|value| constant_time_eq(value, secret))
}

pub(super) fn authorization_token(value: &str) -> &str {
    let value = value.trim();
    if value.len() >= 7 && value[..7].eq_ignore_ascii_case("bearer ") {
        return value[7..].trim();
    }
    value
}

pub(super) fn constant_time_eq(left: &str, right: &str) -> bool {
    const MAX_WEBHOOK_SECRET_BYTES: usize = 4096;

    let left = left.as_bytes();
    let right = right.as_bytes();
    let mut diff = left.len() ^ right.len();
    if left.len() > MAX_WEBHOOK_SECRET_BYTES || right.len() > MAX_WEBHOOK_SECRET_BYTES {
        diff |= 1;
    }
    for index in 0..MAX_WEBHOOK_SECRET_BYTES {
        let left_byte = left.get(index).copied().unwrap_or(0);
        let right_byte = right.get(index).copied().unwrap_or(0);
        diff |= usize::from(left_byte ^ right_byte);
    }
    diff == 0
}
