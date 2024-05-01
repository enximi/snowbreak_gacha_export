use crate::record::{BannerType, GachaRecord, GachaRecords};
use enum_iterator::all;
use rust_xlsxwriter::{Format, Workbook};
use std::collections::HashMap;

/// Save the records to an excel file.
pub fn save_excel(records: HashMap<BannerType, Vec<GachaRecord>>) {
    let mut workbook = Workbook::new();
    // 五星格式
    let format_5_star = Format::new().set_background_color(0xe99b37);
    // 四星格式
    let format_4_star = Format::new().set_background_color(0xc069d6);
    // 其他格式
    let format_other = Format::new();
    all::<BannerType>().for_each(|banner_type: BannerType| {
        let worksheet = workbook.add_worksheet();
        worksheet
            .set_name(banner_type.chinese_display_name())
            .unwrap();
        worksheet.write(0, 0, "品质").unwrap();
        worksheet.set_column_width(0, 5).unwrap();
        worksheet.write(0, 1, "名称").unwrap();
        worksheet.set_column_width(1, 20).unwrap();
        worksheet.write(0, 2, "类型").unwrap();
        worksheet.set_column_width(2, 5).unwrap();
        worksheet.write(0, 3, "时间").unwrap();
        worksheet.set_column_width(3, 20).unwrap();
        worksheet.write(0, 4, "5星保底内抽数").unwrap();
        worksheet.set_column_width(4, 14).unwrap();
        worksheet.write(0, 5, "距5星保底还剩").unwrap();
        worksheet.set_column_width(5, 14).unwrap();
        worksheet.write(0, 6, "4星保底内抽数").unwrap();
        worksheet.set_column_width(6, 14).unwrap();
        // worksheet.write(0, 7, "距4星保底还剩").unwrap();
        // worksheet.set_column_width(7, 14).unwrap();
        let records = GachaRecords::new(
            banner_type,
            records.get(&banner_type).unwrap_or(&vec![]).clone(),
        );
        let length = records.records().len();
        for i in 0..length {
            let record = &records.records()[i];
            let i = i as u32;
            let format = match record.star {
                5 => &format_5_star,
                4 => &format_4_star,
                _ => &format_other,
            };
            worksheet
                .write_with_format(i + 1, 0, record.star, format)
                .unwrap();
            worksheet
                .write_with_format(i + 1, 1, record.item_name.clone(), format)
                .unwrap();
            worksheet
                .write_with_format(i + 1, 2, record.item_type.chinese_display_name(), format)
                .unwrap();
            worksheet
                .write_with_format(i + 1, 3, record.readable_date_time_str(), format)
                .unwrap();
            worksheet
                .write_with_format(i + 1, 4, records.count_after_5_star(i), format)
                .unwrap();
            worksheet
                .write_with_format(i + 1, 5, records.count_to_5_star_pity(i), format)
                .unwrap();
            worksheet
                .write_with_format(i + 1, 6, records.count_after_4_star(i), format)
                .unwrap();
            // worksheet
            //     .write_with_format(i + 1, 7, records.count_to_4_star_pity(i), format)
            //     .unwrap();
        }
    });
    workbook.save("gacha_records.xlsx").unwrap()
}

pub fn get_gache_records_from_file(banner_type: BannerType) -> Vec<GachaRecord> {
    let file_name = format!("gacha_records/{}.csv", banner_type.save_file_name());
    if !std::path::Path::new(&file_name).exists() {
        vec![]
    } else {
        let mut rdr = csv::Reader::from_path(file_name).unwrap();
        let mut records = vec![];
        for result in rdr.deserialize() {
            let record: GachaRecord = result.unwrap();
            records.push(record);
        }
        records
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_save_excel() {
        let mut records = HashMap::new();
        all::<BannerType>().for_each(|banner_type: BannerType| {
            records.insert(banner_type, get_gache_records_from_file(banner_type));
        });
        save_excel(records);
    }
}
