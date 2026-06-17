use axum::{http::StatusCode, Json};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::Digest as _;

#[derive(Deserialize)]
pub struct PluginAuthRequest {
    encrypted_token: String,
}

#[derive(Serialize)]
pub struct PluginAuthResponse {
    token: String,
}

/// Decrypt an encrypted litellm token delivered from the parent frame.
///
/// litellm encrypts with Fernet using a key derived from LITELLM_SALT_KEY:
///   key = base64url(sha256(LITELLM_SALT_KEY))
///   ciphertext = Fernet.encrypt(token, key)
///
/// We reverse the same derivation and Fernet decryption here so that
/// LITELLM_SALT_KEY never has to leave either process in plaintext.
pub async fn plugin_auth(
    Json(body): Json<PluginAuthRequest>,
) -> Result<Json<PluginAuthResponse>, StatusCode> {
    let salt_key = std::env::var("LITELLM_SALT_KEY").unwrap_or_default();
    if salt_key.is_empty() {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }

    let token =
        fernet_decrypt(&body.encrypted_token, &salt_key).map_err(|_| StatusCode::UNAUTHORIZED)?;

    Ok(Json(PluginAuthResponse { token }))
}

/// Fernet decryption compatible with Python's `cryptography.fernet.Fernet`.
///
/// Fernet token format (after base64url decode):
///   version(1) | timestamp(8) | iv(16) | ciphertext(n) | hmac(32)
///
/// Key layout (32 bytes total):
///   signing_key(16) | encryption_key(16)
fn fernet_decrypt(token: &str, salt_key: &str) -> Result<String, Box<dyn std::error::Error>> {
    use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;
    type HmacSha256 = Hmac<Sha256>;

    // Derive 32-byte Fernet key from salt
    let raw_key = Sha256::digest(salt_key.as_bytes());
    let signing_key = &raw_key[..16];
    let encryption_key = &raw_key[16..];

    // Decode Fernet token (url-safe base64, no padding)
    let data = URL_SAFE_NO_PAD.decode(token.trim_end_matches('='))?;
    if data.len() < 1 + 8 + 16 + 32 {
        return Err("token too short".into());
    }
    if data[0] != 0x80 {
        return Err("unsupported fernet version".into());
    }

    let hmac_start = data.len() - 32;
    let payload = &data[..hmac_start];
    let expected_hmac = &data[hmac_start..];

    // Verify HMAC-SHA256
    let mut mac = HmacSha256::new_from_slice(signing_key)?;
    mac.update(payload);
    mac.verify_slice(expected_hmac)
        .map_err(|_| "hmac mismatch")?;

    // Decrypt AES-128-CBC
    let iv = &data[9..25];
    let ciphertext = &data[25..hmac_start];
    let mut buf = ciphertext.to_vec();
    let plaintext = Aes128CbcDec::new(encryption_key.into(), iv.into())
        .decrypt_padded_mut::<Pkcs7>(&mut buf)
        .map_err(|_| "decrypt failed")?;

    Ok(String::from_utf8(plaintext.to_vec())?)
}
