#[tokio::main]
async fn main() {
    if let Err(error) = relay_agent::bootstrap::run().await {
        eprintln!("relay-agent stopped: {error}");
        std::process::exit(1);
    }
}
