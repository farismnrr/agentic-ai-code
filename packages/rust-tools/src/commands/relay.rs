#[cfg(target_os = "linux")]
use ai_tools::core::config::{Command, ServerConfig};
#[cfg(target_os = "linux")]
use ai_tools::infrastructure::pidfile::Pidfile;
#[cfg(target_os = "linux")]
use ai_tools::infrastructure::transport::create_router_with_jobs_and_hooks;
#[cfg(target_os = "linux")]
use std::net::SocketAddr;
#[cfg(target_os = "linux")]
use std::sync::Arc;
#[cfg(target_os = "linux")]
use tokio::signal;

// Bubblewrap (bwrap) is a Linux-only tool. The portable binaries retain the
// relay command in their CLI surface for discoverability, but fail explicitly
// at runtime rather than making the whole binary impossible to build.
#[cfg(not(target_os = "linux"))]
pub async fn run(_cli: ai_tools::core::config::Cli) -> Result<(), Box<dyn std::error::Error>> {
    Err("relay-agent sandboxed execution requires Linux (bubblewrap/bwrap)".into())
}

#[cfg(target_os = "linux")]
pub async fn run(cli: ai_tools::core::config::Cli) -> Result<(), Box<dyn std::error::Error>> {
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
            Err(_) => {
                eprintln!("relay-agent: failed to stop relay");
                std::process::exit(1);
            }
        }
    }

    let config = ServerConfig::from(&cli);
    config
        .validate()
        .map_err(|_| "invalid relay configuration")?;

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
    let dir = config
        .resolved_dir()
        .map_err(|_| "failed to resolve relay working directory")?;
    if !dir.is_dir() {
        return Err("relay working directory does not exist".into());
    }

    let jobs = ai_tools::application::execution::JobManager::new(config.clone());
    let hooks = ai_tools::application::hooks::HookManager::load(Arc::new(config.clone()))
        .map_err(|error| format!("agent hook configuration failed closed: {error}"))?;
    let router = create_router_with_jobs_and_hooks(config.clone(), jobs.clone(), hooks);

    let addr: SocketAddr = format!("{}:{}", config.bind_host, config.port)
        .parse()
        .map_err(|_| "invalid relay bind address")?;
    println!("relay-agent listening");

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .map_err(|_| "failed to bind relay listener")?;

    // Support graceful shutdown and cancel all running/queued execution jobs
    // before Axum waits for in-flight requests to drain.
    let shutdown_jobs = jobs.clone();
    let shutdown_signal = async move {
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
        shutdown_jobs.shutdown().await;
        println!("Shutting down gracefully...");
    };

    let server_result = axum::serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal)
    .await;

    jobs.shutdown().await;
    server_result.map_err(|_| "relay server failed")?;
    Ok(())
}
