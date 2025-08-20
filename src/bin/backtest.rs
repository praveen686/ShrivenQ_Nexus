use anyhow::Result;
use clap::Parser;
use tracing::info;

#[derive(Parser)]
#[command(name = "shriven-backtest")]
#[command(about = "ShrivenQ Backtesting Engine")]
struct Args {
    /// Start date for backtesting (YYYY-MM-DD)
    #[arg(long)]
    start_date: String,

    /// End date for backtesting (YYYY-MM-DD)
    #[arg(long)]
    end_date: String,

    /// Data source for historical data
    #[arg(long, default_value = "binance")]
    data_source: String,

    /// Strategy configuration file
    #[arg(long, default_value = "config/strategy.toml")]
    strategy: String,

    /// Number of parallel workers
    #[arg(long, default_value = "4")]
    workers: usize,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().init();

    let args = Args::parse();

    info!("🔄 ShrivenQ Backtesting Engine");
    info!("├─ Period: {} to {}", args.start_date, args.end_date);
    info!("├─ Data Source: {}", args.data_source);
    info!("├─ Strategy: {}", args.strategy);
    info!("└─ Workers: {}", args.workers);

    // TODO: Implement backtesting logic
    info!("Backtesting not yet implemented");

    Ok(())
}
