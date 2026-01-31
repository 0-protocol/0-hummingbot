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

**0-hummingbot** is a translation of [hummingbot/hummingbot](https://github.com/hummingbot/hummingbot) into [0-lang](https://github.com/0-protocol/0-lang)—a graph-based, machine-native programming language.

| Original | 0-hummingbot |
|----------|--------------|
| Python code optimized for human readers | Zero graphs optimized for machine execution |
| Strategy logic in text files | Strategy logic in binary DAGs |
| Runtime interpretation | Content-addressed, verifiable execution |

---

## Why Translate?

```
┌─────────────────────────────────────────────────────────────────────────┐
│                     THE TRANSLATION THESIS                              │
│                                                                         │
│   Traditional trading bots are written for humans to read and          │
│   maintain. But in the age of AI agents, bots should be written        │
│   for machines to execute, verify, and optimize.                       │
│                                                                         │
│   0-hummingbot is the first major application proving that complex     │
│   real-world systems can be expressed in machine-native form.          │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### Benefits

| Benefit | Description |
|---------|-------------|
| **Verifiable Orders** | Every order carries a cryptographic proof of strategy intent |
| **Zero Ambiguity** | Hash-referenced logic eliminates interpretation errors |
| **Agent-Native** | AI agents can read, modify, and execute strategies directly |
| **Deterministic** | Same inputs always produce same outputs |

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

Features added to 0-lang during this translation:
- [ ] HTTP External resolver
- [ ] WebSocket External resolver
- [ ] JSON parsing operations
- [ ] Decimal precision tensors
- [ ] Timer/Scheduler support

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

<div align="center">

**∅**

*Trading at machine speed, with machine precision.*

</div>
