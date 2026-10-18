mod monitor;
mod server;

use anyhow::Context;
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "gpu-monitor", about = "Realtime NVIDIA GPU web monitor")]
struct Args {
    /// HTTP listen port
    #[arg(long, default_value_t = 8080)]
    port: u16,

    /// Sampling interval in milliseconds
    #[arg(long, default_value_t = 1000)]
    interval: u64,

    /// Rolling window size in seconds (accepted for parity with frontend;
    /// the client keeps its own window)
    #[arg(long, default_value_t = 300)]
    window: u64,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let rt = tokio::runtime::Runtime::new().context("failed to start tokio runtime")?;
    rt.block_on(server::run(args))
}
