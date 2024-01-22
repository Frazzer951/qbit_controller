mod config;
mod constants;
mod logger;
mod processes;

use anyhow::Result;
use config::{load_config, ControllerConfig};
use log4rs::Handle;
use processes::tag_names;
use qbit_rs::model::{Credential, GetTorrentListArg, Torrent};
use qbit_rs::Qbit;

async fn process_torrents(
    config: ControllerConfig,
    qbit: Qbit,
    torrents: Vec<Torrent>,
) -> Result<()> {
    if config.processes.tag_names {
        tag_names::process_tag_names(config, qbit, torrents).await?;
    }

    Ok(())
}

async fn run(log_handle: Handle) -> Result<()> {
    let config_path = constants::CONFIG_DIR.to_owned() + constants::CONFIG_FILE;

    let config = load_config(config_path.as_str())?;

    if config.settings.debug {
        logger::update_logger(&log_handle, log::LevelFilter::Debug)?;
    }

    log::debug!("{:#?}", config);

    let credential = Credential::new(&config.qbit.username, &config.qbit.password);
    let qbit = Qbit::new(config.qbit.url.as_str(), credential);

    let qbit_version = qbit.get_version().await?;
    log::info!("QbitTorrent Version: {qbit_version}");

    let torrents = qbit.get_torrent_list(GetTorrentListArg::default()).await?;

    process_torrents(config, qbit, torrents).await?;

    Ok(())
}

#[tokio::main]
async fn main() {
    let log_handle = logger::setup_logger().unwrap();

    log::info!("Starting qbit-controller");

    if let Err(e) = run(log_handle).await {
        log::error!("{e}");
        std::process::exit(1);
    }
}
