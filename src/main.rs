use clap::{App, Arg};
use notify::{watcher, RecursiveMode, Watcher};
use std::sync::mpsc::channel;
use std::time::Duration;

fn main() {
    let matches = App::new("voicememo2task")
        .version("0.1")
        .author("Nikos Koukis <nickkouk@gmail.com>")
        .about("Convert voice recordings to TaskWarrior Tasks")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("FILE")
                .help("Set the configuration file to be used")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("INPUT")
                .help("Set the input directory from which to grab the recordings")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .get_matches();

    // Create a channel to receive the events.
    let (tx, rx) = channel();

    // Create a watcher object, delivering debounced events.
    // The notification back-end is selected based on the platform.
    let mut watcher = watcher(tx, Duration::from_secs(10)).unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher
        .watch("/home/user/test", RecursiveMode::Recursive)
        .unwrap();

    loop {
        match rx.recv() {
            Ok(event) => println!("{:?}", event),
            Err(e) => println!("watch error: {:?}", e),
        }
    }
}
