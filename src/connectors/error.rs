//! Connector Error Types
//!
//! Unified error handling for all exchange connectors.

use std::fmt;

/// Errors that can occur when interacting with exchange connectors
#[derive(Debug, Clone)]
pub enum ConnectorError {
    /// Network-related errors (connection failed, timeout, etc.)
    Network(String),
    
    /// API rate limit exceeded
    RateLimited { retry_after_ms: Option<u64> },
    
    /// Authentication failed (invalid API key, signature, etc.)
    Authentication(String),
    
    /// Invalid request parameters
    InvalidRequest(String),
    
    /// Order-related errors
    OrderError {
        code: Option<i32>,
        message: String,
    },
    
    /// Insufficient balance for operation
    InsufficientBalance {
        asset: String,
        required: String,
        available: String,
    },
    
    /// Exchange returned an error
    ExchangeError {
        code: i32,
        message: String,
    },
    
    /// Response parsing failed
    ParseError(String),
    
    /// WebSocket connection error
    WebSocketError(String),
    
    /// Operation timed out
    Timeout(String),
    
    /// Exchange is under maintenance
    Maintenance,
    
    /// Unknown or unexpected error
    Unknown(String),
}

impl fmt::Display for ConnectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConnectorError::Network(msg) => write!(f, "Network error: {}", msg),
            ConnectorError::RateLimited { retry_after_ms } => {
                if let Some(ms) = retry_after_ms {
                    write!(f, "Rate limited, retry after {}ms", ms)
                } else {
                    write!(f, "Rate limited")
                }
            }
            ConnectorError::Authentication(msg) => write!(f, "Authentication error: {}", msg),
            ConnectorError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            ConnectorError::OrderError { code, message } => {
                if let Some(c) = code {
                    write!(f, "Order error [{}]: {}", c, message)
                } else {
                    write!(f, "Order error: {}", message)
                }
            }
            ConnectorError::InsufficientBalance { asset, required, available } => {
                write!(f, "Insufficient {}: required {}, available {}", asset, required, available)
            }
            ConnectorError::ExchangeError { code, message } => {
                write!(f, "Exchange error [{}]: {}", code, message)
            }
            ConnectorError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            ConnectorError::WebSocketError(msg) => write!(f, "WebSocket error: {}", msg),
            ConnectorError::Timeout(msg) => write!(f, "Timeout: {}", msg),
            ConnectorError::Maintenance => write!(f, "Exchange is under maintenance"),
            ConnectorError::Unknown(msg) => write!(f, "Unknown error: {}", msg),
        }
    }
}

impl std::error::Error for ConnectorError {}

impl From<reqwest::Error> for ConnectorError {
    fn from(err: reqwest::Error) -> Self {
        if err.is_timeout() {
            ConnectorError::Timeout(err.to_string())
        } else if err.is_connect() {
            ConnectorError::Network(format!("Connection failed: {}", err))
        } else {
            ConnectorError::Network(err.to_string())
        }
    }
}

impl From<serde_json::Error> for ConnectorError {
    fn from(err: serde_json::Error) -> Self {
        ConnectorError::ParseError(err.to_string())
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for ConnectorError {
    fn from(err: tokio_tungstenite::tungstenite::Error) -> Self {
        ConnectorError::WebSocketError(err.to_string())
    }
}
