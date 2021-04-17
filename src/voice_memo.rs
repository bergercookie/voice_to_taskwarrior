use std::path::{PathBuf, Path};
use anyhow::{Result, anyhow};

pub struct VoiceMemo {
    pub path: PathBuf,
}

impl VoiceMemo {
    pub fn new(path: &Path) -> Result<Self> {
        if !path.exists() || !path.is_file() {
            return Err(anyhow!("Invalid path to voice memo provided -> {}", path.to_str().unwrap()));
        }

        Ok(Self {
            path: path.into()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_voice_memo_invalid_file() {
        match VoiceMemo::new(Path::new("some/random/path")) {
            Ok(_res) => assert!(false, "No error emmitted as it should"),
            Err(e) => assert!(e.to_string().starts_with("Invalid path"), "Wrong error message")
        }
    }
}
