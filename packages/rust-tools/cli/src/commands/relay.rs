use relay_core::config::{Command, ServerConfig};
use relay_infrastructure::pidfile::Pidfile;
use relay_infrastructure::transport::create_router;
use std::net::SocketAddr;
use tokio::signal;

// Bubblewrap (bwrap) is a Linux-only tool. Sandboxed execution is not supported
// on other platforms. Emit a compile-time warning to make this visible.
#[cfg(not(target_os = "linux"))]
compile_error!(
    "relay-agent sandboxed execution requires Linux (bubblewrap/bwrap). \
     Build with a *-unknown-linux-gnu target only."
);

pub async fn run(cli: relay_core::config::Cli) -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    if unsafe { libc::geteuid() } == 0 {
        return Err(
            "refusing to start as UID 0 (root). execution policy requires an unprivileged user."
                .into(),
        );
    }

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

    // P0-1 / P0-8: verify bwrap is available BEFORE binding. If bwrap is missing or
    // not executable, execution would silently fail per-call without this gate.
    // We abort at startup so the operator knows immediately that the sandbox is broken.
    #[cfg(target_os = "linux")]
    {
        let bwrap_check = std::process::Command::new("bwrap")
            .arg("--version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        if bwrap_check.is_err() {
            return Err("bubblewrap (bwrap) is not available. \
                 Sandboxed execution requires bwrap to be installed. Refusing to start."
                .into());
        }
    }

    let _pidfile = Pidfile::new(config.port)?;

    // Fail fast if the resolved working directory doesn't exist, matching
    // the legacy relay's startup behavior.
    let dir = config.resolved_dir().map_err(|e| e.to_string())?;
    if !dir.is_dir() {
        return Err(format!("working directory does not exist: {}", dir.display()).into());
    }

    let router = create_router(config.clone());

    let addr: SocketAddr = format!("{}:{}", config.bind_host, config.port).parse()?;
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

    axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal)
    .await?;

    Ok(())
}
