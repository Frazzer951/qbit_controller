use anyhow::Result;
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
    config::{Appender, Logger, Root},
    encode::pattern::PatternEncoder,
    Config,
};

pub fn setup_logger() -> Result<()> {
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

    let log_config = Config::builder()
        .appender(Appender::builder().build("console", Box::new(stdout)))
        .appender(Appender::builder().build("file", Box::new(logfile)))
        .logger(Logger::builder().build("app::console", LevelFilter::Info)) // Console logger
        .logger(Logger::builder().build("app::file", LevelFilter::Trace)) // File logger
        .build(
            Root::builder()
                .appender("console")
                .appender("file")
                .build(LevelFilter::Trace),
        )
        .unwrap();

    log4rs::init_config(log_config).unwrap();

    Ok(())
}
