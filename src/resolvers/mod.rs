//! External resolvers for 0-hummingbot
//!
//! These resolvers bridge 0-lang graphs to external services.

pub mod exchange;
pub mod http;

// Re-export resolver types
pub use exchange::binance::BinanceResolver;
pub use http::HttpResolver;
