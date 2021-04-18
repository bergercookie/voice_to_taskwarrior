use anyhow::{anyhow, Result};

use audrey::read::Reader;
use std::cell::RefCell;
use std::fs::canonicalize;
use std::fs::File;
use std::path::Path;
use std::vec;
use task_hookrs::annotation::Annotation;
use task_hookrs::tag::Tag;
use uuid::Uuid;

use dasp_interpolate::linear::Linear;
use dasp_signal::{from_iter, interpolate, Signal};
use deepspeech::Model;

use task_hookrs::status::TaskStatus;
use task_hookrs::task::Task;
use task_hookrs::tw;
use task_hookrs::uda::UDA;

use crate::config::{DeepSpeechConfig, TaskWarriorConfig};

pub struct V2TConverter {
    model: RefCell<Model>,

    /// Target sample rate [Hz]. If sample doesn't match, we'll interpolate
    sample_rate: u32,
    tw_config: TaskWarriorConfig,
}

impl V2TConverter {
    pub fn new(deepspeech: DeepSpeechConfig, tw_config: TaskWarriorConfig) -> Result<Self> {
        let mut model = Model::load_from_files(Path::new(deepspeech.model.to_str().unwrap()))?;
        match deepspeech.scorer {
            Some(s) => {
                model.enable_external_scorer(Path::new(s.to_str().unwrap()))?;
            }
            None => {}
        }

        // TODO Deal with default tag
        Ok(Self {
            model: RefCell::new(model),
            sample_rate: 16_000,
            tw_config,
        })
    }

    /// Convert the given voice memo to a TaskWarrior task
    pub fn convert_to_task(&self, p: &Path) -> Result<Uuid> {
        let task_content = self.voice_recognition(p)?;

        let uuid = self.create_task(&task_content, p)?;
        Ok(uuid)
    }

    fn voice_recognition(&self, p: &Path) -> Result<String> {
        let f = File::open(p)?;
        let mut reader = Reader::new(f)?;

        // make sure that we have mono
        if reader.description().channel_count() != 1 {
            return Err(anyhow!("Can only handle mono files!"));
        }

        let sample_rate = reader.description().sample_rate();

        // Obtain the buffer of sample
        let audio_buf: Vec<_> = if sample_rate == self.sample_rate {
            reader.samples().map(|s| s.unwrap()).collect()
        } else {
            // We need to interpolate to the target sample rate
            let interpolator = Linear::new([0i16], [0]);
            let conv = interpolate::Converter::from_hz_to_hz(
                from_iter(reader.samples::<i16>().map(|s| [s.unwrap()])),
                interpolator,
                sample_rate as f64,
                self.sample_rate as f64,
            );
            conv.until_exhausted().map(|v| v[0]).collect()
        };

        // Run the speech to text algorithm
        Ok(self.model.borrow_mut().speech_to_text(&audio_buf)?)
    }

    fn create_task(&self, task_content: &str, ref_fpath: &Path) -> Result<Uuid> {
        // sanitize
        let content = task_content.to_lowercase();

        let mut t = self.assemble_task(&content);
        let entry_date = chrono::offset::Local::now().naive_utc();

        // TODO parse due date --------------------------------------------------------------------
        // // TODO currently I support only a single word after the due word
        // let due_word = self.tw_config.due_word.clone().unwrap();

        // let content_v: Vec<&str> = content.split(" ").collect();
        // println!("BEFORE remove content_v: {:#?}", content_v);
        // let schedule_at: Some<String> = None
        // match content_v
        //     .iter()
        //     .position(|word| word == &due_word)
        //     {
        //         Some(pos) => {
        //             content_v.remove(pos);
        //             schedule_at = content_v.remove(pos);
        //         }
        //         None => {}
        //     }
        // println!("AFTER remove content_v: {:#?}", content_v);

        // t.set_due();

        // parse tags + default tags --------------------------------------------------------------
        let tags = match &self.tw_config.extra_tags {
            Some(extra_tags) => extra_tags.clone(),
            None => Vec::new(),
        };

        // TODO detect tags in voice memo
        t.set_tags::<_, Vec<Tag>>(Some(tags));

        // add annotations ------------------------------------------------------------------------
        let annotations = vec![Annotation::new(
            entry_date.into(),
            format!(
                "Created from file: \"{}\"",
                canonicalize(ref_fpath)?.as_path().to_str().unwrap()
            )
            .into(),
        )];

        t.set_annotations::<_, Vec<Annotation>>(Some(annotations));

        tw::save(Some(&t)).unwrap();
        return Ok(t.uuid().clone());
    }

    fn assemble_task<S: AsRef<str>>(&self, description: S) -> Task
    where
        String: From<S>,
    {
        let date = chrono::offset::Local::now().naive_utc();
        Task::new(
            None,
            TaskStatus::Pending,
            Uuid::new_v4(),
            date.into(),
            description.into(),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            None,
            UDA::default(),
        )
    }
}
