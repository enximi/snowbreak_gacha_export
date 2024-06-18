use anyhow::Result;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::language::Language;
use crate::user_interaction::language;

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub struct Config {
    pub language: Language,
}

impl Config {
    pub fn set_language(&mut self, language: Language) {
        self.language = language;
    }
    
    pub fn load_config() -> Result<Self> {
        let path = "config.json";
        let file = std::fs::File::open(path)?;
        let reader = std::io::BufReader::new(file);
        serde_json::from_reader(reader).map_err(|e| e.into())
    }

    pub fn is_config_file_exists() -> bool {
        let path = "config.json";
        std::path::Path::new(path).exists()
    }

    pub fn save_config(&self) -> Result<()> {
        let path = "config.json";
        let file = std::fs::File::create(path)?;
        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer(writer, self).map_err(|e| e.into())
    }

    pub fn load_or_init_config() -> Self {
        let is_config_file_exists = Self::is_config_file_exists();
        let config_res = Self::load_config();
        if is_config_file_exists && config_res.is_ok() {
            return config_res.unwrap();
        }
        let mut config = Self::default();
        let language = language();
        config.set_language(language);
        config.save_config().unwrap();
        config
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: Language::ChineseSimplified,
        }
    }
}

lazy_static! {
    pub static ref CONFIG: Config = Config::load_or_init_config();
}
