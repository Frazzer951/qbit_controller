mod config;
mod constants;

use anyhow::Result;
use config::load_config;
use qbit_rs::model::Credential;
use qbit_rs::Qbit;

async fn run() -> Result<()> {
    let config_path = constants::CONFIG_DIR.to_owned() + constants::CONFIG_FILE;

    let config = load_config(config_path.as_str())?;

    println!("{:#?}", config);

    let credential = Credential::new(config.qbit.username, config.qbit.password);
    let api = Qbit::new(config.qbit.url.as_str(), credential);
    let torrents = api.get_version().await?;

    println!("{torrents}");

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
