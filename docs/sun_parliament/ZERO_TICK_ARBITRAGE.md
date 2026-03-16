# 0-hummingbot: Zero-Tick Arbitrage & Regime Mutation (Phase 27)

## Core Architecture Upgrade
- **Deprecated:** Legacy Python `while True` tick-based polling loops.
- **Phase 26 Temporal Anchors (`Op::SleepUntil`):** Agent state is frozen in a ZK-hash and only thaws when a specific price threshold or mempool event is cryptographically proven via `stream<tensor>`. Zero compute wasted on polling.
- **Phase 21 Liquid AST (`explore/fallback`):** Dynamic Market Making (DMM). If market volatility spikes and orders revert, the AST mutates its own inventory skew and spread formulas via genetic recombination.
- **Phase 22 Intent Invariance (`Op::VerifyInvariant`):** Hardcoded max-drawdown and risk limits. The mutated trading logic is mathematically constrained from liquidating the portfolio.
