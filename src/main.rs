use anyhow::Result;
use clap::{App, Arg};
use notify::{watcher, DebouncedEvent::Create, RecursiveMode, Watcher};
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use std::sync::mpsc::channel;
use std::time::Duration;
use voice_to_taskwarrior::config::ConfigBuilder;
use voice_to_taskwarrior::voice_to_task_converter::V2TConverter;
use voice_to_taskwarrior::logger::get_logger;

use slog::*;

const LAST_VOICE_MEMO_MARK_FNAME: &str = "last_voice_memo";

const APP_NAME: &str = "v2t";

fn main() -> Result<()> {
    let matches = App::new("voice_to_taskwarrior")
        .version("0.1.0")
        .author("Nikos Koukis <nickkouk@gmail.com>")
        .about("Convert voice recordings to TaskWarrior Tasks")
        .arg(
            Arg::with_name("daemon")
                .short("d")
                .long("daemon")
                .help("Watch the designated directory for changes"),
        )
        .arg(
            Arg::with_name("v")
                .short("v")
                .long("verbose")
                .multiple(true)
                .help("Sets the level of verbosity"),
        )
        .get_matches();

    let logger = get_logger(APP_NAME);
    let config = ConfigBuilder::new(&APP_NAME)?.get();

    let is_daemon = matches.is_present("daemon");

    let v2t = V2TConverter::new(config.deepspeech, config.tw.unwrap())?;

    if is_daemon {
        info!(
            logger,
            "Watching directory {} for changes",
            config.input_dir.to_str().unwrap()
        );
        watch_convert_on_change(v2t, &config.input_dir)?;
    } else {
        // Run the conversion only for the voice memos that were added since the last time the
        // script was executed
        convert_all_since_last_run(v2t, &config.input_dir)?;
    }
    Ok(())
}

/// Convert all the voice memos that were recorded since the last time the script was executed.
fn convert_all_since_last_run(v2t: V2TConverter, input_dir: &PathBuf) -> Result<()> {
    let logger = get_logger(APP_NAME);

    // get the last converted memo
    let last_voice_memo_mark_path = input_dir.join(LAST_VOICE_MEMO_MARK_FNAME);
    let last_voice_memo_path = String::from_utf8(fs::read(last_voice_memo_mark_path.clone())?)?;
    let last_created_date = fs::metadata(last_voice_memo_path)?.created()?;

    // get all the memos that were created after the last I processed -------------------------
    let mut memos_to_process = fs::read_dir(input_dir)?
        .map(|dir| dir.unwrap().path())
        .filter(|p| fs::metadata(p).unwrap().created().unwrap() > last_created_date)
        .collect::<Vec<PathBuf>>();

    // skip some files
    match memos_to_process
        .iter()
        .position(|p| p.file_name().unwrap() == LAST_VOICE_MEMO_MARK_FNAME)
    {
        Some(pos) => {
            memos_to_process.remove(pos);
        }
        None => {}
    }
    info!(logger, "memos_to_process: {:#?}", memos_to_process);

    if memos_to_process.is_empty() {
        debug!(logger, "No voice memos to process since last run...");
        return Ok(());
    }

    for p in memos_to_process.iter() {
        convert_mark_inform(&v2t, p, &last_voice_memo_mark_path)?;
    }

    Ok(())
}

/// Watch the designated directory for newly created voice memos and when one is created, create a
/// corresponding task for it
fn watch_convert_on_change(v2t: V2TConverter, input_dir: &PathBuf) -> Result<()> {
    let logger = get_logger(APP_NAME);
    let last_voice_memo_mark_path = input_dir.join(LAST_VOICE_MEMO_MARK_FNAME);

    // Create a channel to receive the events.
    let (tx, rx) = channel();

    // Create a watcher object, delivering debounced events.
    // The notification back-end is selected based on the platform.
    let mut watcher = watcher(tx, Duration::from_secs(10)).unwrap();

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    watcher
        .watch(input_dir, RecursiveMode::NonRecursive)
        .unwrap();

    loop {
        match rx.recv() {
            Ok(event) => match event {
                Create(pathbuf) => {
                    // skip some files
                    if pathbuf.file_name().unwrap().to_str().unwrap() == LAST_VOICE_MEMO_MARK_FNAME
                    {
                        continue;
                    }

                    convert_mark_inform(&v2t, &pathbuf, &last_voice_memo_mark_path)?;
                }
                _ => {}
            },
            Err(e) => error!(logger, "watch error: {:?}", e),
        }
    }
}

fn convert_mark_inform(
    v2t: &V2TConverter,
    voice_memo: &PathBuf,
    last_voice_memo_mark_path: &PathBuf,
) -> Result<()> {
    let logger = get_logger(APP_NAME);
    match v2t.convert_to_task(&voice_memo) {
        Ok(uuid) => {
            info!(
                logger,
                "Memo \"{}\" => Task: \"{}\"",
                voice_memo.to_str().unwrap(),
                uuid
            );
            // write down the last voice task that I edited
            let mut f = std::fs::File::create(last_voice_memo_mark_path.to_str().unwrap())?;
            f.write(voice_memo.to_str().unwrap().as_bytes())?;
        }
        Err(err) => error!(
            logger,
            "Memo \"{}\", Error creating a task for it: {}",
            voice_memo.to_str().unwrap(),
            err
        ),
    }
    Ok(())
}
