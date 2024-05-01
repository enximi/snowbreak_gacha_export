use anyhow::Result;
use enum_iterator::Sequence;
use lazy_static::lazy_static;
use serde::{Deserialize, Serialize};

use crate::consume_all_events;

#[derive(Debug, Clone, Serialize, Deserialize, Copy, Sequence)]
pub enum Language {
    Zh,
    En,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub language: Language,
}

impl Config {
    pub fn set_lanauge_as_zh(&mut self) {
        self.language = Language::Zh;
    }

    pub fn set_lanauge_as_en(&mut self) {
        self.language = Language::En;
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

    pub fn get_config() -> Self {
        let is_config_file_exists = Self::is_config_file_exists();
        let config_res = Self::load_config();
        if is_config_file_exists && config_res.is_ok() {
            return config_res.unwrap();
        }
        let mut config = Self::default();
        consume_all_events();
        println!("按下数字键选择语言: 1. 简体中文 2. English");
        println!("Press a number to select language: 1. 简体中文 2. English");
        loop {
            if let Ok(crossterm::event::Event::Key(key)) = crossterm::event::read() {
                match key.code {
                    crossterm::event::KeyCode::Char('1') => {
                        config.set_lanauge_as_zh();
                        log::info!("Set language as zh");
                        break;
                    }
                    crossterm::event::KeyCode::Char('2') => {
                        config.set_lanauge_as_en();
                        log::info!("Set language as en");
                        break;
                    }
                    _ => {
                        log::warn!("Invalid input: {:?}", key);
                        println!("按下数字键选择语言: 1. 简体中文 2. English");
                        println!("Press a number to select language: 1. 简体中文 2. English");
                    }
                }
            }
        }
        config.save_config().unwrap();
        config
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            language: Language::Zh,
        }
    }
}

lazy_static! {
    pub static ref CONFIG: Config = Config::get_config();
}
