use rust_xlsxwriter::{Format, Workbook};

use crate::language::Language;
use crate::record::{BannerType, OneRecord, TotalRecords};

fn headers(language: Language) -> Vec<&'static str> {
    match language {
        Language::ChineseSimplified => {
            vec!["品质", "名称", "类型", "时间", "5星后", "5星保底", "4星后"]
        }
        Language::English => vec![
            "Star", "Name", "Type", "Time", "After 5*", "5* Pity", "After 4*",
        ],
    }
}

fn get_other_data(
    one_records: Vec<OneRecord>,
    banner_type: BannerType,
) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let mut count_after_5_star = 1;
    let mut count_after_4_star = 1;
    let mut counts_after_5_star = vec![];
    let mut counts_after_4_star = vec![];
    for one_record in one_records.iter().rev() {
        counts_after_5_star.push(count_after_5_star);
        counts_after_4_star.push(count_after_4_star);
        if one_record.star == 5 {
            count_after_4_star += 1;
            count_after_5_star = 1;
        } else if one_record.star == 4 {
            count_after_4_star = 1;
            count_after_5_star += 1;
        } else {
            count_after_5_star += 1;
            count_after_4_star += 1;
        }
    }
    let counts_to_5_star_pity = counts_after_5_star
        .iter()
        .map(|count| banner_type.pity_count() - count)
        .collect::<Vec<_>>();
    let counts_after_5_star = counts_after_5_star.into_iter().rev().collect();
    let counts_to_5_star_pity = counts_to_5_star_pity.into_iter().rev().collect();
    let counts_after_4_star = counts_after_4_star.into_iter().rev().collect();
    (
        counts_after_5_star,
        counts_to_5_star_pity,
        counts_after_4_star,
    )
}

// Save the records to an Excel file.
pub fn save_excel(total_records: TotalRecords, language: Language) {
    let mut workbook = Workbook::new();
    // 五星格式
    let format_5_star = Format::new().set_background_color(0xe99b37);
    // 四星格式
    let format_4_star = Format::new().set_background_color(0xc069d6);
    // 其他格式
    let format_other = Format::new();

    for (account_id, account_record) in total_records.records {
        for (banner_type, one_records) in account_record.records {
            let worksheet = workbook.add_worksheet();
            worksheet
                .set_name(format!(
                    "{}-{}",
                    account_id,
                    banner_type.display_name_for_user(language)
                ))
                .unwrap();
            let headers = headers(language);
            let colum_widths = [5, 20, 5, 20, 8, 8, 8];
            for i in 0..headers.len() {
                worksheet.write(0, i as u16, headers[i]).unwrap();
                worksheet
                    .set_column_width(i as u16, colum_widths[i])
                    .unwrap();
            }
            let (counts_after_5_star, counts_to_5_star_pity, counts_after_4_star) =
                get_other_data(one_records.clone(), banner_type);
            one_records
                .iter()
                .zip(counts_after_5_star)
                .zip(counts_to_5_star_pity)
                .zip(counts_after_4_star)
                .enumerate()
                .for_each(
                    |(
                        i,
                        (
                            ((one_record, count_after_5_star), count_to_5_star_pity),
                            count_after_4_star,
                        ),
                    )| {
                        let format = match one_record.star {
                            5 => &format_5_star,
                            4 => &format_4_star,
                            _ => &format_other,
                        };
                        let i = i as u32;
                        worksheet
                            .write_with_format(i + 1, 0, one_record.star, format)
                            .unwrap();
                        worksheet
                            .write_with_format(i + 1, 1, one_record.item_name.clone(), format)
                            .unwrap();
                        worksheet
                            .write_with_format(
                                i + 1,
                                2,
                                one_record.item_type.display_name_for_user(language),
                                format,
                            )
                            .unwrap();
                        worksheet
                            .write_with_format(
                                i + 1,
                                3,
                                one_record.readable_date_time_str(),
                                format,
                            )
                            .unwrap();
                        worksheet
                            .write_with_format(i + 1, 4, count_after_5_star, format)
                            .unwrap();
                        worksheet
                            .write_with_format(i + 1, 5, count_to_5_star_pity, format)
                            .unwrap();
                        worksheet
                            .write_with_format(i + 1, 6, count_after_4_star, format)
                            .unwrap();
                    },
                );
            workbook.save("records.xlsx").unwrap()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_save_excel() {
        let total_records = TotalRecords::read_or_default();
        save_excel(total_records, Language::ChineseSimplified);
    }
}
