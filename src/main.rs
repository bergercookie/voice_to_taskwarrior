use anyhow::Result;
use clap::{App, Arg};
use notify::{watcher, DebouncedEvent::Create, RecursiveMode, Watcher};

use std::sync::mpsc::channel;
use std::time::Duration;
use voicememo2task::voice_to_task_converter::V2TConverter;

fn main() -> Result<()> {
    let matches = App::new("voicememo2task")
        .version("0.1.0")
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
            Arg::with_name("input-dir")
                .help("Set the input directory from which to grab the recordings")
                .required(true)
                .short("i")
                .long("input-dir")
                .value_name("INPUT_DIR")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .get_matches();

    let v2t = V2TConverter::new()?;

    // Create a channel to receive the events.
    let (tx, rx) = channel();

    // Create a watcher object, delivering debounced events.
    // The notification back-end is selected based on the platform.
    let mut watcher = watcher(tx, Duration::from_secs(10)).unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher
        // TODO Parametrize this
        .watch("/home/berger/sync/recordings/", RecursiveMode::Recursive)
        .unwrap();

    loop {
        match rx.recv() {
            Ok(event) => match event {
                Create(pathbuf) => match v2t.convert_to_task(&pathbuf) {
                    Ok(uuid) => println!(
                        "Memo \"{}\" => Task: \"{}\"",
                        pathbuf.to_str().unwrap(),
                        uuid
                    ),
                    Err(err) => println!(
                        "Memo \"{}\", Error creating a task for it: {}",
                        pathbuf.to_str().unwrap(),
                        err
                    ),
                },
                _ => {}
            },
            Err(e) => println!("watch error: {:?}", e),
        }
    }
}
