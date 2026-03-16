# ARCHITECTURE.md

> Technical specification for 0-hummingbot.

---

## System Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          0-HUMMINGBOT SYSTEM                                │
└─────────────────────────────────────────────────────────────────────────────┘
                                    │
          ┌─────────────────────────┼─────────────────────────┐
          │                         │                         │
          ▼                         ▼                         ▼
  ┌───────────────┐         ┌───────────────┐         ┌───────────────┐
  │   Strategies  │         │  Connectors   │         │   Runtime     │
  │   (Graphs)    │         │   (Graphs)    │         │   (Rust)      │
  └───────────────┘         └───────────────┘         └───────────────┘
          │                         │                         │
          ▼                         ▼                         ▼
  Trading Logic              Exchange APIs            Graph Execution
```

---

## Core Components

### 1. Strategy Graphs

Trading strategies are expressed as 0-lang DAGs:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     MARKET MAKING STRATEGY GRAPH                            │
│                                                                             │
│   ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐                │
│   │ GetMid  │ →  │ CalcBid │ →  │ Branch  │ →  │ PlaceOrd│                │
│   │ Price   │    │ /Ask    │    │ (conf>  │    │ (if yes)│                │
│   │         │    │ Spread  │    │  0.8)   │    │         │                │
│   └─────────┘    └─────────┘    └─────────┘    └─────────┘                │
│                                      │                                      │
│                                      ▼                                      │
│                                 ┌─────────┐                                │
│                                 │  Skip   │                                │
│                                 │(if no)  │                                │
│                                 └─────────┘                                │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 2. Connector Graphs

Exchange connectors handle API communication:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      BINANCE CONNECTOR GRAPH                                │
│                                                                             │
│   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐                    │
│   │  External   │ →  │  JsonParse  │ →  │  Transform  │                    │
│   │  HTTP GET   │    │  Response   │    │  to Tensor  │                    │
│   │ /api/ticker │    │             │    │             │                    │
│   └─────────────┘    └─────────────┘    └─────────────┘                    │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### 3. Runtime Layer

The Rust runtime provides:

- **External Resolvers**: Bridge between 0-lang graphs and real APIs
- **HTTP Client**: REST API communication
- **WebSocket Client**: Real-time data streams
- **Execution Loop**: Continuous strategy execution

---

## Data Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           DATA FLOW                                         │
│                                                                             │
│   Exchange ──► WebSocket ──► External Resolver ──► Graph ──► Strategy      │
│      │                                                           │          │
│      │                                                           ▼          │
│      │                                                     ┌─────────┐      │
│      │                                                     │ Decision│      │
│      │                                                     │ Tensor  │      │
│      │                                                     └────┬────┘      │
│      │                                                          │           │
│      │◄──────── HTTP POST ◄──── External Resolver ◄──── Order ◄─┘           │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Schema Design

### Order Type

```capnp
struct Order {
    id @0 :Data;              # Content-addressed order ID
    symbol @1 :Text;          # Trading pair (e.g., "BTC/USDT")
    side @2 :OrderSide;       # Buy or Sell
    type @3 :OrderType;       # Market, Limit, etc.
    quantity @4 :Tensor;      # Amount as tensor
    price @5 :Tensor;         # Price as tensor (for limit orders)
    confidence @6 :Float32;   # Strategy confidence in this order
    proof @7 :Proof;          # Proof of strategy intent
}
```

### Position Type

```capnp
struct Position {
    symbol @0 :Text;
    side @1 :PositionSide;
    quantity @2 :Tensor;
    entryPrice @3 :Tensor;
    unrealizedPnl @4 :Tensor;
    timestamp @5 :UInt64;
}
```

### Trade Type

```capnp
struct Trade {
    orderId @0 :Data;
    symbol @1 :Text;
    side @2 :OrderSide;
    quantity @3 :Tensor;
    price @4 :Tensor;
    fee @5 :Tensor;
    timestamp @6 :UInt64;
}
```

---

## External Resolver Interface

### HTTP Resolver

```rust
pub struct HttpResolver {
    client: reqwest::Client,
    base_urls: HashMap<String, String>,
}

impl ExternalResolver for HttpResolver {
    fn resolve(&self, uri: &str, inputs: Vec<&Tensor>) -> Result<Tensor, String> {
        // URI format: "http:get:binance:/api/v3/ticker/price?symbol=BTCUSDT"
        // Parse URI, make request, return response as tensor
    }
}
```

### WebSocket Resolver

```rust
pub struct WebSocketResolver {
    connections: HashMap<String, WebSocketStream>,
}

impl ExternalResolver for WebSocketResolver {
    fn resolve(&self, uri: &str, inputs: Vec<&Tensor>) -> Result<Tensor, String> {
        // URI format: "ws:binance:ticker:BTCUSDT"
        // Return latest data from stream as tensor
    }
}
```

---

## Execution Model

### Single Strategy Execution

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                      EXECUTION LOOP                                         │
│                                                                             │
│   LOOP:                                                                     │
│     1. Fetch market data (via External HTTP/WS)                            │
│     2. Load strategy graph                                                  │
│     3. Execute graph in 0-VM                                               │
│     4. Extract decision tensor                                              │
│     5. If confidence > threshold:                                           │
│        - Generate order with proof                                          │
│        - Submit via External HTTP                                           │
│     6. Sleep(interval)                                                      │
│     7. GOTO LOOP                                                            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Multi-Strategy Execution

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    MULTI-STRATEGY EXECUTION                                 │
│                                                                             │
│     Strategy A ─────┐                                                       │
│                     │                                                       │
│     Strategy B ─────┼──► Aggregator ──► Risk Manager ──► Order Router      │
│                     │                                                       │
│     Strategy C ─────┘                                                       │
│                                                                             │
│   Each strategy produces a decision tensor.                                │
│   Aggregator combines with confidence weighting.                           │
│   Risk manager applies position limits.                                    │
│   Order router submits to appropriate exchange.                            │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Proof System for Orders

Every order includes a proof of strategy intent:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        ORDER PROOF                                          │
│                                                                             │
│   Order {                                                                   │
│       ...                                                                   │
│       proof: Proof {                                                        │
│           strategy_hash: sha256(strategy_graph),  # Which strategy         │
│           input_hash: sha256(market_data),        # What data it saw       │
│           execution_trace: [...],                  # How it decided        │
│           signature: agent_signature,              # Who executed          │
│       }                                                                     │
│   }                                                                         │
│                                                                             │
│   This allows:                                                              │
│   - Auditing: Verify why an order was placed                               │
│   - Debugging: Reproduce decision with same inputs                         │
│   - Compliance: Prove algorithmic intent                                   │
│                                                                             │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Security Considerations

| Risk | Mitigation |
|------|------------|
| API key exposure | Store in env vars, never in graphs |
| Order manipulation | Proof-carrying orders verify intent |
| Malicious graphs | Verify graph hashes before execution |
| Rate limiting | External resolver handles throttling |
| Financial loss | Paper trading mode for testing |

---

## Future Extensions

1. **Multi-Agent Swarm**: Multiple agents coordinate strategies
2. **On-Chain Execution**: DEX orders as 0-lang graphs
3. **ML Integration**: Neural network strategies as graphs
4. **Backtesting**: Historical data replay through graphs

---

<div align="center">

**∅**

*Architecture for algorithmic trading, by machines.*

</div>
