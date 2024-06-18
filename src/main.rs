use std::time::Duration;
use std::time::Instant;

use admin_runner::is_admin;
use admin_runner::run_as_admin;
use tokio::time::sleep;
use window_inspector::top_most::cancel_window_top_most;
use window_inspector::top_most::set_window_top_most;

use crate::action::{next_page, previous_page};
use crate::capture::{capture_image, init_capture, release_capture};
use crate::config::CONFIG;
use crate::game_info::get_game_window_info;
use crate::language::Language;
use crate::record::TotalRecords;
use crate::record_image::RecordImage;
use crate::save::save_excel;
use crate::update::is_up_to_date;
use crate::user_interaction::{banner_type, wait_enter};

mod action;
mod capture;
mod config;
mod game_info;
mod language;
mod record;
mod record_image;
mod save;
mod update;
mod user_interaction;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("snowbreak_gacha_export=info"),
    )
    .init();

    let language = CONFIG.language;

    // 检查更新
    match is_up_to_date().await {
        Ok((is_up_to_date, latest_version)) => {
            if !is_up_to_date {
                log::warn!("New version available: {}", latest_version);
                let tip = match language {
                    Language::ChineseSimplified => "有新版本，请前往 https://github.com/enximi/snowbreak_gacha_export/releases 更新",
                    Language::English => "New version available, please update in https://github.com/enximi/snowbreak_gacha_export/releases",
                };
                println!("{}", tip);
            } else {
                log::info!("Already up to date, version: {}", env!("CARGO_PKG_VERSION"));
            }
        }
        Err(_) => {
            log::error!("Failed to check update");
        }
    }

    // 用户提示
    let tip = match language {
        Language::ChineseSimplified => {
            "仅支持 16:9 窗口化/无边框\n先打开抽卡记录界面，后运行本程序"
        }
        Language::English => {
            "Only support 16:9 windowed/borderless\nOpen the gacha record interface first, then run this program"
        }
    };
    println!("{}", tip);

    // 管理员权限
    if is_admin() {
        log::info!("run as admin");
    } else {
        log::warn!("not run as admin, rerun as admin");
        match run_as_admin() {
            Ok(_) => {
                log::info!("rerun as admin successfully");
                println!("exit in 3s");
                sleep(Duration::from_secs(3)).await;
                return;
            }
            Err(e) => {
                log::error!("rerun as admin failed: {:?}", e);
                wait_enter(language);
                return;
            }
        };
    }

    // 获取游戏窗口
    let (hwnd, window_title) = match get_game_window_info() {
        Ok((hwnd, title)) => {
            log::info!("window title: {title}");
            (hwnd, title)
        }
        Err(e) => {
            log::error!("failed to get game window info: {:?}", e);
            wait_enter(language);
            return;
        }
    };

    // 选择卡池类型
    let user_selected_banner_type = banner_type(language);
    log::info!("Selected banner type: {:?}", user_selected_banner_type);

    let account_id = "default_account_id";

    // 游戏窗口置顶
    set_window_top_most(hwnd).unwrap();
    cancel_window_top_most(hwnd).unwrap();

    // 创建截图工具
    init_capture(window_title);

    let mut record_images = vec![];

    // 获取第一个界面，如果不是第一个界面，回到第一个界面
    let image = capture_image().unwrap();
    let record_image = RecordImage::new(image);
    if record_image.is_record_image() {
        // 回到第一个界面
        let start = Instant::now();
        let mut record_image = record_image.clone();
        let mut index = record_image.index().unwrap();
        log::debug!("index: {}", index);
        while index != 1 {
            previous_page(hwnd);
            sleep(Duration::from_millis(200)).await;
            let image = capture_image().unwrap();
            record_image = RecordImage::new(image);
            index = record_image.index().unwrap();
            log::debug!("index: {}", index);
            if start.elapsed().as_secs_f32() > 15.0 {
                log::error!("Failed to back to the first record image");
                wait_enter(language);
                return;
            }
        }
        record_images.push(record_image);
    } else {
        log::error!("not in the record interface");
        wait_enter(language);
        return;
    }

    let mut now_index = 1;
    loop {
        next_page(hwnd);
        sleep(Duration::from_millis(200)).await;
        let image = capture_image().unwrap();
        let record_image = RecordImage::new(image);
        if record_image.index().unwrap() == now_index + 1 {
            record_images.push(record_image);
            now_index += 1;
        } else {
            break;
        }
    }

    // 停止截图，释放资源
    release_capture();

    log::debug!("record_screens.len(): {}", record_images.len());

    log::info!("ocring...");
    let start = Instant::now();
    let records = record_images
        .into_iter()
        .flat_map(|record_image| record_image.records())
        .collect::<Vec<_>>();
    log::info!("ocr spend: {:?}", start.elapsed());

    let mut total_record = TotalRecords::read_or_default();
    match total_record.add_record(account_id.to_string(), user_selected_banner_type, records) {
        Ok(add_num) => {
            log::info!("add {} records", add_num);
        }
        Err(e) => {
            log::error!("failed to add records: {:?}", e);
            wait_enter(language);
            return;
        }
    }
    total_record.save().unwrap();

    save_excel(total_record, language);
    
    wait_enter(language);
}
