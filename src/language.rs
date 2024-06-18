use std::fmt::Display;

use enum_iterator::Sequence;
use serde::{Deserialize, Serialize};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Sequence)]
pub enum Language {
    /// 简体中文
    ChineseSimplified,
    /// 英文
    English,
}

impl Display for Language {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Language::ChineseSimplified => write!(f, "简体中文"),
            Language::English => write!(f, "English"),
        }
    }
}
