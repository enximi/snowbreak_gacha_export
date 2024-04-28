use std::sync::{Arc, Mutex};

use anyhow::anyhow;
use chrono::{Local, TimeZone};
use image::{DynamicImage, GenericImageView};
use imageproc::contrast::threshold;
use lazy_static::lazy_static;
use tokio::spawn;

use crate::ocr_server;
use crate::ocr_server::OcrClient;

lazy_static! {
    static ref OCR_CLIENT: Arc<Mutex<OcrClient>> = {
        let (_server1_handle, _server2_handle, ocr_client) = ocr_server::run_server();
        Arc::new(Mutex::new(ocr_client))
    };
}

/// 卡池类型
pub enum BannerType {
    /// 新手
    Novice,
    /// 角色常驻
    StandardCharacter,
    /// 武器常驻
    StandardWeapon,
    /// 角色活动
    LimitedTimeCharacter,
    /// 武器活动
    LimitedTimeWeapon,
}

/// 抽卡物品类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum ItemType {
    /// 角色
    Character,
    /// 武器
    Weapon,
}

/// 抽卡记录
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct GachaRecord {
    star: u8,
    item_name: String,
    item_type: ItemType,
    timestamp: u64,
}

impl GachaRecord {
    fn new(star: u8, item_name: String, item_type: ItemType, timestamp: u64) -> Self {
        Self {
            star,
            item_name,
            item_type,
            timestamp,
        }
    }

    fn readable_item_type_str(&self) -> &str {
        match self.item_type {
            ItemType::Character => "角色",
            ItemType::Weapon => "武器",
        }
    }

    fn readable_date_time_str(&self) -> String {
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
    fn star(&self) -> anyhow::Result<u8> {
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
    async fn item_name(&self) -> anyhow::Result<String> {
        let item_name_img = self.item_name_image();
        let item_name_rx = OCR_CLIENT.lock().unwrap().send(item_name_img);
        item_name_rx.await?
    }

    /// 物品类型图片
    fn item_type_image(&self) -> DynamicImage {
        self.img.crop_imm(530, 0, 55, self.img.height())
    }

    /// 物品类型
    async fn item_type(&self) -> anyhow::Result<ItemType> {
        let item_type_img = self.item_type_image();
        let item_type_rx = OCR_CLIENT.lock().unwrap().send(item_type_img);
        let item_type_str = item_type_rx.await??;
        match item_type_str.as_str() {
            "角色" => Ok(ItemType::Character),
            "武器" => Ok(ItemType::Weapon),
            _ => Err(anyhow!("Invalid item type: {}", item_type_str)),
        }
    }

    /// 日期图片
    fn date_image(&self) -> DynamicImage {
        self.img.crop_imm(1020, 0, 120, self.img.height())
    }

    /// 日期字符串
    async fn date_string(&self) -> anyhow::Result<String> {
        let date_img = self.date_image();
        let date_rx = OCR_CLIENT.lock().unwrap().send(date_img);
        date_rx.await?
    }

    /// 时间图片
    fn time_image(&self) -> DynamicImage {
        self.img.crop_imm(1141, 0, 63, self.img.height())
    }

    /// 时间字符串
    async fn time_string(&self) -> anyhow::Result<String> {
        let time_img = self.time_image();
        let time_rx = OCR_CLIENT.lock().unwrap().send(time_img);
        time_rx.await?
    }

    /// 时间戳
    async fn timestamp(&self) -> anyhow::Result<u64> {
        let date_str = self.date_string().await?;
        let time_str = self.time_string().await?;
        let date_time_str = format!("{} {}", date_str, time_str);
        let date_time = chrono::NaiveDateTime::parse_from_str(&date_time_str, "%Y-%m-%d %H:%M")
            .map_err(|e| anyhow!("Failed to parse date time: {:?}", e))?;
        let local_date_time = Local
            .from_local_datetime(&date_time)
            .single()
            .ok_or_else(|| anyhow!("Invalid local date time: {}", date_time_str))?;
        Ok(local_date_time.timestamp() as u64)
    }

    /// 抽卡记录
    async fn gacha_record(&self) -> anyhow::Result<GachaRecord> {
        let star = self.star()?;
        let item_name = self.item_name().await?;
        let item_type = self.item_type().await?;
        let timestamp = self.timestamp().await?;
        Ok(GachaRecord::new(star, item_name, item_type, timestamp))
    }
}

/// 合并抽卡记录
/// 两个抽卡记录按时间顺序合并
pub fn merge_gacha_records(
    new_records: &[GachaRecord],
    old_records: &[GachaRecord],
) -> Vec<GachaRecord> {
    // 抽卡记录是按时间倒序排列的，最新的在最前面
    if new_records.is_empty() {
        return old_records.to_vec();
    }
    if old_records.is_empty() {
        return new_records.to_vec();
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

    merged_records
}

pub struct RecordScreen {
    img: DynamicImage,
}

impl RecordScreen {
    pub fn new(img: DynamicImage) -> Self {
        Self { img }
    }

    fn index_image(&self) -> DynamicImage {
        let img = self.img.crop_imm(1635, 490, 60, 60);
        let img = threshold(&img.to_luma8(), 200);
        DynamicImage::ImageLuma8(img)
    }

    async fn index_string(&self) -> anyhow::Result<String> {
        let index_img = self.index_image();
        let index_rx = OCR_CLIENT.lock().unwrap().send(index_img);
        index_rx.await?
    }

    pub async fn index(&self) -> anyhow::Result<u32> {
        let index_str = self.index_string().await?;
        let index = index_str
            .parse()
            .map_err(|e| anyhow!("Failed to parse index: {:?}", e))?;
        Ok(index)
    }

    fn record_images(&self) -> anyhow::Result<Vec<DynamicImage>> {
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
            .collect::<anyhow::Result<Vec<DynamicImage>>>()
    }

    fn one_record_images(&self) -> anyhow::Result<Vec<OneRecordImage>> {
        let record_images = self.record_images()?;
        let one_record_images = record_images
            .iter()
            .map(|record_image| OneRecordImage::new(record_image.clone()))
            .collect::<Vec<OneRecordImage>>();
        Ok(one_record_images)
    }

    pub async fn gacha_records(&self) -> anyhow::Result<Vec<GachaRecord>> {
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
            .collect::<anyhow::Result<Vec<GachaRecord>>>()
    }
}
