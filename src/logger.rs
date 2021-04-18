use slog::*;
use slog_syslog::*;
use slog_term;
use std::path::Path;

use lazy_static::lazy_static;

lazy_static! {
    static ref LOGGER: Logger = {
        // syslog ---------------------------------------------------------------------------------
        // try logging to /var/run/syslog
        // if that's not there, try /dev/log
        let mut syslog_p = Path::new("/var/run/syslog");
        if !syslog_p.exists() {
            syslog_p = Path::new("/dev/log");
        }
        if !syslog_p.exists() {
            panic!(format!("Can't find a syslog file to log to - {}", syslog_p.to_str().unwrap()))
        }

        let syslog: slog_syslog::Streamer3164;
        match slog_syslog::SyslogBuilder::new()
            .facility(Facility::LOG_USER)
            .level(slog::Level::Debug)
            .unix("/dev/log")
            .start() {
                Ok(x) => {
                    syslog = x;
                },
                Err(e) => panic!("Failed to start syslog on {}. Error {:?}",
                    syslog_p.to_str().unwrap(), e)
            };

        // terminal logger ------------------------------------------------------------------------
        let decorator = slog_term::TermDecorator::new().force_color().build();
        let term = slog_term::FullFormat::new(decorator).build().fuse();
        let term = std::sync::Mutex::new(term).fuse(); // not optimal, but not important either

        // assemble logger ------------------------------------------------------------------------
        let logger = Logger::root(Duplicate::new(
                LevelFilter::new(term, Level::Info),
                LevelFilter::new(syslog, Level::Info)
        ).fuse(), o!());

        logger
    };
}

pub fn get_logger(name: &str) -> Logger {
    LOGGER.new(o!("ID" => name.to_string()))
}
