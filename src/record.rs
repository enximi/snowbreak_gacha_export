use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use anyhow::{anyhow, Result};
use chrono::{Local, TimeZone};
use enum_iterator::{all, Sequence};
use serde::{Deserialize, Serialize};

use crate::language::Language;

/// 卡池类型
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Sequence)]
pub enum BannerType {
    /// 100%限定角色池
    LimitedCharacter100Percent,
    /// 100%限定武器池
    LimitedWeapon100Percent,
    /// 50%限定角色池
    LimitedCharacter50Percent,
    /// 50%限定武器池
    LimitedWeapon50Percent,
    /// 常驻角色池
    PermanentCharacter,
    /// 常驻武器池
    PermanentWeapon,
    /// 新手池
    Beginner,
}

impl BannerType {
    pub fn display_name_for_user(&self, language: Language) -> &str {
        match language {
            Language::ChineseSimplified => match self {
                BannerType::LimitedCharacter100Percent => "100%限定角色池",
                BannerType::LimitedWeapon100Percent => "100%限定武器池",
                BannerType::LimitedCharacter50Percent => "50%限定角色池",
                BannerType::LimitedWeapon50Percent => "50%限定武器池",
                BannerType::PermanentCharacter => "常驻角色池",
                BannerType::PermanentWeapon => "常驻武器池",
                BannerType::Beginner => "新手池",
            },
            Language::English => match self {
                BannerType::LimitedCharacter100Percent => "100% Limited Character Banner",
                BannerType::LimitedWeapon100Percent => "100% Limited Weapon Banner",
                BannerType::LimitedCharacter50Percent => "50% Limited Character Banner",
                BannerType::LimitedWeapon50Percent => "50% Limited Weapon Banner",
                BannerType::PermanentCharacter => "Permanent Character Banner",
                BannerType::PermanentWeapon => "Permanent Weapon Banner",
                BannerType::Beginner => "Beginner Banner",
            },
        }
    }

    pub fn pity_count(&self) -> u32 {
        match self {
            BannerType::LimitedCharacter100Percent => 100,
            BannerType::LimitedWeapon100Percent => 80,
            BannerType::LimitedCharacter50Percent => 80,
            BannerType::LimitedWeapon50Percent => 60,
            BannerType::PermanentCharacter => 80,
            BannerType::PermanentWeapon => 60,
            BannerType::Beginner => 50,
        }
    }
}

/// 抽卡物品类型
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Sequence)]
pub enum ItemType {
    /// 角色
    Character,
    /// 武器
    Weapon,
}

impl ItemType {
    pub fn display_name_for_user(&self, language: Language) -> &str {
        match language {
            Language::ChineseSimplified => match self {
                ItemType::Character => "角色",
                ItemType::Weapon => "武器",
            },
            Language::English => match self {
                ItemType::Character => "Operative",
                ItemType::Weapon => "Weapon",
            },
        }
    }

    pub fn display_name_in_record_page_in_game(&self, language: Language) -> &str {
        match language {
            Language::ChineseSimplified => match self {
                ItemType::Character => "角色",
                ItemType::Weapon => "武器",
            },
            Language::English => match self {
                ItemType::Character => "Operative",
                ItemType::Weapon => "Weapon",
            },
        }
    }

    pub fn display_names_in_record_page_in_game_in_all_languages(&self) -> Vec<&str> {
        all::<Language>()
            .map(|language| self.display_name_in_record_page_in_game(language))
            .collect()
    }
}

/// 抽卡记录
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OneRecord {
    pub star: u8,
    pub item_name: String,
    pub item_type: ItemType,
    pub timestamp: u64,
}

impl OneRecord {
    pub fn new(star: u8, item_name: String, item_type: ItemType, timestamp: u64) -> Self {
        Self {
            star,
            item_name,
            item_type,
            timestamp,
        }
    }

    pub fn readable_date_time_str(&self) -> String {
        let date_time = Local
            .timestamp_opt(self.timestamp as i64, 0)
            .single()
            .unwrap();
        date_time.format("%Y-%m-%d %H:%M").to_string()
    }
}

/// 合并抽卡记录
/// 两个抽卡记录按时间顺序合并
/// # 返回
/// （合并后的抽卡记录，新增抽卡记录数量）
pub fn merge_gacha_records(
    new_records: &[OneRecord],
    old_records: &[OneRecord],
) -> Result<(Vec<OneRecord>, u32)> {
    // 抽卡记录是按时间倒序排列的，最新的在最前面
    if new_records.is_empty() {
        return Ok((old_records.to_vec(), 0));
    }
    if old_records.is_empty() {
        return Ok((new_records.to_vec(), new_records.len() as u32));
    }

    // 现在不知道那个抽卡记录是新的
    // 比较两个抽卡记录的最新时间
    // 时间晚的是新的记录
    let new_records_first_time = new_records.first().unwrap().timestamp;
    let old_records_first_time = old_records.first().unwrap().timestamp;
    if new_records_first_time < old_records_first_time {
        return merge_gacha_records(old_records, new_records);
    }

    // 两个抽卡记录的长度
    let new_records_len = new_records.len();
    let old_records_len = old_records.len();
    let min_len = std::cmp::min(new_records_len, old_records_len);

    // 循环 min_len..=0
    // 如果新纪录的最后i个元素和老记录的前i个元素相同
    // 说明新纪录的前面records1_len - i个元素是新的
    // 把新纪录的前面records1_len - i个元素插入到老记录的前面完成合并
    let same_num = {
        let mut same_num = 0;
        for i in (1..=min_len).rev() {
            if new_records[new_records_len - i..] == old_records[..i] {
                same_num = i;
                break;
            }
        }
        same_num
    };
    let merged_records = new_records[..new_records_len - same_num]
        .iter()
        .chain(old_records.iter())
        .cloned()
        .collect();

    // 检查时间戳是递减的
    let is_timestamp_desc = |records: &Vec<OneRecord>| -> (bool, usize) {
        for i in 1..records.len() {
            if records[i].timestamp > records[i - 1].timestamp {
                return (false, i);
            }
        }
        (true, 0)
    };
    let (is_desc, index) = is_timestamp_desc(&merged_records);
    if !is_desc {
        return Err(anyhow!(
            "merged_records_len: {}, index: {index}, records: {:?}",
            merged_records.len(),
            merged_records[index - 1..index + 1].to_vec(),
        ));
    }

    let add_num = (new_records_len - same_num) as u32;

    Ok((merged_records, add_num))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OneAccountRecords {
    pub id: String,
    pub records: HashMap<BannerType, Vec<OneRecord>>,
}

impl OneAccountRecords {
    pub fn new(id: String, records: HashMap<BannerType, Vec<OneRecord>>) -> Self {
        Self { id, records }
    }

    pub fn add_record(&mut self, banner_type: BannerType, records: Vec<OneRecord>) -> Result<u32> {
        let old_records = self.records.entry(banner_type).or_default();
        let (merged_records, add_num) = merge_gacha_records(&records, old_records)?;
        *old_records = merged_records;
        Ok(add_num)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TotalRecords {
    /// 账号ID -> 账号抽卡记录
    pub records: HashMap<String, OneAccountRecords>,
}

impl TotalRecords {
    pub fn new(records: HashMap<String, OneAccountRecords>) -> Self {
        Self { records }
    }

    pub fn add_record(
        &mut self,
        account_id: String,
        banner_type: BannerType,
        records: Vec<OneRecord>,
    ) -> Result<u32> {
        let account_records = self
            .records
            .entry(account_id.clone())
            .or_insert_with(|| OneAccountRecords::new(account_id.clone(), HashMap::new()));
        account_records.add_record(banner_type, records)
    }

    pub fn save(&self) -> Result<()> {
        let path = "records/records.json";
        if let Some(parent) = Path::new(path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = File::create(path)?;
        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(writer, self).map_err(|e| e.into())
    }

    fn read() -> Result<Self> {
        let path = "records/records.json";
        let file = File::open(path)?;
        let reader = std::io::BufReader::new(file);
        serde_json::from_reader(reader).map_err(|e| e.into())
    }

    pub fn read_or_default() -> Self {
        Self::read().unwrap_or_else(|e| {
            log::error!("Failed to read records: {:?}", e);
            Self::default()
        })
    }
}

impl Default for TotalRecords {
    fn default() -> Self {
        Self::new(HashMap::new())
    }
}
