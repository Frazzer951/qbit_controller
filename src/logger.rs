use anyhow::{anyhow, Result};
use log::LevelFilter;
use log4rs::{
    append::{
        console::ConsoleAppender,
        rolling_file::{
            policy::compound::{
                roll::fixed_window::FixedWindowRoller, trigger::size::SizeTrigger, CompoundPolicy,
            },
            RollingFileAppender,
        },
    },
    config::{Appender, Root},
    encode::pattern::PatternEncoder,
    filter::threshold::ThresholdFilter,
    Config, Handle,
};

pub fn setup_logger() -> Result<Handle> {
    let log_config = generate_config(LevelFilter::Info)?;

    match log4rs::init_config(log_config) {
        Ok(handle) => Ok(handle),
        Err(e) => Err(anyhow!("Error initializing logger: {}", e)),
    }
}

pub fn update_logger(handle: &Handle, log_level: LevelFilter) -> Result<()> {
    let log_config = generate_config(log_level)?;

    handle.set_config(log_config);

    Ok(())
}

fn generate_config(log_level: LevelFilter) -> Result<Config> {
    let pattern = "{({d(%Y-%m-%d %H:%M:%S)} {h({l})}):<25} - {m}{n}";

    let stdout: ConsoleAppender = ConsoleAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .build();

    let log_file_count = 5;
    let fixed_window_roller = FixedWindowRoller::builder()
        .build("config/qbit_controller.{}.log", log_file_count)
        .unwrap();
    let size_trigger = SizeTrigger::new(1024 * 1024 * 10); // 10 MB
    let compound_policy =
        CompoundPolicy::new(Box::new(size_trigger), Box::new(fixed_window_roller));
    let logfile = RollingFileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(pattern)))
        .build("config/qbit_controller.log", Box::new(compound_policy))
        .unwrap();

    match Config::builder()
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(log_level)))
                .build("console", Box::new(stdout)),
        )
        .appender(
            Appender::builder()
                .filter(Box::new(ThresholdFilter::new(LevelFilter::Trace)))
                .build("file", Box::new(logfile)),
        )
        .build(
            Root::builder()
                .appender("console")
                .appender("file")
                .build(LevelFilter::Trace),
        ) {
        Ok(config) => Ok(config),
        Err(e) => Err(anyhow!("Error generating config: {}", e)),
    }
}
