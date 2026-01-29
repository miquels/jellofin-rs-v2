use clap::Parser;
use std::process;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long, default_value = "jellofin-server.yaml")]
    config: String,

    /// Enable debug mode
    #[arg(short, long)]
    debug: bool,
}

#[tokio::main]
async fn main() {
    // Install default crypto provider to avoid runtime panic with rustls 0.23
    let _ = rustls::crypto::ring::default_provider().install_default();

    let args = Args::parse();

    if let Err(e) = jellofin_rs::run(args.config, args.debug).await {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
