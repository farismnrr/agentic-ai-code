use clap::Parser;
use rust_tools::relay_agent::{
    config::{Cli, Command, ServerConfig},
    pidfile::Pidfile,
    transport::create_router,
};
use std::net::SocketAddr;
use tokio::signal;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    if unsafe { libc::geteuid() } == 0 {
        return Err(
            "refusing to start as UID 0 (root). execution policy requires an unprivileged user."
                .into(),
        );
    }

    let cli = Cli::parse();

    if let Some(Command::Stop { port }) = cli.command {
        match Pidfile::stop(port) {
            Ok(_) => {
                println!("relay-agent: stopped port {port}");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("relay-agent: error stopping port {port}: {e}");
                std::process::exit(1);
            }
        }
    }

    let config = ServerConfig::from(&cli);
    config.validate().map_err(|e| e.to_string())?;

    let _pidfile = Pidfile::new(config.port)?;

    // Fail fast if the resolved working directory doesn't exist, matching
    // the legacy relay's startup behavior.
    let dir = config.resolved_dir().map_err(|e| e.to_string())?;
    if !dir.is_dir() {
        return Err(format!("working directory does not exist: {}", dir.display()).into());
    }

    let router = create_router(config.clone());

    // Localhost-only binding per the plan's security invariant #1.
    // In REMOTE mode, we bind to 0.0.0.0 (or similar) behind a reverse proxy.
    let addr = match config.mode {
        rust_tools::relay_agent::config::SecurityMode::Local => {
            SocketAddr::from(([127, 0, 0, 1], config.port))
        }
        rust_tools::relay_agent::config::SecurityMode::Remote => {
            SocketAddr::from(([0, 0, 0, 0], config.port))
        }
    };
    println!("relay-agent listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;

    // Support graceful shutdown
    let shutdown_signal = async {
        let ctrl_c = async {
            signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };

        #[cfg(unix)]
        let terminate = async {
            signal::unix::signal(signal::unix::SignalKind::terminate())
                .expect("failed to install signal handler")
                .recv()
                .await;
        };

        #[cfg(not(unix))]
        let terminate = std::future::pending::<()>();

        tokio::select! {
            _ = ctrl_c => {},
            _ = terminate => {},
        }
        println!("Shutting down gracefuly...");
    };

    axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    Ok(())
}
