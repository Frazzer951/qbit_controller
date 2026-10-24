use anyhow::Result;

pub fn setup_logger() -> Result<()> {
    log4rs::init_file("log_config.yml", Default::default())
}

#[tokio::main]
async fn main() {
    setup_logger().unwrap();

    let result = match parse_mode() {
        Ok(RunMode::Normal) => qbit_controller::run().await,
        Ok(RunMode::OrphanCheck) => qbit_controller::run_orphan_check().await,
        Ok(RunMode::Help) => {
            print_help();
            Ok(())
        }
        Err(e) => Err(e),
    };

    if let Err(e) = result {
        log::error!("{e}");
        std::process::exit(1);
    }
}

enum RunMode {
    Normal,
    OrphanCheck,
    Help,
}

fn parse_mode() -> Result<RunMode> {
    let mut mode = RunMode::Normal;

    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--orphan-check" => mode = RunMode::OrphanCheck,
            "-h" | "--help" => mode = RunMode::Help,
            _ => anyhow::bail!("Unknown argument '{arg}'. Run with --help for usage."),
        }
    }

    Ok(mode)
}

fn print_help() {
    println!(
        "qbit_controller\n\n\
         Usage:\n  \
           qbit_controller                 Run normal maintenance once\n  \
           qbit_controller --orphan-check  Run standalone read-only orphan data check\n  \
           qbit_controller --help          Show this help"
    );
}
