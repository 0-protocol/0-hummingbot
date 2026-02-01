//! 0-hummingbot - High-frequency trading bot in 0-lang
//!
//! Trading strategies as executable graphs.

use clap::{Parser, Subcommand};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

pub mod connectors;
pub mod composer;
pub mod pco;
mod resolvers;
mod runtime;
pub mod wallet;

/// 0-hummingbot: High-frequency crypto trading bot
#[derive(Parser)]
#[command(name = "0-hummingbot")]
#[command(about = "Trading strategies as executable graphs", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Execute a strategy graph once
    Execute {
        /// Path to the .0 graph file
        #[arg(value_name = "GRAPH")]
        graph: PathBuf,

        /// Enable verbose output
        #[arg(short, long)]
        verbose: bool,
    },

    /// Run a strategy continuously
    Run {
        /// Path to the strategy .0 graph file
        #[arg(value_name = "STRATEGY")]
        strategy: PathBuf,

        /// Exchange connector to use
        #[arg(short, long, default_value = "binance")]
        connector: String,

        /// Trading pair (e.g., BTC/USDT)
        #[arg(short, long)]
        pair: String,

        /// Trading mode
        #[arg(short, long, default_value = "paper")]
        mode: TradingMode,

        /// Execution interval in milliseconds
        #[arg(short, long, default_value = "1000")]
        interval: u64,
    },

    /// Inspect a graph without executing
    Inspect {
        /// Path to the .0 graph file
        #[arg(value_name = "GRAPH")]
        graph: PathBuf,
    },

    /// Verify a graph's proofs
    Verify {
        /// Path to the .0 graph file
        #[arg(value_name = "GRAPH")]
        graph: PathBuf,
    },

    /// List available strategies
    ListStrategies,

    /// List available connectors
    ListConnectors,
}

#[derive(Clone, Debug, Default)]
enum TradingMode {
    #[default]
    Paper,
    Live,
}

impl std::str::FromStr for TradingMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "paper" => Ok(TradingMode::Paper),
            "live" => Ok(TradingMode::Live),
            _ => Err(format!("Unknown trading mode: {}", s)),
        }
    }
}

fn main() {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set subscriber");

    let cli = Cli::parse();

    match cli.command {
        Commands::Execute { graph, verbose } => {
            info!("Executing graph: {:?}", graph);
            if verbose {
                info!("Verbose mode enabled");
            }
            execute_graph(&graph, verbose);
        }
        Commands::Run {
            strategy,
            connector,
            pair,
            mode,
            interval,
        } => {
            info!(
                "Running strategy: {:?} on {} with pair {} in {:?} mode",
                strategy, connector, pair, mode
            );
            run_strategy(&strategy, &connector, &pair, mode, interval);
        }
        Commands::Inspect { graph } => {
            info!("Inspecting graph: {:?}", graph);
            inspect_graph(&graph);
        }
        Commands::Verify { graph } => {
            info!("Verifying graph: {:?}", graph);
            verify_graph(&graph);
        }
        Commands::ListStrategies => {
            list_strategies();
        }
        Commands::ListConnectors => {
            list_connectors();
        }
    }
}

fn execute_graph(path: &PathBuf, _verbose: bool) {
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│  EXECUTE GRAPH                                              │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│  Path: {:?}", path);
    println!("│  Status: Not yet implemented                                │");
    println!("│                                                             │");
    println!("│  This feature requires:                                     │");
    println!("│  - Graph loading from .0 files                              │");
    println!("│  - 0-VM execution                                           │");
    println!("│  - External resolver for HTTP/WS                            │");
    println!("└─────────────────────────────────────────────────────────────┘");
}

fn run_strategy(
    _path: &PathBuf,
    _connector: &str,
    _pair: &str,
    _mode: TradingMode,
    _interval: u64,
) {
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│  RUN STRATEGY                                               │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│  Status: Not yet implemented                                │");
    println!("│                                                             │");
    println!("│  Coming soon:                                               │");
    println!("│  - Continuous execution loop                                │");
    println!("│  - Real-time market data                                    │");
    println!("│  - Order placement                                          │");
    println!("└─────────────────────────────────────────────────────────────┘");
}

fn inspect_graph(path: &PathBuf) {
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│  INSPECT GRAPH                                              │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│  Path: {:?}", path);
    println!("│  Status: Not yet implemented                                │");
    println!("└─────────────────────────────────────────────────────────────┘");
}

fn verify_graph(path: &PathBuf) {
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│  VERIFY GRAPH                                               │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│  Path: {:?}", path);
    println!("│  Status: Not yet implemented                                │");
    println!("└─────────────────────────────────────────────────────────────┘");
}

fn list_strategies() {
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│  AVAILABLE STRATEGIES                                       │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│                                                             │");
    println!("│  Market Making:                                             │");
    println!("│  ├─ pure_mm.0        │ Basic spread-based market making     │");
    println!("│  ├─ avellaneda_mm.0  │ Optimal MM (Avellaneda-Stoikov)     │");
    println!("│  └─ grid_mm.0        │ Grid trading strategy                │");
    println!("│                                                             │");
    println!("│  Arbitrage:                                                 │");
    println!("│  └─ cross_arb.0      │ Cross-exchange arbitrage             │");
    println!("│                                                             │");
    println!("│  Execution:                                                 │");
    println!("│  ├─ twap.0           │ Time-weighted average price          │");
    println!("│  ├─ vwap.0           │ Volume-weighted average price        │");
    println!("│  └─ iceberg.0        │ Hidden large order execution         │");
    println!("│                                                             │");
    println!("│  Components:                                                │");
    println!("│  ├─ risk_check.0     │ Reusable risk validation             │");
    println!("│  ├─ position_mgr.0   │ Position & P&L tracking              │");
    println!("│  └─ order_exec.0     │ Order execution handler              │");
    println!("│                                                             │");
    println!("└─────────────────────────────────────────────────────────────┘");
}

fn list_connectors() {
    println!("┌─────────────────────────────────────────────────────────────┐");
    println!("│  AVAILABLE CONNECTORS                                       │");
    println!("├─────────────────────────────────────────────────────────────┤");
    println!("│                                                             │");
    println!("│  Exchange     │ Type │ Chain   │ Status      │ Module      │");
    println!("│  ─────────────┼──────┼─────────┼─────────────┼──────────── │");
    println!("│  binance      │ CEX  │ N/A     │ In Progress │ resolvers   │");
    println!("│  hyperliquid  │ DEX  │ Arbitrum│ Ready       │ connectors  │");
    println!("│  dydx         │ DEX  │ Cosmos  │ Ready       │ connectors  │");
    println!("│  jupiter      │ DEX  │ Solana  │ Ready       │ connectors  │");
    println!("│  okx          │ CEX  │ N/A     │ Planned     │ connectors  │");
    println!("│                                                             │");
    println!("└─────────────────────────────────────────────────────────────┘");
}
