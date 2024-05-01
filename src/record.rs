use std::sync::{Arc, Mutex};

use anyhow::{anyhow, Result};
use chrono::{Local, TimeZone};
use enum_iterator::Sequence;
use image::{DynamicImage, GenericImageView};
use lazy_static::lazy_static;
use serde_json::Value;
use tokio::spawn;

use crate::ocr_server;
use crate::ocr_server::OcrClient;

lazy_static! {
    static ref OCR_CLIENT: Arc<Mutex<OcrClient>> = {
        let (_, _, ocr_client) = ocr_server::run_server();
        Arc::new(Mutex::new(ocr_client))
    };
}

/// 卡池类型
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Sequence,
)]
pub enum BannerType {
    /// 角色活动
    LimitedTimeCharacter,
    /// 武器活动
    LimitedTimeWeapon,
    /// 角色常驻
    StandardCharacter,
    /// 武器常驻
    StandardWeapon,
    /// 新手
    Novice,
}

impl BannerType {
    pub fn chinese_display_name(&self) -> String {
        match self {
            BannerType::LimitedTimeCharacter => "限定角色池",
            BannerType::LimitedTimeWeapon => "限定武器池",
            BannerType::StandardCharacter => "常驻角色池",
            BannerType::StandardWeapon => "常驻武器池",
            BannerType::Novice => "新手池",
        }
        .to_string()
    }
    
    pub fn save_file_name(&self) -> String {
        match self {
            BannerType::LimitedTimeCharacter => "limited_time_character",
            BannerType::LimitedTimeWeapon => "limited_time_weapon",
            BannerType::StandardCharacter => "standard_character",
            BannerType::StandardWeapon => "standard_weapon",
            BannerType::Novice => "novice",
        }
        .to_string()
    }

    pub fn pity_count(&self) -> u32 {
        match self {
            BannerType::LimitedTimeCharacter | BannerType::StandardCharacter => 80,
            BannerType::LimitedTimeWeapon | BannerType::StandardWeapon => 60,
            BannerType::Novice => 50,
        }
    }
}

/// 抽卡物品类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ItemType {
    /// 角色
    Character,
    /// 武器
    Weapon,
}

impl ItemType {
    pub fn chinese_display_name(&self) -> String {
        match self {
            ItemType::Character => "角色",
            ItemType::Weapon => "武器",
        }
        .to_string()
    }
}

/// 抽卡记录
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct GachaRecord {
    pub star: u8,
    pub item_name: String,
    pub item_type: ItemType,
    pub timestamp: u64,
}

impl GachaRecord {
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

#[derive(Debug, Clone)]
pub struct OneRecordImage {
    img: DynamicImage,
}

impl OneRecordImage {
    fn new(img: DynamicImage) -> Self {
        Self { img }
    }

    /// 星级颜色
    fn star_color(&self) -> (u8, u8, u8) {
        let color = self.img.get_pixel(2, 23);
        (color.0[0], color.0[1], color.0[2])
    }

    /// 星级
    fn star(&self) -> Result<u8> {
        let star_color = self.star_color();
        let star3_color = (55, 98, 242);
        let star4_color = (192, 105, 214);
        let star5_color = (233, 155, 55);
        if star_color == star3_color {
            return Ok(3);
        }
        if star_color == star4_color {
            return Ok(4);
        }
        if star_color == star5_color {
            return Ok(5);
        }
        Err(anyhow!("Invalid color: {:?}", star_color))
    }

    /// 物品名称图片
    fn item_name_image(&self) -> DynamicImage {
        self.img.crop_imm(13, 0, 517, self.img.height())
    }

    /// 物品名称
    pub async fn item_name(&self) -> Result<String> {
        let item_name_img = self.item_name_image();
        let item_name_rx = OCR_CLIENT.lock().unwrap().send(item_name_img);
        let data = item_name_rx.await??;
        get_string_from_data(data, false)
    }

    /// 物品类型图片
    fn item_type_image(&self) -> DynamicImage {
        self.img.crop_imm(530, 0, 55, self.img.height())
    }

    /// 物品类型
    async fn item_type(&self) -> Result<ItemType> {
        let item_type_img = self.item_type_image();
        let item_type_rx = OCR_CLIENT.lock().unwrap().send(item_type_img);
        let data = item_type_rx.await??;
        let item_type_str = get_string_from_data(data, false)?;
        match item_type_str.as_str() {
            "角色" => Ok(ItemType::Character),
            "武器" => Ok(ItemType::Weapon),
            _ => Err(anyhow!("Invalid item type: {}", item_type_str)),
        }
    }

    pub fn time_img(&self) -> DynamicImage {
        self.img.crop_imm(1020, 0, 184, self.img.height())
    }

    pub async fn time_str(&self) -> Result<String> {
        let time_img = self.time_img();
        let time_str_rx = OCR_CLIENT.lock().unwrap().send(time_img);
        let data = time_str_rx.await??;
        get_string_from_data(data, true)
    }

    /// 时间戳
    async fn timestamp(&self) -> Result<u64> {
        let date_time_str = self.time_str().await?;
        let date_time = chrono::NaiveDateTime::parse_from_str(&date_time_str, "%Y-%m-%d %H:%M")
            .map_err(|e| anyhow!("Failed to parse date time: {:?}", e))?;
        let local_date_time = Local
            .from_local_datetime(&date_time)
            .single()
            .ok_or_else(|| anyhow!("Invalid local date time: {}", date_time_str))?;
        Ok(local_date_time.timestamp() as u64)
    }

    /// 抽卡记录
    async fn gacha_record(&self) -> Result<GachaRecord> {
        let star = self.star()?;
        let item_name = self.item_name().await?;
        let item_type = self.item_type().await?;
        let timestamp = self.timestamp().await?;
        Ok(GachaRecord::new(star, item_name, item_type, timestamp))
        // 得想个办法让这里也可以并行
        // let item_name_handle = spawn(async { self.item_name().await });
        // let item_type_handle = spawn(async { self.item_type().await });
        // let timestamp_handle = spawn(async { self.timestamp().await });
        // let item_name = item_name_handle.await??;
        // let item_type = item_type_handle.await??;
        // let timestamp = timestamp_handle.await??;
        // Ok(GachaRecord::new(star, item_name, item_type, timestamp))
    }
}

/// 合并抽卡记录
/// 两个抽卡记录按时间顺序合并
/// # 返回
/// （合并后的抽卡记录，新增抽卡记录数量）
pub fn merge_gacha_records(
    new_records: &[GachaRecord],
    old_records: &[GachaRecord],
) -> Result<(Vec<GachaRecord>, u32)> {
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
    let is_timestamp_desc = |records: &Vec<GachaRecord>| -> bool {
        for i in 1..records.len() {
            if records[i].timestamp > records[i - 1].timestamp {
                return false;
            }
        }
        true
    };
    if !is_timestamp_desc(&merged_records) {
        return Err(anyhow!("Invalid merged records: {:?}", merged_records));
    }

    let add_num = (new_records_len - same_num) as u32;

    Ok((merged_records, add_num))
}

#[derive(Clone)]
pub struct RecordScreen {
    img: DynamicImage,
}

impl RecordScreen {
    pub fn new(img: DynamicImage) -> Self {
        Self { img }
    }

    pub fn img(&self) -> &DynamicImage {
        &self.img
    }

    fn index_image(&self) -> DynamicImage {
        self.img.crop_imm(1625, 480, 80, 80)
    }

    async fn index_string(&self) -> Result<String> {
        let index_img = self.index_image();
        let index_rx = OCR_CLIENT.lock().unwrap().send(index_img);
        let data = index_rx.await??;
        get_string_from_data(data, false)
    }

    pub async fn index(&self) -> Result<u32> {
        let index_str = self.index_string().await?;
        let index = index_str
            .parse()
            .map_err(|e| anyhow!("Failed to parse index: {:?}", e))?;
        Ok(index)
    }

    fn record_images(&self) -> Result<Vec<DynamicImage>> {
        let record_width = 1228;
        let record_height = 45;
        let record_x = 349;
        let _end_x = 1577;
        let first_record_y = 201;
        let space_height = 607.0 / 9.0 - 45.0;
        (0..10)
            .map(|i| {
                let record_y = (first_record_y as f64
                    + i as f64 * (record_height as f64 + space_height))
                    .round() as u32;
                Ok(self
                    .img
                    .crop_imm(record_x, record_y, record_width, record_height))
            })
            .collect::<Result<Vec<DynamicImage>>>()
    }

    pub fn one_record_images(&self) -> Result<Vec<OneRecordImage>> {
        let record_images = self.record_images()?;
        let one_record_images = record_images
            .iter()
            .map(|record_image| OneRecordImage::new(record_image.clone()))
            .collect::<Vec<OneRecordImage>>();
        Ok(one_record_images)
    }

    pub async fn gacha_records(&self) -> Result<Vec<GachaRecord>> {
        let one_record_images = self.one_record_images()?;
        let handles = one_record_images
            .into_iter()
            .map(|one_record_image| spawn(async move { one_record_image.gacha_record().await }))
            .collect::<Vec<_>>();
        let results = futures::future::join_all(handles).await;
        results
            .into_iter()
            .take_while(|result| result.is_ok())
            .map(|result| result.unwrap())
            .take_while(|record| record.is_ok())
            .collect::<Result<Vec<GachaRecord>>>()
    }
}

fn get_string_from_data(data: Vec<Value>, is_date_time: bool) -> Result<String> {
    let get_item_1_text = |data: &Vec<Value>| -> Result<String> {
        let item1 = match data.first() {
            Some(item1) => item1,
            None => {
                return Err(anyhow!("Failed to get item1 from data: {:?}", data));
            }
        };
        let item1_text = match item1["text"].as_str() {
            Some(item1_text) => item1_text,
            None => {
                return Err(anyhow!("Failed to get text from item1: {:?}", item1));
            }
        };
        Ok(item1_text.to_string())
    };
    if is_date_time && data.len() >= 2 {
        let item1_text = get_item_1_text(&data)?;
        let item2 = match data.get(1) {
            Some(item2) => item2,
            None => {
                return Err(anyhow!("Failed to get item2 from data: {:?}", data));
            }
        };
        let item2_text = match item2["text"].as_str() {
            Some(item2_text) => item2_text,
            None => {
                return Err(anyhow!("Failed to get text from item2: {:?}", item2));
            }
        };
        Ok(format!("{} {}", item1_text, item2_text))
    } else {
        get_item_1_text(&data)
    }
}

pub struct GachaRecords {
    banner_type: BannerType,
    records: Vec<GachaRecord>,
}

impl GachaRecords {
    pub fn new(banner_type: BannerType, records: Vec<GachaRecord>) -> Self {
        Self {
            banner_type,
            records,
        }
    }

    pub fn banner_type(&self) -> BannerType {
        self.banner_type
    }
    
    pub fn records(&self) -> &Vec<GachaRecord> {
        &self.records
    }

    /// 第 index 条记录是上次 5 星之后的第几条记录
    pub fn count_after_5_star(&self, index: u32) -> u32 {
        let mut index_ = index as usize;
        index_ += 1;
        while index_ < self.records.len() {
            if self.records[index_].star == 5 {
                return index_ as u32 - index;
            }
            index_ += 1;
        }
        index_ as u32 - index
    }

    /// 第 index 条记录是上次 4 星之后的第几条记录
    pub fn count_after_4_star(&self, index: u32) -> u32 {
        let mut index_ = index as usize;
        index_ += 1;
        while index_ < self.records.len() {
            if self.records[index_].star == 4 {
                return index_ as u32 - index;
            }
            index_ += 1;
        }
        index_ as u32 - index
    }

    /// 距离五星保底还有多少次
    pub fn count_to_5_star_pity(&self, index: u32) -> u32 {
        self.banner_type.pity_count() - self.count_after_5_star(index)
    }

    /// 距离四星保底还有多少次
    pub fn count_to_4_star_pity(&self, index: u32) -> u32 {
        10 - self.count_after_4_star(index)
    }
}
