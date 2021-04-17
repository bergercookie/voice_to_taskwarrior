use anyhow::{anyhow, Result};
use audrey::read::Reader;
use std::cell::RefCell;
use std::fs::canonicalize;
use std::fs::File;
use std::path::{Path, PathBuf};
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

pub struct V2TConverter {
    model: RefCell<Model>,
    taskrc_override: Option<PathBuf>,

    /// Target sample rate [Hz]. If sample doesn't match, we'll interpolate
    sample_rate: u32,
    default_tag: Tag,
}

impl V2TConverter {
    pub fn new() -> Result<Self> {
        // TODO Parametrize this
        let mut model = Model::load_from_files(
                Path::new("/home/berger/sync/bulk/software/deepspeech_native_client.amd64.cpu.linux/deepspeech-0.9.0-models.pbmm"))?;
        model.enable_external_scorer(Path::new("/home/berger/sync/bulk/software/deepspeech_native_client.amd64.cpu.linux/deepspeech-0.9.3-models.scorer"))?;

        Ok(Self {
            model: RefCell::new(model),
            taskrc_override: None,
            sample_rate: 16_000,
            default_tag: "voice_memo".into(),
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
        let mut reader = Reader::new(f).unwrap();

        // make sure that we have mono
        if reader.description().channel_count() != 1 {
            anyhow!("Can only handle mono files!");
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
        let mut t = self.assemble_task(task_content);
        let entry_date = chrono::offset::Local::now().naive_utc();

        // TODO parse due date --------------------------------------------------------------------
        // t.set_due();

        // TODO assign tags + default tag ---------------------------------------------------------
        let mut tags = Vec::new();
        tags.push(self.default_tag.clone());

        t.set_tags::<_, Vec<Tag>>(Some(tags));

        // add annotations -------------------------------------------------------------------
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
        println!("Task content to be created: {:#?}", t);
        return Ok(t.uuid().clone());
    }

    fn assemble_task(&self, description: &str) -> Task {
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
