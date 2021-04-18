use slog::*;
use slog_syslog::Facility;
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

        let logger: Logger;
        match slog_syslog::SyslogBuilder::new()
            .facility(Facility::LOG_USER)
            .level(slog::Level::Debug)
            .unix("/dev/log")
            .start() {
                Ok(x) => {
                    logger = Logger::root(x.fuse(), o!());
                },
                Err(e) => panic!("Failed to start syslog on {}. Error {:?}",
                    syslog_p.to_str().unwrap(), e)
            };

        logger
    };
}

pub fn get_logger(name: &str) -> Logger {
    LOGGER.new(o!("ID" => name.to_string()))
}
