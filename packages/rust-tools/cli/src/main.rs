use clap::{Parser, Subcommand};
use relay_infrastructure::telemetry::extract_ai_tools_traceparent;
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;

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
    Relay(Box<relay_core::config::Cli>),
    /// Search the web using SearxNG
    Searxng(commands::searxng::Args),
    /// Run an executable command within the workspace directory
    Terminal(commands::terminal::Args),
    /// Provision relay-owned Telegram credentials from an owner-controlled env file
    Telegram(commands::telegram::Args),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let telemetry_guard = relay_infrastructure::telemetry::init_telemetry();
    tracing::info_span!("ai_tools.startup").in_scope(|| {
        tracing::info!("ai-tools starting");
    });

    let cli = Cli::parse();

    let command_name = match &cli.command {
        Commands::Curl(_) => "curl",
        Commands::Relay(_) => "relay",
        Commands::Searxng(_) => "searxng",
        Commands::Terminal(_) => "terminal",
        Commands::Telegram(_) => "telegram",
    };
    let command_span = tracing::info_span!("ai_tools.command", command = command_name);
    let _ = command_span.set_parent(extract_ai_tools_traceparent());
    let result = async move {
        match cli.command {
            Commands::Curl(args) => {
                commands::curl::run(args).await;
                Ok(())
            }
            Commands::Relay(args) => commands::relay::run(*args).await,
            Commands::Searxng(args) => {
                commands::searxng::run(args).await;
                Ok(())
            }
            Commands::Terminal(args) => {
                commands::terminal::run(args).await;
                Ok(())
            }
            Commands::Telegram(args) => commands::telegram::run(args),
        }
    }
    .instrument(command_span)
    .await;

    relay_infrastructure::telemetry::shutdown_telemetry_default(telemetry_guard).await;

    result
}
