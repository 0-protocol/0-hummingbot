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

        // Create a synchronous wrapper around the async reqwest client
        // Since resolve is synchronous in ExternalResolver, we use tokio::task::block_in_place
        // inside a new runtime or existing handle
        let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create runtime: {}", e))?;
        
        let response_text = rt.block_on(async {
            let mut req = match method.as_str() {
                "get" => self.client.get(&url),
                "post" => self.client.post(&url),
                "put" => self.client.put(&url),
                "delete" => self.client.delete(&url),
                _ => return Err(format!("Unsupported HTTP method: {}", method)),
            };

            // If we have input tensors, we could theoretically serialize them to JSON body
            // For now, we'll just send empty bodies for simplicity, but log it
            if !inputs.is_empty() && (method == "post" || method == "put") {
                tracing::debug!("Warning: Tensor serialization to HTTP body is basic");
                // Example: req = req.json(&some_struct);
            }

            let res = req.send().await.map_err(|e| format!("Request failed: {}", e))?;
            
            if !res.status().is_success() {
                return Err(format!("HTTP Error: {}", res.status()));
            }

            res.text().await.map_err(|e| format!("Failed to read response: {}", e))
        })?;

        // In a full implementation, we'd parse response_text (JSON) into a Tensor.
        // For now, we return a success signal with the length of the string as a proxy value
        tracing::debug!("HTTP response received ({} bytes)", response_text.len());
        Ok(Tensor::scalar(1.0, 1.0)) // Return 1.0 with 1.0 confidence for success
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
