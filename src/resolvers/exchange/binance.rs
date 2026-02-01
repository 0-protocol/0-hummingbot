//! Binance Exchange Resolver
//!
//! Specialized resolver for Binance API calls with authentication support.

use std::collections::HashMap;
use std::sync::Arc;
use zerolang::{ExternalResolver, Tensor};

/// Binance API resolver
pub struct BinanceResolver {
    /// API key (optional, for authenticated requests)
    api_key: Option<String>,
    /// API secret (optional, for signing requests)
    api_secret: Option<String>,
    /// Base URL for REST API
    base_url: String,
    /// Base URL for futures API
    futures_base_url: String,
    /// Testnet mode
    testnet: bool,
}

impl BinanceResolver {
    /// Create a new Binance resolver for public endpoints
    pub fn new() -> Self {
        Self {
            api_key: None,
            api_secret: None,
            base_url: "https://api.binance.com".to_string(),
            futures_base_url: "https://fapi.binance.com".to_string(),
            testnet: false,
        }
    }

    /// Create a resolver with API credentials
    pub fn with_credentials(api_key: &str, api_secret: &str) -> Self {
        Self {
            api_key: Some(api_key.to_string()),
            api_secret: Some(api_secret.to_string()),
            base_url: "https://api.binance.com".to_string(),
            futures_base_url: "https://fapi.binance.com".to_string(),
            testnet: false,
        }
    }

    /// Use testnet endpoints
    pub fn testnet(mut self) -> Self {
        self.base_url = "https://testnet.binance.vision".to_string();
        self.futures_base_url = "https://testnet.binancefuture.com".to_string();
        self.testnet = true;
        self
    }

    /// Parse Binance-specific URI
    /// Format: "binance:{endpoint}:{params}"
    /// Example: "binance:ticker:BTCUSDT"
    fn parse_uri(&self, uri: &str) -> Result<(String, HashMap<String, String>), String> {
        let parts: Vec<&str> = uri.splitn(3, ':').collect();

        if parts.len() < 2 {
            return Err(format!("Invalid Binance URI: {}", uri));
        }

        if parts[0] != "binance" {
            return Err(format!("Expected 'binance' prefix, got: {}", parts[0]));
        }

        let endpoint = parts[1].to_string();
        let mut params = HashMap::new();

        if parts.len() > 2 {
            // Parse params (format: "key=value,key=value" or just "symbol")
            let param_str = parts[2];
            if param_str.contains('=') {
                for pair in param_str.split(',') {
                    let kv: Vec<&str> = pair.split('=').collect();
                    if kv.len() == 2 {
                        params.insert(kv[0].to_string(), kv[1].to_string());
                    }
                }
            } else {
                // Single value assumed to be symbol
                params.insert("symbol".to_string(), param_str.to_string());
            }
        }

        Ok((endpoint, params))
    }

    /// Get ticker price
    fn get_ticker(&self, symbol: &str) -> Result<Tensor, String> {
        // In production, this would make an actual HTTP request
        // For now, return a placeholder
        tracing::info!("Binance: Getting ticker for {}", symbol);
        
        // Placeholder: Return a simulated price
        // In production: fetch from https://api.binance.com/api/v3/ticker/price?symbol={symbol}
        Ok(Tensor::scalar(50000.0, 0.5)) // Placeholder BTC price
    }

    /// Get orderbook
    fn get_orderbook(&self, symbol: &str, limit: u32) -> Result<Tensor, String> {
        tracing::info!("Binance: Getting orderbook for {} (limit: {})", symbol, limit);
        
        // Placeholder: Return simulated orderbook
        // Shape: [limit * 2, 2] - bids then asks, each with [price, quantity]
        let mut data = Vec::new();
        
        // Simulated bids (below mid price)
        for i in 0..limit {
            data.push(49990.0 - (i as f32 * 10.0)); // price
            data.push(0.1 + (i as f32 * 0.01));     // quantity
        }
        
        // Simulated asks (above mid price)
        for i in 0..limit {
            data.push(50010.0 + (i as f32 * 10.0)); // price
            data.push(0.1 + (i as f32 * 0.01));     // quantity
        }

        Ok(Tensor::new(
            vec![limit * 2, 2],
            data,
            0.5, // Placeholder confidence
        ))
    }

    /// Get account balance (requires authentication)
    fn get_balance(&self, asset: &str) -> Result<Tensor, String> {
        if self.api_key.is_none() {
            return Err("API credentials required for balance check".to_string());
        }

        tracing::info!("Binance: Getting balance for {}", asset);
        
        // Placeholder: Return simulated balance
        Ok(Tensor::new(
            vec![2],
            vec![1.5, 10000.0], // [free, locked]
            0.5,
        ))
    }

    /// Place an order (requires authentication)
    fn place_order(
        &self,
        symbol: &str,
        side: &str,
        quantity: f32,
        price: Option<f32>,
    ) -> Result<Tensor, String> {
        if self.api_key.is_none() {
            return Err("API credentials required for placing orders".to_string());
        }

        tracing::info!(
            "Binance: Placing {} order for {} {} @ {:?}",
            side, quantity, symbol, price
        );

        // Placeholder: Return simulated order response
        // [order_id, status, filled_qty, avg_price]
        Ok(Tensor::new(
            vec![4],
            vec![
                12345.0,   // order_id (simulated)
                1.0,       // status: 1.0 = open
                0.0,       // filled_qty
                price.unwrap_or(0.0),
            ],
            0.5,
        ))
    }
}

impl Default for BinanceResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ExternalResolver for BinanceResolver {
    fn resolve(&self, uri: &str, inputs: Vec<&Tensor>) -> Result<Tensor, String> {
        let (endpoint, params) = self.parse_uri(uri)?;

        match endpoint.as_str() {
            "ticker" => {
                let symbol = params.get("symbol").map(|s| s.as_str()).unwrap_or("BTCUSDT");
                self.get_ticker(symbol)
            }
            "orderbook" | "depth" => {
                let symbol = params.get("symbol").map(|s| s.as_str()).unwrap_or("BTCUSDT");
                let limit = params
                    .get("limit")
                    .and_then(|l| l.parse().ok())
                    .unwrap_or(10);
                self.get_orderbook(symbol, limit)
            }
            "balance" => {
                let asset = params.get("asset").map(|s| s.as_str()).unwrap_or("BTC");
                self.get_balance(asset)
            }
            "order" => {
                // Extract order params from input tensor
                if let Some(input) = inputs.first() {
                    if input.data.len() >= 4 {
                        let symbol = params.get("symbol").map(|s| s.as_str()).unwrap_or("BTCUSDT");
                        let side = if input.data[0] > 0.0 { "BUY" } else { "SELL" };
                        let quantity = input.data[1];
                        let price = if input.data[2] > 0.0 {
                            Some(input.data[2])
                        } else {
                            None
                        };
                        return self.place_order(symbol, side, quantity, price);
                    }
                }
                Err("Invalid order input tensor".to_string())
            }
            _ => Err(format!("Unknown Binance endpoint: {}", endpoint)),
        }
    }
}

/// Create a Binance resolver as Arc<dyn ExternalResolver>
pub fn create_binance_resolver() -> Arc<dyn ExternalResolver> {
    Arc::new(BinanceResolver::new())
}

/// Create an authenticated Binance resolver
pub fn create_authenticated_binance_resolver(
    api_key: &str,
    api_secret: &str,
) -> Arc<dyn ExternalResolver> {
    Arc::new(BinanceResolver::with_credentials(api_key, api_secret))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_uri_simple() {
        let resolver = BinanceResolver::new();
        let (endpoint, params) = resolver.parse_uri("binance:ticker:BTCUSDT").unwrap();
        
        assert_eq!(endpoint, "ticker");
        assert_eq!(params.get("symbol"), Some(&"BTCUSDT".to_string()));
    }

    #[test]
    fn test_parse_uri_with_params() {
        let resolver = BinanceResolver::new();
        let (endpoint, params) = resolver
            .parse_uri("binance:orderbook:symbol=ETHUSDT,limit=20")
            .unwrap();
        
        assert_eq!(endpoint, "orderbook");
        assert_eq!(params.get("symbol"), Some(&"ETHUSDT".to_string()));
        assert_eq!(params.get("limit"), Some(&"20".to_string()));
    }

    #[test]
    fn test_get_ticker() {
        let resolver = BinanceResolver::new();
        let result = resolver.resolve("binance:ticker:BTCUSDT", vec![]);
        
        assert!(result.is_ok());
        let tensor = result.unwrap();
        assert!(tensor.is_scalar());
    }

    #[test]
    fn test_get_orderbook() {
        let resolver = BinanceResolver::new();
        let result = resolver.resolve("binance:orderbook:symbol=BTCUSDT,limit=5", vec![]);
        
        assert!(result.is_ok());
        let tensor = result.unwrap();
        assert_eq!(tensor.shape, vec![10, 2]); // 5 bids + 5 asks, each with price and qty
    }

    #[test]
    fn test_unauthenticated_balance_fails() {
        let resolver = BinanceResolver::new();
        let result = resolver.resolve("binance:balance:BTC", vec![]);
        
        assert!(result.is_err());
    }
}
