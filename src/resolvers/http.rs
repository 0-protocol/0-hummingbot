//! HTTP External Resolver
//!
//! Resolves HTTP requests from 0-lang graphs.

use std::collections::HashMap;
use std::sync::Arc;
use zerolang::{ExternalResolver, Tensor};

/// HTTP resolver for external API calls
pub struct HttpResolver {
    /// HTTP client
    client: reqwest::Client,
    /// Base URLs for different services
    base_urls: HashMap<String, String>,
}

impl HttpResolver {
    /// Create a new HTTP resolver
    pub fn new() -> Self {
        let mut base_urls = HashMap::new();
        
        // Default exchange base URLs
        base_urls.insert("binance".to_string(), "https://api.binance.com".to_string());
        base_urls.insert("okx".to_string(), "https://www.okx.com".to_string());
        base_urls.insert("hyperliquid".to_string(), "https://api.hyperliquid.xyz".to_string());

        Self {
            client: reqwest::Client::new(),
            base_urls,
        }
    }

    /// Add or update a base URL
    pub fn with_base_url(mut self, name: &str, url: &str) -> Self {
        self.base_urls.insert(name.to_string(), url.to_string());
        self
    }

    /// Parse URI and extract method, service, and path
    /// URI format: "http:{method}:{service}:{path}"
    /// Example: "http:get:binance:/api/v3/ticker/price?symbol=BTCUSDT"
    fn parse_uri(&self, uri: &str) -> Result<(String, String, String), String> {
        let parts: Vec<&str> = uri.splitn(4, ':').collect();
        
        if parts.len() < 4 {
            return Err(format!(
                "Invalid URI format. Expected 'http:{{method}}:{{service}}:{{path}}', got: {}",
                uri
            ));
        }

        if parts[0] != "http" {
            return Err(format!("Expected 'http' prefix, got: {}", parts[0]));
        }

        let method = parts[1].to_lowercase();
        let service = parts[2].to_string();
        let path = parts[3].to_string();

        Ok((method, service, path))
    }

    /// Build full URL from service and path
    fn build_url(&self, service: &str, path: &str) -> Result<String, String> {
        let base = self.base_urls.get(service).ok_or_else(|| {
            format!("Unknown service: {}. Available: {:?}", service, self.base_urls.keys())
        })?;

        Ok(format!("{}{}", base, path))
    }
    
    async fn execute_request(&self, method: &str, url: &str, inputs: Vec<&Tensor>) -> Result<String, String> {
        let mut req = match method {
            "get" => self.client.get(url),
            "post" => self.client.post(url),
            "put" => self.client.put(url),
            "delete" => self.client.delete(url),
            _ => return Err(format!("Unsupported HTTP method: {}", method)),
        };

        if !inputs.is_empty() && (method == "post" || method == "put") {
            tracing::debug!("Warning: Tensor serialization to HTTP body is basic");
        }

        let res = req.send().await.map_err(|e| format!("Request failed: {}", e))?;
        
        if !res.status().is_success() {
            return Err(format!("HTTP Error: {}", res.status()));
        }

        res.text().await.map_err(|e| format!("Failed to read response: {}", e))
    }
}

impl Default for HttpResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ExternalResolver for HttpResolver {
    fn resolve(&self, uri: &str, inputs: Vec<&Tensor>) -> Result<Tensor, String> {
        let (method, service, path) = self.parse_uri(uri)?;
        let url = self.build_url(&service, &path)?;

        tracing::info!(
            "HTTP {} {} (inputs: {})",
            method.to_uppercase(),
            url,
            inputs.len()
        );

        // Use tokio handle to spawn or block_on if we're in async context
        // Try getting existing handle, otherwise it means we are purely sync
        let response_text = match tokio::runtime::Handle::try_current() {
            Ok(handle) => tokio::task::block_in_place(|| {
                handle.block_on(async {
                    self.execute_request(&method, &url, inputs).await
                })
            })?,
            Err(_) => {
                // We shouldn't create a new runtime per request. If we are completely outside tokio,
                // we should use a global static runtime, but for now blocking client is better.
                let rt = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
                rt.block_on(async { self.execute_request(&method, &url, inputs).await })?
            }
        };

        // Parse response_text (JSON) into a Tensor.
        tracing::debug!("HTTP response received ({} bytes)", response_text.len());
        
        // Simple heuristic parser for common Exchange API responses
        match serde_json::from_str::<serde_json::Value>(&response_text) {
            Ok(json) => {
                // Try to extract price or value heuristically
                // Better heuristic: try to extract price from nested objects if not at root
                fn find_price(val: &serde_json::Value) -> Option<f32> {
                    if let Some(obj) = val.as_object() {
                        if let Some(p) = obj.get("price").or_else(|| obj.get("last")) {
                            if let Some(s) = p.as_str() {
                                return s.parse().ok();
                            }
                            if let Some(n) = p.as_f64() {
                                return Some(n as f32);
                            }
                        }
                        for (_, v) in obj {
                            if let Some(p) = find_price(v) {
                                return Some(p);
                            }
                        }
                    } else if let Some(arr) = val.as_array() {
                        for v in arr {
                            if let Some(p) = find_price(v) {
                                return Some(p);
                            }
                        }
                    }
                    None
                }
                
                if let Some(value) = find_price(&json) {
                    Ok(Tensor::scalar(value, 1.0))
                } else {
                    tracing::warn!("Could not find price/last field in response, returning 0.0");
                    Ok(Tensor::scalar(0.0, 0.0))
                }
            }
            Err(e) => {
                tracing::error!("Failed to parse JSON response: {}", e);
                // Return 0.0 with 0.0 confidence on parse failure
                Ok(Tensor::scalar(0.0, 0.0))
            }
        }
    }
}

/// Create a shared HTTP resolver
pub fn create_http_resolver() -> Arc<dyn ExternalResolver> {
    Arc::new(HttpResolver::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_uri() {
        let resolver = HttpResolver::new();
        
        let (method, service, path) = resolver
            .parse_uri("http:get:binance:/api/v3/ticker/price?symbol=BTCUSDT")
            .unwrap();
        
        assert_eq!(method, "get");
        assert_eq!(service, "binance");
        assert_eq!(path, "/api/v3/ticker/price?symbol=BTCUSDT");
    }

    #[test]
    fn test_build_url() {
        let resolver = HttpResolver::new();
        
        let url = resolver.build_url("binance", "/api/v3/ticker/price").unwrap();
        assert_eq!(url, "https://api.binance.com/api/v3/ticker/price");
    }

    #[test]
    fn test_invalid_uri() {
        let resolver = HttpResolver::new();
        
        let result = resolver.parse_uri("invalid");
        assert!(result.is_err());
    }
}
