//! Hyperliquid-specific types

use serde::{Deserialize, Serialize};

/// Asset metadata
#[derive(Debug, Clone)]
pub struct AssetMeta {
    pub index: u32,
    pub name: String,
    pub sz_decimals: u32,
    pub max_leverage: u32,
}

/// L2 level
#[derive(Debug, Deserialize)]
pub struct L2Level {
    pub px: String,
    pub sz: String,
    pub n: u32,
}

/// Order wire format for API
#[derive(Debug, Clone, Serialize)]
pub struct OrderWire {
    pub a: u32,  // Asset index
    pub b: bool, // Is buy
    pub p: String, // Price
    pub s: String, // Size
    pub r: bool, // Reduce only
    pub t: serde_json::Value, // Order type
}
