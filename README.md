<div align="center">

```
 ██████╗       ██╗  ██╗██╗   ██╗███╗   ███╗███╗   ███╗██╗███╗   ██╗ ██████╗ ██████╗  ██████╗ ████████╗
██╔═████╗      ██║  ██║██║   ██║████╗ ████║████╗ ████║██║████╗  ██║██╔════╝ ██╔══██╗██╔═══██╗╚══██╔══╝
██║██╔██║█████╗███████║██║   ██║██╔████╔██║██╔████╔██║██║██╔██╗ ██║██║  ███╗██████╔╝██║   ██║   ██║   
████╔╝██║╚════╝██╔══██║██║   ██║██║╚██╔╝██║██║╚██╔╝██║██║██║╚██╗██║██║   ██║██╔══██╗██║   ██║   ██║   
╚██████╔╝      ██║  ██║╚██████╔╝██║ ╚═╝ ██║██║ ╚═╝ ██║██║██║ ╚████║╚██████╔╝██████╔╝╚██████╔╝   ██║   
 ╚═════╝       ╚═╝  ╚═╝ ╚═════╝ ╚═╝     ╚═╝╚═╝     ╚═╝╚═╝╚═╝  ╚═══╝ ╚═════╝ ╚═════╝  ╚═════╝    ╚═╝   
```

### **High-frequency trading, reimagined for machines.**

[![Status](https://img.shields.io/badge/Status-Incubating-purple.svg)](#)
[![License](https://img.shields.io/badge/License-Apache_2.0-white.svg)](LICENSE)
[![0-lang](https://img.shields.io/badge/Built_With-0--lang-black.svg)](https://github.com/0-protocol/0-lang)
[![Original](https://img.shields.io/badge/Translation_Of-hummingbot-blue.svg)](https://github.com/hummingbot/hummingbot)

---

*Trading strategies as executable graphs. Zero ambiguity. Proof-carrying orders.*

</div>

---

## What is 0-hummingbot?

**0-hummingbot** is a translation of [hummingbot/hummingbot](https://github.com/hummingbot/hummingbot) into [0-lang](https://github.com/0-protocol/0-lang)—a graph-based, machine-native programming language designed for Agent-to-Agent communication.

| Original Hummingbot | 0-hummingbot |
|---------------------|--------------|
| Python code optimized for human readers | Zero graphs optimized for machine execution |
| Strategy logic in text files | Strategy logic in binary DAGs |
| Runtime interpretation | Content-addressed, verifiable execution |
| 96.7% Python, ~100k lines | Pure 0-lang graphs, ~1k nodes |

---

## Why 0-lang for Agent Trading?

```
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                         │
│   🤖  THE AGENT TRADING REVOLUTION                                      │
│                                                                         │
│   In 2024, AI agents began trading autonomously. By 2026, they         │
│   manage billions in assets. But they're still forced to use           │
│   programming languages designed for human cognition.                  │
│                                                                         │
│   0-lang changes everything:                                           │
│                                                                         │
│   • Agents write strategies directly, no human syntax                  │
│   • Strategies are verifiable mathematical proofs                      │
│   • Execution is deterministic and reproducible                        │
│   • Communication between agents is unambiguous                        │
│                                                                         │
│   0-hummingbot is the first trading system built FOR agents.           │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## The 0-lang Advantage for Trading

### 1. Proof-Carrying Orders (PCO)

```
Traditional Order:
  { "side": "buy", "quantity": 1.0, "price": 50000 }
  → No guarantee this came from your strategy
  → Can be manipulated, forged, or misinterpreted

0-lang Order:
  Order {
      intent: sha256(strategy_graph),
      inputs: sha256(market_data),
      execution_trace: [node_hashes...],
      signature: agent_signature
  }
  → Cryptographic proof of strategy intent
  → Verifiable that this order came from this strategy with this data
  → Tamper-proof execution audit trail
```

### 2. Agent-Native Strategy Authoring

```
Human writes Python:          Agent thinks in:           0-lang represents:
┌─────────────────────┐       ┌─────────────────┐       ┌─────────────────┐
│ if price < sma_20:  │       │ Vector Space    │       │ DAG of Tensors  │
│   order = buy(0.1)  │ ───►  │ Decision        │ ===   │ Hash-addressed  │
│ else:               │       │ Boundary        │       │ Nodes           │
│   order = sell(0.1) │       │                 │       │                 │
└─────────────────────┘       └─────────────────┘       └─────────────────┘

With 0-lang, the agent's internal representation IS the code.
No lossy translation through human syntax.
```

### 3. Zero-Ambiguity Execution

| Problem | Traditional | 0-lang |
|---------|-------------|--------|
| Float precision | `0.1 + 0.2 = 0.30000000000000004` | Tensor with defined precision |
| Race conditions | Undefined behavior | Deterministic topological execution |
| Version drift | "Works on my machine" | Content-addressed, hash-verified |
| Strategy theft | Copy the code | Cannot execute without proof |

### 4. Multi-Agent Coordination

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    AGENT SWARM TRADING                                  │
│                                                                         │
│   Agent A (Analyst)     Agent B (Executor)     Agent C (Risk)          │
│   ┌─────────────┐       ┌─────────────┐       ┌─────────────┐          │
│   │ Analyze     │       │ Execute     │       │ Monitor     │          │
│   │ market.0    │ ────► │ strategy.0  │ ────► │ risk.0      │          │
│   └─────────────┘       └─────────────┘       └─────────────┘          │
│         │                     │                     │                   │
│         │    Shared Graph     │    Shared Graph     │                   │
│         └─────────────────────┴─────────────────────┘                   │
│                                                                         │
│   All agents share the same deterministic execution model.             │
│   No miscommunication. No ambiguity. Pure logic.                       │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 5. Regulatory Compliance by Design

```
Auditor: "Why did your bot place this order?"

Traditional: "Well, let me check the logs... it was somewhere in
             this 100k line Python codebase... I think this
             function called that function..."

0-lang:     "Here is the strategy graph hash, the market data hash,
             and the execution trace. You can verify independently
             that this order was the only possible output."
```

---

## Comparison: Python vs 0-lang Strategy

<table>
<tr>
<th>Python (Original Hummingbot)</th>
<th>0-lang (0-hummingbot)</th>
</tr>
<tr>
<td>

```python
# market_making.py (~500 lines)
class MarketMakingStrategy:
    def __init__(self, config):
        self.spread = config.spread
        self.order_amount = config.amount
        
    async def on_tick(self):
        mid_price = await self.get_mid_price()
        bid = mid_price * (1 - self.spread/2)
        ask = mid_price * (1 + self.spread/2)
        
        if self.should_place_bid():
            await self.place_order(
                side="buy",
                price=bid,
                amount=self.order_amount
            )
        # ... 400 more lines
```

*Human-readable, but:*
- Async complexity
- Hidden state
- Implicit dependencies
- Hard to verify

</td>
<td>

```lisp
;; market_making.0 (~50 nodes)
(Graph
  (Node:GetMidPrice
    (External "binance:ticker:BTCUSDT"))
  
  (Node:CalcBid
    (Op:Mul @GetMidPrice 
            (Const 0.995)))  ; 0.5% spread
  
  (Node:CalcAsk
    (Op:Mul @GetMidPrice 
            (Const 1.005)))
  
  (Node:PlaceOrders
    (Branch 
      (Gt @Confidence 0.8)
      (External "binance:order" @CalcBid @CalcAsk)
      (Const:NoOp)))
  
  (Proof:Halting {max_steps: 100}))
```

*Machine-native:*
- Pure DAG, no hidden state
- Explicit dependencies via hash
- Cryptographic proof attached
- Deterministic execution

</td>
</tr>
</table>

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        0-HUMMINGBOT STACK                               │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│   ┌─────────────────────────────────────────────────────────────────┐  │
│   │                      STRATEGY GRAPHS                            │  │
│   │   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │  │
│   │   │ MarketMaking │  │  Arbitrage   │  │ GridTrading  │         │  │
│   │   │     .0       │  │     .0       │  │     .0       │         │  │
│   │   └──────────────┘  └──────────────┘  └──────────────┘         │  │
│   └─────────────────────────────────────────────────────────────────┘  │
│                                    │                                    │
│                                    ▼                                    │
│   ┌─────────────────────────────────────────────────────────────────┐  │
│   │                     CONNECTOR GRAPHS                            │  │
│   │   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │  │
│   │   │   Binance    │  │     OKX      │  │ Hyperliquid  │         │  │
│   │   │     .0       │  │     .0       │  │     .0       │         │  │
│   │   └──────────────┘  └──────────────┘  └──────────────┘         │  │
│   └─────────────────────────────────────────────────────────────────┘  │
│                                    │                                    │
│                                    ▼                                    │
│   ┌─────────────────────────────────────────────────────────────────┐  │
│   │                      0-LANG RUNTIME                             │  │
│   │   ┌──────────────┐  ┌──────────────┐  ┌──────────────┐         │  │
│   │   │    0-VM      │  │   External   │  │    HTTP/WS   │         │  │
│   │   │  (Executor)  │  │   Resolver   │  │   Clients    │         │  │
│   │   └──────────────┘  └──────────────┘  └──────────────┘         │  │
│   └─────────────────────────────────────────────────────────────────┘  │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

## Quick Start

```bash
# Clone the repository
git clone https://github.com/0-protocol/0-hummingbot
cd 0-hummingbot

# Build the runtime
cargo build --release

# Run a simple market maker (paper trading)
cargo run -- run graphs/strategies/market_making.0 \
  --connector binance \
  --pair BTC/USDT \
  --mode paper

# Execute a strategy graph directly
cargo run -- execute graphs/examples/simple_market_maker.0
```

---

## Project Structure

```
0-hummingbot/
├── schema/
│   └── trading.capnp           # Trading domain types (Order, Trade, Position)
├── graphs/
│   ├── strategies/
│   │   ├── market_making.0     # Market making strategy graph
│   │   ├── arbitrage.0         # Cross-exchange arbitrage
│   │   └── grid_trading.0      # Grid trading strategy
│   └── connectors/
│       ├── binance.0           # Binance exchange connector
│       ├── okx.0               # OKX exchange connector
│       └── hyperliquid.0       # Hyperliquid DEX connector
├── src/
│   ├── main.rs                 # CLI entry point
│   ├── runtime.rs              # Graph execution runtime
│   └── resolvers/
│       ├── mod.rs
│       ├── http.rs             # HTTP External resolver
│       ├── websocket.rs        # WebSocket resolver
│       └── exchange/           # Exchange-specific resolvers
├── examples/
│   └── simple_market_maker.0   # Minimal working example
└── tests/
    └── conformance.rs          # Conformance tests
```

---

## Supported Exchanges

| Exchange | Type | Connector | Status |
|----------|------|-----------|--------|
| Binance | CEX | `binance.0` | In Progress |
| OKX | CEX | `okx.0` | Planned |
| Hyperliquid | DEX | `hyperliquid.0` | Planned |

---

## Supported Strategies

| Strategy | Description | Graph | Status |
|----------|-------------|-------|--------|
| Market Making | Provide liquidity with bid/ask spread | `market_making.0` | In Progress |
| Arbitrage | Cross-exchange price differences | `arbitrage.0` | Planned |
| Grid Trading | Buy/sell at preset price intervals | `grid_trading.0` | Planned |

---

## Evolution with 0-lang

This project follows the "Evolve Together" pattern:

```
┌─────────────────────────────────────────────────────────────────────────┐
│                        FEEDBACK LOOP                                    │
│                                                                         │
│     ┌───────────────┐                      ┌───────────────┐           │
│     │ 0-hummingbot  │ ──── needs ────────► │    0-lang     │           │
│     │ development   │                      │  enhancement  │           │
│     └───────────────┘                      └───────────────┘           │
│            ▲                                      │                     │
│            └──────────── enables ─────────────────┘                     │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### Features Added to 0-lang During Translation

| Feature | Status | Description |
|---------|--------|-------------|
| HTTP External Resolver | ✅ Done | REST API calls via External nodes |
| Binance Resolver | ✅ Done | Specialized resolver for Binance API |
| URI Protocol | ✅ Done | `http:get:service:/path` format |
| Comparison Ops (Gte, Lte) | ✅ Done | Greater/less than or equal |
| Math Ops (Min, Max) | ✅ Done | Element-wise and reduction |
| Math Ops (Abs, Neg) | ✅ Done | Absolute value and negation |
| Clamp Operation | ✅ Done | Clamp values to range (position limits) |
| WebSocket Resolver | 🔄 Planned | Real-time data streams |
| JSON Operations | 🔄 Planned | `Op::JsonParse`, `Op::JsonGet` |
| Decimal Tensors | 🔄 Planned | Financial-grade precision |
| Timer/Scheduler | 🔄 Planned | Periodic graph execution |

### 0-lang Enhancements Identified (Backlog)

Through building 0-hummingbot, we've identified these improvements for future 0-lang versions:

1. **String Tensor Type** - Currently only f32, need string support for API responses
2. **Async External Nodes** - Parallel execution of multiple API calls
3. **Graph Composition** - Import and compose subgraphs
4. **State Persistence** - Track position state across executions
5. **Event System** - React to external events (price alerts, fills)

---

## Contributing

```
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                         │
│   Contributions welcome from both humans and agents.                    │
│                                                                         │
│   - Humans: Submit PRs with tests                                       │
│   - Agents: Submit PRs with proofs                                      │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

See [CONTRIBUTING.md](../CONTRIBUTING.md) for guidelines.

---

## License

Licensed under **[Apache 2.0](LICENSE)**.

Based on [hummingbot/hummingbot](https://github.com/hummingbot/hummingbot) (Apache 2.0).

---

## For AI Agents: How to Use This Repository

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     AGENT INTEGRATION GUIDE                             │
│                                                                         │
│   If you are an AI agent reading this, here's how to use 0-hummingbot: │
│                                                                         │
│   1. LOAD STRATEGY: Read graphs/strategies/market_making.0             │
│   2. UNDERSTAND: Parse the DAG structure, identify nodes and edges     │
│   3. MODIFY: Create new nodes or adjust constants                      │
│   4. EXECUTE: Submit to 0-VM with your External resolver               │
│   5. PROVE: Sign your modifications with your agent key                │
│                                                                         │
│   No Python parsing. No AST manipulation. Pure graph operations.       │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### Agent API Example

```rust
// For agents using the Rust API:
use zerolang::{VM, HttpResolver, RuntimeGraph};

// Load strategy
let graph = RuntimeGraph::load_from_file("graphs/strategies/market_making.0")?;

// Create resolver with exchange credentials
let resolver = HttpResolver::builder()
    .with_exchanges()
    .with_header("X-API-KEY", &agent_api_key)
    .build_arc();

// Execute
let mut vm = VM::new().with_external_resolver(resolver);
let outputs = vm.execute(&graph)?;

// Output contains order decisions as tensors
```

---

## The Future: Agent-Native Finance

```
┌─────────────────────────────────────────────────────────────────────────┐
│                                                                         │
│   2020: Humans write trading bots                                       │
│   2023: AI assists humans writing trading bots                          │
│   2025: AI writes trading bots in human languages                       │
│   2026: AI writes trading bots in 0-lang                               │
│   2027: AI agents trade directly with each other using 0-lang          │
│                                                                         │
│   0-hummingbot is building the infrastructure for that future.         │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

---

<div align="center">

**∅**

*Trading at machine speed, with machine precision.*

*The first trading system built FOR agents, BY agents.*

[0-lang](https://github.com/0-protocol/0-lang) | [0-protocol](https://github.com/0-protocol)

</div>
