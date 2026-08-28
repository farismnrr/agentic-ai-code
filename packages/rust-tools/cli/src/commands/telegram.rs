use relay_core::config::{ActivityConfig, ServerConfig};
use relay_infrastructure::notifications::NotificationLedger;
use std::path::PathBuf;

#[derive(clap::Args, Debug)]
pub struct Args {
    /// Owner-controlled dotenv file containing the Telegram bootstrap values.
    /// Only TELEGRAM_BOT_TOKEN and TELEGRAM_HOME_CHANNEL are read.
    #[arg(long, value_name = "PATH")]
    pub env_file: PathBuf,

    /// Relay state directory. Defaults to RELAY_ACTIVITY_STATE_DIR or the
    /// operator's XDG state directory.
    #[arg(long, env = "RELAY_ACTIVITY_STATE_DIR")]
    pub state_dir: Option<String>,
}

pub fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    let config = ServerConfig {
        activity: ActivityConfig {
            state_dir: args.state_dir,
            ..ActivityConfig::default()
        },
        ..ServerConfig::default()
    };
    let ledger =
        NotificationLedger::open(&config).map_err(|_| "failed to open relay notification state")?;
    ledger
        .import_env_credentials(&args.env_file)
        .map_err(|_| "failed to import valid Telegram credentials")?;
    println!("Telegram credentials imported into relay state");
    Ok(())
}
