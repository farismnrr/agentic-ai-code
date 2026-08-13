use clap::{Parser, Subcommand};

mod commands;

#[derive(Parser, Debug)]
#[command(name = "ai-tools")]
#[command(version, about = "Unified CLI for MasihAwam AI Tools", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Fetch a URL and return its response
    Curl(commands::curl::Args),
    /// Run the relay agent
    RelayAgent(relay_core::config::Cli),
    /// Search the web using SearxNG
    Searxng(commands::searxng::Args),
    /// Run an executable command within the workspace directory
    Terminal(commands::terminal::Args),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Curl(args) => {
            commands::curl::run(args).await;
            Ok(())
        }
        Commands::RelayAgent(args) => commands::relay::run(args).await,
        Commands::Searxng(args) => {
            commands::searxng::run(args).await;
            Ok(())
        }
        Commands::Terminal(args) => {
            commands::terminal::run(args).await;
            Ok(())
        }
    }
}
