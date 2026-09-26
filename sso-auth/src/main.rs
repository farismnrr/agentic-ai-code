mod application;
mod bootstrap;
mod domain;
mod infrastructure;
mod interfaces;

#[cfg(test)]
mod tests;

#[tokio::main]
async fn main() {
    if let Err(error) = bootstrap::run().await {
        tracing::error!(%error, "sso-auth stopped");
        std::process::exit(1);
    }
}
