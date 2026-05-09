use anyhow::Result;

pub fn setup_logger() -> Result<()> {
    log4rs::init_file("log_config.yml", Default::default())
}

#[tokio::main]
async fn main() {
    setup_logger().unwrap();

    if let Err(e) = qbit_controller::run().await {
        log::error!("{e}");
        std::process::exit(1);
    }
}
