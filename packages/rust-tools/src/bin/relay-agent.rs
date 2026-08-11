use clap::Parser;
use rust_tools::relay_agent::{
    config::{Cli, Command, ServerConfig},
    transport::create_router,
};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    if let Some(Command::Stop { port }) = cli.command {
        // Pidfile-based stop lifecycle is out of scope for Phase 1/2 (see
        // Plan 028 "PID / lifecycle" section, deferred to Phase 5). This is
        // a placeholder that keeps the CLI surface stable without silently
        // pretending to have stopped anything.
        eprintln!("relay-agent: `stop --port {port}` is not implemented yet (Plan 028 Phase 5).");
        std::process::exit(1);
    }

    let config = ServerConfig::from(&cli);
    config.validate().map_err(|e| e.to_string())?;

    // Fail fast if the resolved working directory doesn't exist, matching
    // the legacy relay's startup behavior.
    let dir = config.resolved_dir().map_err(|e| e.to_string())?;
    if !dir.is_dir() {
        return Err(format!("working directory does not exist: {}", dir.display()).into());
    }

    let router = create_router(config.clone());

    // Localhost-only binding per the plan's security invariant #1.
    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
    println!("relay-agent listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, router).await?;

    Ok(())
}
