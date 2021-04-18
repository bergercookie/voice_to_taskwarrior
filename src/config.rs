use anyhow::{anyhow, Result};
use serde_derive::Deserialize;
use shellexpand::tilde;
use std::fs::read_to_string;
use std::path::PathBuf;
use std::str::FromStr;
use toml;
use xdg;

trait DataSanitizationAndVerification {
    /// Make sure that the given data is valid
    fn verify(&self) -> Result<()>;
    fn sanitize(&mut self) -> Result<()>;
}

// ------------------------------------------------------------------------------------------------
#[derive(Deserialize, Debug, Default)]
pub struct Config {
    pub input_dir: PathBuf,
    pub deepspeech: DeepSpeechConfig,
    pub tw: Option<TaskWarriorConfig>,
}

impl DataSanitizationAndVerification for Config {
    fn verify(&self) -> Result<()> {
        // input directory
        if !self.input_dir.exists() || !self.input_dir.is_dir() {
            return Err(anyhow!(
                "Invalid input directory provided \"{}\"!",
                self.input_dir.to_str().unwrap()
            ));
        }

        self.deepspeech.verify()?;
        match &self.tw {
            Some(tw) => tw.verify()?,
            None => {}
        }

        Ok(())
    }
    fn sanitize(&mut self) -> Result<()> {
        self.input_dir = PathBuf::from_str(&tilde(&self.input_dir.to_str().unwrap()))?;

        self.deepspeech.sanitize()?;
        match &mut self.tw {
            Some(tw) => {
                tw.sanitize()?;
            }
            None => {
                let mut tw: TaskWarriorConfig = Default::default();
                tw.sanitize()?;

                self.tw = Some(tw);
            }
        }

        Ok(())
    }
}

// ------------------------------------------------------------------------------------------------
#[derive(Deserialize, Debug, Default)]
pub struct DeepSpeechConfig {
    pub model: PathBuf,
    pub scorer: Option<PathBuf>,
}

impl DataSanitizationAndVerification for DeepSpeechConfig {
    fn verify(&self) -> Result<()> {
        // deepspeech
        if !self.model.exists() || !self.model.is_file() {
            return Err(anyhow!(
                "Invalid deepspeech model provided \"{}\"!",
                self.model.to_str().unwrap()
            ));
        }
        match self.scorer {
            None => {}
            Some(ref scorer) => {
                if !scorer.exists() || !scorer.is_file() {
                    return Err(anyhow!(
                        "Invalid deepspeech scorer provided \"{}\"!",
                        scorer.to_str().unwrap()
                    ));
                }
            }
        }

        Ok(())
    }
    fn sanitize(&mut self) -> Result<()> {
        self.model = PathBuf::from_str(&tilde(&self.model.to_str().unwrap()))?;

        match self.scorer {
            None => {}
            Some(ref mut scorer) => {
                self.scorer = Some(PathBuf::from_str(&tilde(&scorer.to_str().unwrap()))?);
            }
        }
        Ok(())
    }
}

// ------------------------------------------------------------------------------------------------
#[derive(Deserialize, Debug)]
pub struct TaskWarriorConfig {
    // TODO
    pub ignore_word: Option<String>,
    pub extra_tags: Option<Vec<String>>,
}

impl DataSanitizationAndVerification for TaskWarriorConfig {
    fn verify(&self) -> Result<()> {
        Ok(())
    }
    fn sanitize(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Default for TaskWarriorConfig {
    fn default() -> Self {
        Self {
            ignore_word: Some("skip".to_string()),
            extra_tags: Some(vec!["voice_memo".to_string()]),
        }
    }
}

// ------------------------------------------------------------------------------------------------
pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    fn get_path(app_name: &str) -> Result<PathBuf> {
        let xdg_dirs = xdg::BaseDirectories::with_prefix(app_name)?;
        let config_path = xdg_dirs.place_config_file("config.toml")?;
        if config_path.exists() && !config_path.is_file() {
            return Err(anyhow!(format!(
                "Configuration path exists and it isn't a file \"{}\"! Can't handle this.",
                config_path.to_str().unwrap()
            )));
        }

        if !config_path.exists() {
            return Err(anyhow!(format!("TOML Configuration file doesn't exist \"{}\"! Please create it first with the appropriate keys and re-run this.",
                        config_path.to_str().unwrap())));
        }

        return Ok(config_path);
    }

    pub fn new(app_name: &str) -> Result<Self> {
        let config_path = ConfigBuilder::get_path(&app_name)?;
        let toml_contents = read_to_string(config_path)?;
        let mut config: Config = toml::from_str(&toml_contents)?;

        // sanitize & fill in the gaps - defaults -------------------------------------------------
        match config.sanitize() {
            Ok(_) => {}
            Err(err) => return Err(err),
        }

        // verify config --------------------------------------------------------------------------
        match config.verify() {
            Ok(_) => {}
            Err(err) => return Err(err),
        }

        Ok(ConfigBuilder { config })
    }

    pub fn get(self) -> Config {
        self.config
    }
}
