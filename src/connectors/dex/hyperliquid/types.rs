//! Hyperliquid-specific types
//!
//! Types for Hyperliquid API responses and internal data structures.

use serde::{Deserialize, Serialize};

/// Asset metadata
#[derive(Debug, Clone)]
pub struct AssetMeta {
    /// Asset index (used in orders)
    pub index: u32,
    /// Asset name/symbol
    pub name: String,
    /// Size decimals
    pub sz_decimals: u32,
    /// Maximum leverage
    pub max_leverage: u32,
}

/// Meta response from /info
#[derive(Debug, Deserialize)]
pub struct MetaResponse {
    pub universe: Vec<AssetInfo>,
}

/// Asset info from meta
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AssetInfo {
    pub name: String,
    pub sz_decimals: u32,
    pub max_leverage: u32,
    #[serde(default)]
    pub only_isolated: bool,
}

/// L2 Book response
#[derive(Debug, Deserialize)]
pub struct L2BookResponse {
    pub coin: String,
    pub levels: Vec<Vec<L2Level>>,
    pub time: i64,
}

/// L2 level
#[derive(Debug, Deserialize)]
pub struct L2Level {
    pub px: String,
    pub sz: String,
    pub n: u32,
}

/// Trade from recent trades
#[derive(Debug, Deserialize)]
pub struct HlTrade {
    pub coin: String,
    pub side: String,
    pub px: String,
    pub sz: String,
    pub time: i64,
    pub hash: String,
    pub tid: u64,
}

/// User state from clearinghouse
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserState {
    pub margin_summary: MarginSummary,
    pub cross_margin_summary: CrossMarginSummary,
    pub asset_positions: Vec<AssetPosition>,
    #[serde(default)]
    pub withdrawable: String,
}

/// Margin summary
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MarginSummary {
    pub account_value: String,
    pub total_ntl_pos: String,
    pub total_raw_usd: String,
    pub total_margin_used: String,
}

/// Cross margin summary
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CrossMarginSummary {
    pub account_value: String,
    pub total_ntl_pos: String,
    pub total_raw_usd: String,
    pub total_margin_used: String,
}

/// Asset position
#[derive(Debug, Deserialize)]
pub struct AssetPosition {
    #[serde(rename = "type")]
    pub position_type: String,
    pub position: PositionData,
}

/// Position data
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PositionData {
    pub coin: String,
    pub szi: String,
    pub entry_px: Option<String>,
    pub position_value: String,
    pub unrealized_pnl: String,
    pub leverage: LeverageInfo,
    pub liquidation_px: Option<String>,
    pub margin_used: String,
    #[serde(default)]
    pub max_trade_szs: Vec<String>,
}

/// Leverage info
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LeverageInfo {
    #[serde(rename = "type")]
    pub leverage_type: String,
    pub value: String,
    #[serde(default)]
    pub raw_usd: String,
}

/// Open order
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HlOpenOrder {
    pub coin: String,
    pub side: String,
    pub limit_px: String,
    pub sz: String,
    pub oid: u64,
    pub timestamp: i64,
    pub orig_sz: String,
    #[serde(default)]
    pub cloid: Option<String>,
}

/// Order status response
#[derive(Debug, Deserialize)]
pub struct HlOrderStatus {
    pub status: String,
    pub order: HlOrderInfo,
}

/// Order info
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HlOrderInfo {
    pub coin: String,
    pub side: String,
    pub limit_px: String,
    pub sz: String,
    pub oid: u64,
    pub timestamp: i64,
    #[serde(default)]
    pub cloid: Option<String>,
}

/// Exchange response
#[derive(Debug, Deserialize)]
pub struct ExchangeResponse {
    pub status: String,
    pub response: Option<ExchangeResponseData>,
}

/// Exchange response data
#[derive(Debug, Deserialize)]
pub struct ExchangeResponseData {
    #[serde(rename = "type")]
    pub response_type: String,
    pub data: Option<ExchangeData>,
}

/// Exchange data
#[derive(Debug, Deserialize)]
pub struct ExchangeData {
    pub statuses: Vec<String>,
}

/// EIP-712 domain for Hyperliquid
#[derive(Debug, Clone, Serialize)]
pub struct HyperliquidDomain {
    pub name: String,
    pub version: String,
    #[serde(rename = "chainId")]
    pub chain_id: u64,
    #[serde(rename = "verifyingContract")]
    pub verifying_contract: String,
}

impl Default for HyperliquidDomain {
    fn default() -> Self {
        Self {
            name: "HyperliquidSignTransaction".to_string(),
            version: "1".to_string(),
            chain_id: 42161, // Arbitrum
            verifying_contract: "0x0000000000000000000000000000000000000000".to_string(),
        }
    }
}

/// Agent type for signing
#[derive(Debug, Clone, Serialize)]
pub struct Agent {
    pub source: String,
    #[serde(rename = "connectionId")]
    pub connection_id: String,
}

/// Action types
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum HyperliquidAction {
    #[serde(rename = "order")]
    Order {
        orders: Vec<OrderWire>,
        grouping: String,
    },
    #[serde(rename = "cancel")]
    Cancel { cancels: Vec<CancelWire> },
    #[serde(rename = "cancelByCloid")]
    CancelByCloid { cancels: Vec<CancelByCloidWire> },
}

/// Order wire format
#[derive(Debug, Clone, Serialize)]
pub struct OrderWire {
    /// Asset index
    pub a: u32,
    /// Is buy
    pub b: bool,
    /// Price
    pub p: String,
    /// Size
    pub s: String,
    /// Reduce only
    pub r: bool,
    /// Order type
    pub t: serde_json::Value,
    /// Client order id
    #[serde(skip_serializing_if = "Option::is_none")]
    pub c: Option<String>,
}

/// Cancel wire format
#[derive(Debug, Clone, Serialize)]
pub struct CancelWire {
    /// Asset index
    pub a: u32,
    /// Order ID
    pub o: u64,
}

/// Cancel by cloid wire format
#[derive(Debug, Clone, Serialize)]
pub struct CancelByCloidWire {
    /// Asset index
    pub asset: u32,
    /// Client order ID
    pub cloid: String,
}
