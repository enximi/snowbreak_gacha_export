use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use admin_runner::{is_admin, run_as_admin};
use anyhow::{anyhow, Result};
use enigo::Coordinate::Abs;
use enigo::{Button, Direction::Click, Enigo, Mouse, Settings};
use enum_iterator::all;
use image::DynamicImage;
use lazy_static::lazy_static;
use tokio::spawn;
use tokio::time::sleep;
use window_inspector::find::get_hwnd_ref_cache;
use window_inspector::position_size::{get_client_xywh, get_window_xywh_include_shadow};
use window_inspector::top_most::{cancel_window_top_most, set_window_top_most};
use xcap::Window;

use crate::record::{merge_gacha_records, BannerType, GachaRecord, RecordScreen};
use crate::save::{get_gache_records_from_file, save_excel};

mod ocr_server;
mod record;
mod save;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("仅支持1920x1080窗口化/无边框");
    println!("先打开抽卡记录界面，后运行本程序");

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
                wait_any_key();
                return;
            }
        };
    }

    let (hwnd, _, _) = match get_game_window_info() {
        Ok((hwnd, title, remark)) => {
            log::info!("Game: {}, window title: {}", remark, title);
            (hwnd, title, remark)
        }
        Err(e) => {
            log::error!("failed to get game window info: {:?}", e);
            wait_any_key();
            return;
        }
    };

    println!(
        "按下数字键选择卡池类型（1：角色活动，2：武器活动，3：角色常驻，4：武器常驻，5：新手）"
    );

    consume_all_events();
    let user_selected_banner_type = loop {
        if let Ok(crossterm::event::Event::Key(event)) = crossterm::event::read() {
            match event.code {
                crossterm::event::KeyCode::Char('1') => break BannerType::LimitedTimeCharacter,
                crossterm::event::KeyCode::Char('2') => break BannerType::LimitedTimeWeapon,
                crossterm::event::KeyCode::Char('3') => break BannerType::StandardCharacter,
                crossterm::event::KeyCode::Char('4') => break BannerType::StandardWeapon,
                crossterm::event::KeyCode::Char('5') => break BannerType::Novice,
                _ => {
                    log::warn!("Invalid input: {:?}", event.code);
                    println!("Invalid input, please input again");
                }
            }
        }
    };

    log::info!("Selected banner type: {:?}", user_selected_banner_type);

    // 游戏窗口置顶
    set_window_top_most(hwnd).unwrap();
    cancel_window_top_most(hwnd).unwrap();

    let windows = Window::all().unwrap();
    let window = windows
        .iter()
        .find(|window| window.id() == hwnd as u32)
        .unwrap();

    let mut record_screens = vec![];

    let img = client_img(window).unwrap();
    let record_screen = RecordScreen::new(img);
    match record_screen.index().await {
        Ok(index) => {
            if index == 1 {
                record_screens.push(record_screen);
                log::info!("now in the screen 1");
            } else {
                log::warn!("now in the screen {}", index);
                log::info!("click to back to the screen 1");
                // back to the first screen
                let mut click_time = 0;
                loop {
                    click_to_change_page(hwnd, false);
                    sleep(Duration::from_millis(200)).await;
                    click_time += 1;
                    let img = client_img(window).unwrap();
                    let record_screen = RecordScreen::new(img);
                    let index = record_screen.index().await.unwrap();
                    if index == 1 {
                        log::info!("back to the first screen");
                        record_screens.push(record_screen.clone());
                        break;
                    }
                    if click_time > 20 {
                        log::error!("Failed to back to the first screen");
                        wait_any_key();
                        return;
                    }
                }
            }
        }
        Err(e) => {
            log::error!(
                "Failed to get index: {:?}, may not in gacha record interface",
                e
            );
            wait_any_key();
            return;
        }
    };

    'outer: loop {
        click_to_change_page(hwnd, true);
        sleep(Duration::from_millis(200)).await;
        let start = Instant::now();
        loop {
            let img = client_img(window).unwrap();
            let record_screen = RecordScreen::new(img);
            let index = record_screen.index().await.unwrap();
            log::debug!("index: {}", index);
            if index == record_screens.len() as u32 + 1 {
                record_screens.push(record_screen);
                break;
            }
            if start.elapsed().as_secs_f32() > 1.0 {
                log::info!("now in the last screen");
                break 'outer;
            }
        }
    }

    log::debug!("record_screens.len(): {}", record_screens.len());

    log::info!("ocring...");
    let start = Instant::now();
    let handles = record_screens
        .into_iter()
        .map(|record_screen| spawn(async move { record_screen.gacha_records().await }))
        .collect::<Vec<_>>();
    let results = futures::future::join_all(handles).await;
    log::info!("ocr spent {:?}", start.elapsed());
    let gacha_records = results
        .into_iter()
        .flat_map(|result| result.unwrap().unwrap())
        .collect::<Vec<GachaRecord>>();

    // 读取保存的抽卡记录
    let save_floder = "gacha_records";
    // 文件夹是否存在
    if !std::path::Path::new(save_floder).exists() {
        std::fs::create_dir(save_floder).unwrap();
    }
    // 根据卡池类型获取文件名
    let file_name = match user_selected_banner_type {
        BannerType::LimitedTimeCharacter => "limited_time_character",
        BannerType::LimitedTimeWeapon => "limited_time_weapon",
        BannerType::StandardCharacter => "standard_character",
        BannerType::StandardWeapon => "standard_weapon",
        BannerType::Novice => "novice",
    };
    let old_gacha_records = get_gache_records_from_file(user_selected_banner_type);

    // 合并抽卡记录
    let merged_gacha_records = match merge_gacha_records(&gacha_records, &old_gacha_records) {
        Ok((merged_gacha_records, new_gacha_records_count)) => {
            log::info!(
                "Merge gacha records successfully, add {} new gacha records",
                new_gacha_records_count
            );
            merged_gacha_records
        }
        Err(e) => {
            log::error!("Failed to merge gacha records: {:?}", e);
            log::warn!("Save new gacha records only");
            // log 文件夹是否存在
            if !std::path::Path::new("log").exists() {
                std::fs::create_dir("log").unwrap();
            }
            let now_timestamp = chrono::Local::now().timestamp();
            // 旧抽卡记录保存路径
            let old_gacha_records_save_path =
                format!("log/{}_{}_old.csv", now_timestamp, file_name);
            // 此次扫描的抽卡记录保存路径
            let this_time_gacha_records_save_path =
                format!("log/{}_{}_this_time.csv", now_timestamp, file_name);
            // 保存旧的抽卡记录
            let mut writer = csv::Writer::from_path(old_gacha_records_save_path.clone()).unwrap();
            for record in &old_gacha_records {
                writer.serialize(record).unwrap();
            }
            writer.flush().unwrap();
            log::info!("Save old gacha records to {}", old_gacha_records_save_path);
            // 保存这一次扫描的抽卡记录
            let mut writer =
                csv::Writer::from_path(this_time_gacha_records_save_path.clone()).unwrap();
            for record in &gacha_records {
                writer.serialize(record).unwrap();
            }
            writer.flush().unwrap();
            log::info!(
                "Save this time gacha records to {}",
                this_time_gacha_records_save_path
            );
            wait_any_key();
            return;
        }
    };

    // 保存抽卡记录
    let mut writer = csv::Writer::from_path(format!("{}/{}.csv", save_floder, file_name)).unwrap();
    for record in &merged_gacha_records {
        writer.serialize(record).unwrap();
    }
    writer.flush().unwrap();
    log::info!("Save gacha records to {}/{}.csv", save_floder, file_name);

    // 再保存一份带时间戳的抽卡记录
    let timestamp = chrono::Local::now().timestamp();
    let mut writer =
        csv::Writer::from_path(format!("{}/{}_{}.csv", save_floder, file_name, timestamp)).unwrap();
    for record in &merged_gacha_records {
        writer.serialize(record).unwrap();
    }
    writer.flush().unwrap();
    log::info!(
        "Save gacha records to {}/{}_{}.csv",
        save_floder,
        file_name,
        timestamp
    );

    let mut records = HashMap::new();
    all::<BannerType>().for_each(|banner_type: BannerType| {
        if banner_type != user_selected_banner_type {
            records.insert(banner_type, get_gache_records_from_file(banner_type));
        }
    });
    records.insert(user_selected_banner_type, merged_gacha_records);

    save_excel(records);
    log::info!("Save excel to gacha_records.xlsx");

    // 按任意键退出
    wait_any_key();
}

fn get_game_window_info() -> Result<(isize, String, String)> {
    let window_class = "UnrealWindow";
    let possible_window_titles = ["尘白禁区", "Snowbreak: Containment Zone"];
    let window_titles_remarks = ["中国服", "国际服"];

    for (title, remark) in possible_window_titles
        .iter()
        .zip(window_titles_remarks.iter())
    {
        if let Ok(hwnd) = get_hwnd_ref_cache(window_class, title) {
            return Ok((hwnd, title.to_string(), remark.to_string()));
        }
    }

    Err(anyhow!("Failed to get game window info"))
}

/// 消耗所有已经存在的事件
fn consume_all_events() {
    while crossterm::event::poll(Duration::from_millis(10)).unwrap() {
        let _ = crossterm::event::read().unwrap();
    }
}

fn wait_any_key() {
    // 使用 cargo run 启动时，会有一个 enter 事件
    consume_all_events();

    println!("press any key to exit");
    // 等待任意一个键
    loop {
        if let Ok(crossterm::event::Event::Key(_)) = crossterm::event::read() {
            break;
        }
    }
}

fn client_img(window: &Window) -> Result<DynamicImage> {
    let img = window.capture_image()?;
    let window_xywh = get_window_xywh_include_shadow(window.id() as isize)?;
    // check window size
    let img_size = (img.width(), img.height());
    if img_size.0 != window_xywh.2 || img_size.1 != window_xywh.3 {
        return Err(anyhow!(
            "window size not match, window_xywh: {:?}, img_wh: {:?}",
            window_xywh,
            img_size
        ));
    }
    let client_xywh = get_client_xywh(window.id() as isize)?;
    // check client in window
    if client_xywh.0 < window_xywh.0
        || client_xywh.1 < window_xywh.1
        || client_xywh.0 + client_xywh.2 as i32 > window_xywh.0 + window_xywh.2 as i32
        || client_xywh.1 + client_xywh.3 as i32 > window_xywh.1 + window_xywh.3 as i32
    {
        return Err(anyhow!("client out of window"));
    }
    let img = DynamicImage::ImageRgba8(img).crop_imm(
        (client_xywh.0 - window_xywh.0) as u32,
        (client_xywh.1 - window_xywh.1) as u32,
        client_xywh.2,
        client_xywh.3,
    );
    let img_size = (img.width(), img.height());
    let ratio = num_rational::Ratio::new(img_size.0 as i64, img_size.1 as i64);
    if ratio != num_rational::Ratio::new(16, 9) {
        return Err(anyhow!("image ratio not 16:9, ratio: {:?}", ratio));
    }
    let img = if img_size.0 != 1920 {
        img.resize_exact(1920, 1080, image::imageops::FilterType::Nearest)
    } else {
        img
    };
    Ok(img)
}

fn click_to_change_page(hwnd: isize, next_page: bool) {
    let click_xy_1080 = if next_page { (1665, 616) } else { (1665, 425) };
    let client_xywh = get_client_xywh(hwnd).unwrap();
    let click_xy = (
        ((click_xy_1080.0 * client_xywh.2 + client_xywh.2 / 2) / 1920) as i32 + client_xywh.0,
        ((click_xy_1080.1 * client_xywh.3 + client_xywh.3 / 2) / 1080) as i32 + client_xywh.1,
    );
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    enigo.move_mouse(click_xy.0, click_xy.1, Abs).unwrap();
    enigo.button(Button::Left, Click).unwrap();
}
