//! Authentication Utilities for Exchange Connectors
//!
//! Common authentication patterns used by CEX APIs.

use hmac::{Hmac, Mac};
use sha2::Sha256;

/// API credentials for exchange authentication
#[derive(Clone)]
pub struct ApiCredentials {
    /// API key (public identifier)
    pub api_key: String,
    /// API secret (for signing requests)
    pub api_secret: String,
    /// Passphrase (required by some exchanges like OKX)
    pub passphrase: Option<String>,
}

impl ApiCredentials {
    /// Create credentials with key and secret
    pub fn new(api_key: &str, api_secret: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            api_secret: api_secret.to_string(),
            passphrase: None,
        }
    }
    
    /// Create credentials with passphrase (for OKX, KuCoin)
    pub fn with_passphrase(api_key: &str, api_secret: &str, passphrase: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            api_secret: api_secret.to_string(),
            passphrase: Some(passphrase.to_string()),
        }
    }
}

impl std::fmt::Debug for ApiCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ApiCredentials")
            .field("api_key", &format!("{}...", &self.api_key.chars().take(8).collect::<String>()))
            .field("api_secret", &"[REDACTED]")
            .field("passphrase", &self.passphrase.as_ref().map(|_| "[REDACTED]"))
            .finish()
    }
}

/// Sign a message using HMAC-SHA256 (used by Binance, Bybit)
/// Returns hex-encoded signature
pub fn hmac_sha256_sign(secret: &str, message: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Sign a message using HMAC-SHA256 and return base64 (used by OKX, KuCoin)
pub fn hmac_sha256_sign_base64(secret: &str, message: &str) -> String {
    use base64::{Engine as _, engine::general_purpose::STANDARD};
    
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .expect("HMAC can take key of any size");
    mac.update(message.as_bytes());
    STANDARD.encode(mac.finalize().into_bytes())
}

/// Generate a timestamp in milliseconds
pub fn timestamp_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_millis() as u64
}

/// Generate ISO 8601 timestamp (used by OKX)
pub fn timestamp_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

/// Build a query string from key-value pairs
pub fn build_query_string(params: &[(&str, &str)]) -> String {
    params
        .iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&")
}

/// URL-encode a string
pub fn url_encode(s: &str) -> String {
    // Simple percent encoding for query parameters
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hmac_sha256_sign() {
        // Test vector
        let secret = "NhqPtmdSJYdKjVHjA7PZj4Mge3R5YNiP1e3UZjInClVN65XAbvqqM6A7H5fATj0j";
        let message = "symbol=LTCBTC&side=BUY&type=LIMIT&timeInForce=GTC&quantity=1&price=0.1&timestamp=1499827319559";
        let expected = "c8db56825ae71d6d79447849e617115f4a920fa2acdcab2b053c4b2838bd6b71";
        
        assert_eq!(hmac_sha256_sign(secret, message), expected);
    }
    
    #[test]
    fn test_build_query_string() {
        let params = [("symbol", "BTCUSDT"), ("side", "BUY"), ("quantity", "1.0")];
        let query = build_query_string(&params);
        assert_eq!(query, "symbol=BTCUSDT&side=BUY&quantity=1.0");
    }
    
    #[test]
    fn test_credentials_debug_redacts_secrets() {
        let creds = ApiCredentials::with_passphrase("myapikey123456", "mysecret", "mypass");
        let debug_str = format!("{:?}", creds);
        
        assert!(debug_str.contains("myapikey"));
        assert!(!debug_str.contains("mysecret"));
        assert!(!debug_str.contains("mypass"));
        assert!(debug_str.contains("[REDACTED]"));
    }
}
