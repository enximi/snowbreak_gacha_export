use anyhow::{anyhow, Result};
use chrono::{Local, TimeZone};
use enum_iterator::all;
use image::{DynamicImage, GenericImageView, GrayImage};
use imageproc::contrast::{otsu_level, threshold, ThresholdType};
use lazy_static::lazy_static;
use simple_ocr::ocr;

use crate::record::{ItemType, OneRecord};

static _MAX_RECORD_NUM: u32 = 10;

static _FIRST_RECORD_Y0: u32 = 207;
static _LAST_RECORD_Y0: u32 = 814;
static _LAST_RECORD_Y1: u32 = 846;
// MAX_RECORD_NUM*RECORD_HEIGHT + (MAX_RECORD_NUM-1)*SPACE_HEIGHT = _LAST_RECORD_Y1 - FIRST_RECORD_Y0
// (MAX_RECORD_NUM-1)*RECORD_HEIGHT + (MAX_RECORD_NUM-1)*SPACE_HEIGHT = _LAST_RECORD_Y0 - FIRST_RECORD_Y0
static RECORD_HEIGHT: u32 = 32;
static SPACE_HEIGHT: f32 = 319.0 / 9.0;

static STAR_X: u32 = 352;
static ITEM_NAME_X0: u32 = 367;
static ITEM_NAME_X1: u32 = 883;
static ITEM_TYPE_X0: u32 = 883;
static TIME_X1: u32 = 1548;
static ITEM_TYPE_X1: u32 = (ITEM_TYPE_X0 + TIME_X1 + 1) / 2;
static TIME_X0: u32 = ITEM_TYPE_X1;

static _PAGE_BUTTON_X: u32 = 1664;

static INDEX_X0: u32 = 1577;
static INDEX_X1: u32 = (_PAGE_BUTTON_X - INDEX_X0) + _PAGE_BUTTON_X;
static INDEX_Y0: u32 = 464;
static INDEX_Y1: u32 = 577;

static _OCR_IMAGE_HEIGHT: u32 = 32;
/// 字符与图片边界的间距
static CHAR_MARGIN: u32 = 7;
static CHAR_HEIGHT: u32 = _OCR_IMAGE_HEIGHT - 2 * CHAR_MARGIN;

lazy_static! {
    static ref RECORD_Y0S: Vec<u32> = (0.._MAX_RECORD_NUM)
        .map(
            |i| ((RECORD_HEIGHT as f32 + SPACE_HEIGHT) * i as f32).round() as u32
                + _FIRST_RECORD_Y0
        )
        .collect();
    static ref RECORD_Y1S: Vec<u32> = RECORD_Y0S.iter().map(|y0| y0 + RECORD_HEIGHT).collect();
    static ref STAR_YS: Vec<u32> = RECORD_Y0S
        .iter()
        .map(|y0| y0 + (RECORD_HEIGHT + 1) / 2)
        .collect();
}

#[derive(Clone)]
pub struct RecordImage {
    pub image: DynamicImage,
}

impl RecordImage {
    pub fn new(image: DynamicImage) -> Self {
        assert_eq!(1920, image.width());
        assert_eq!(1080, image.height());
        Self { image }
    }

    pub fn is_record_image(&self) -> bool {
        if self.stars().is_empty() {
            return false;
        }
        if self.index().is_err() {
            return false;
        }
        true
    }

    fn stars(&self) -> Vec<u8> {
        /// RGB 颜色转换为星级
        fn rgb_to_star(rgb: (u8, u8, u8)) -> Result<u8> {
            /// 计算两个 RGB 颜色的欧氏距离
            fn rgb_distance(rgb1: (u8, u8, u8), rgb2: (u8, u8, u8)) -> f32 {
                let r = (rgb1.0 as f32 - rgb2.0 as f32).powi(2);
                let g = (rgb1.1 as f32 - rgb2.1 as f32).powi(2);
                let b = (rgb1.2 as f32 - rgb2.2 as f32).powi(2);
                (r + g + b).sqrt()
            }

            static STAR_3_RGB: (u8, u8, u8) = (55, 98, 242);
            static STAR_4_RGB: (u8, u8, u8) = (192, 105, 214);
            static STAR_5_RGB: (u8, u8, u8) = (233, 155, 55);

            static ACCURACY: f32 = 5.0;

            if rgb_distance(rgb, STAR_3_RGB) < ACCURACY {
                Ok(3)
            } else if rgb_distance(rgb, STAR_4_RGB) < ACCURACY {
                Ok(4)
            } else if rgb_distance(rgb, STAR_5_RGB) < ACCURACY {
                Ok(5)
            } else {
                Err(anyhow!("Unknown star RGB: {:?}", rgb))
            }
        }

        STAR_YS
            .iter()
            .map_while(|&y| {
                let rgba = self.image.get_pixel(STAR_X, y);
                let rgb = (rgba[0], rgba[1], rgba[2]);
                rgb_to_star(rgb).ok()
            })
            .collect()
    }

    /// 传入包含字符的区域的左上角和右下角坐标，返回用于 OCR 的图片。
    /// # 参数
    /// - x0: 左上角 x 坐标
    /// - y0: 左上角 y 坐标
    /// - x1: 右下角 x 坐标
    /// - y1: 右下角 y 坐标
    /// # 返回
    /// 用于 OCR 的图片
    fn get_ocr_image(&self, x0: u32, y0: u32, x1: u32, y1: u32) -> GrayImage {
        /// 找出图片中字符的区域
        fn get_char_xywh(image: DynamicImage) -> (u32, u32, u32, u32) {
            let image = image.to_luma8();
            let otsu = otsu_level(&image);
            let image = threshold(&image, otsu, ThresholdType::Binary);
            let (x_min, x_max, y_min, y_max) = image.enumerate_pixels().fold(
                (image.width() - 1, 0, image.height() - 1, 0),
                |(x_min, x_max, y_min, y_max), (x, y, pixel)| {
                    if pixel[0] == 0 {
                        (x_min.min(x), x_max.max(x), y_min.min(y), y_max.max(y))
                    } else {
                        (x_min, x_max, y_min, y_max)
                    }
                },
            );
            (x_min, y_min, x_max - x_min + 1, y_max - y_min + 1)
        }

        /// 通过字符的高度计算字符与图片边界应该的间距
        fn calculate_char_margin(char_height: u32) -> u32 {
            (char_height as f32 / CHAR_HEIGHT as f32 * CHAR_MARGIN as f32).round() as u32
        }

        // 1. 裁剪包含字符区域的图片
        // 2. 找出字符的区域
        // 3. 计算字符与图片边界的间距
        // 4. 从原图裁剪出用于 OCR 的图片
        let image = self.image.crop_imm(x0, y0, x1 - x0, y1 - y0);
        let (x, y, w, h) = get_char_xywh(image);
        let char_margin = calculate_char_margin(h);
        let x = x0 + x - char_margin;
        let y = y0 + y - char_margin;
        let w = w + 2 * char_margin;
        let h = h + 2 * char_margin;
        self.image.crop_imm(x, y, w, h).to_luma8()
    }

    fn index_ocr_image(&self) -> GrayImage {
        self.get_ocr_image(INDEX_X0, INDEX_Y0, INDEX_X1, INDEX_Y1)
    }

    fn item_name_ocr_image(&self, index: usize) -> GrayImage {
        let y0 = RECORD_Y0S[index];
        let y1 = RECORD_Y1S[index];
        self.get_ocr_image(ITEM_NAME_X0, y0, ITEM_NAME_X1, y1)
    }

    fn item_type_ocr_image(&self, index: usize) -> GrayImage {
        let y0 = RECORD_Y0S[index];
        let y1 = RECORD_Y1S[index];
        self.get_ocr_image(ITEM_TYPE_X0, y0, ITEM_TYPE_X1, y1)
    }

    fn time_ocr_image(&self, index: usize) -> GrayImage {
        let y0 = RECORD_Y0S[index];
        let y1 = RECORD_Y1S[index];
        self.get_ocr_image(TIME_X0, y0, TIME_X1, y1)
    }

    fn index_str(&self) -> String {
        let image = self.index_ocr_image();
        ocr(DynamicImage::ImageLuma8(image)).0
    }

    fn item_name_str(&self, index: usize) -> String {
        let image = self.item_name_ocr_image(index);
        ocr(DynamicImage::ImageLuma8(image)).0
    }

    fn item_type_str(&self, index: usize) -> String {
        let image = self.item_type_ocr_image(index);
        ocr(DynamicImage::ImageLuma8(image)).0
    }

    fn time_str(&self, index: usize) -> String {
        let image = self.time_ocr_image(index);
        ocr(DynamicImage::ImageLuma8(image)).0
    }

    pub fn index(&self) -> Result<u32> {
        self.index_str()
            .parse()
            .map_err(|e| anyhow!("Failed to parse index, {:?}", e))
    }

    fn item_type(&self, index: usize) -> Result<ItemType> {
        let item_type = self.item_type_str(index);
        all::<ItemType>()
            .find(|&item| {
                item.display_names_in_record_page_in_game_in_all_languages()
                    .contains(&item_type.as_str())
            })
            .ok_or(anyhow!("Unknown item type: {}", item_type))
    }

    fn timestamp(&self, index: usize) -> Result<u64> {
        let time_str = self.time_str(index);
        let time = chrono::NaiveDateTime::parse_from_str(&time_str, "%Y-%m-%d %H:%M")
            .map_err(|e| anyhow!("Failed to parse date time: {:?}", e))?;
        let local_date_time = Local
            .from_local_datetime(&time)
            .single()
            .ok_or(anyhow!("Invalid local date time: {}", time_str))?;
        Ok(local_date_time.timestamp() as u64)
    }

    pub fn records(&self) -> Vec<OneRecord> {
        let stars = self.stars();
        stars
            .into_iter()
            .enumerate()
            .map(|(i, star)| {
                let item_name = self.item_name_str(i);
                let item_type = self.item_type(i).unwrap();
                let time = self.timestamp(i).unwrap();
                OneRecord::new(star, item_name, item_type, time)
            })
            .collect()
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;
    use std::time::Instant;

    use super::*;

    #[test]
    fn test() {
        let image_dir = "not_in_git/images";
        // let image_dir = r"D:\PortableSoftware\ShareX\ShareX\Screenshots\2024-06";
        let image_file_paths = Path::new(image_dir)
            .read_dir()
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        let image_file_paths = vec![Path::new("not_in_git/images/image_7.png")];
        for image_file_path in image_file_paths {
            let image = image::open(&image_file_path).unwrap();
            let record_image = RecordImage::new(image);
            println!("image: {:?}", image_file_path);
            if !record_image.is_record_image() {
                println!("Not a record image");
                continue;
            }
            println!("index: {}", record_image.index().unwrap());
            let start = Instant::now();
            let records = record_image.records();
            println!("ocr records spend: {:?}", start.elapsed());
            for record in records {
                println!("{} {}", record.item_name, record.readable_date_time_str());
            }
        }
    }
}
