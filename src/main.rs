use std::io::Write;
use admin_runner::{is_admin, run_as_admin};
use std::time::Instant;

use anyhow::{anyhow, Result};
use client_capture::ClientCapture;
use enigo::Coordinate::Abs;
use enigo::{Button, Direction::Click, Enigo, Mouse, Settings};
use image::DynamicImage;
use tokio::spawn;
use tokio::time::sleep;
use window_inspector::find::get_hwnd_ref_cache;
use window_inspector::foreground::set_foreground_window;
use window_inspector::position_size::get_client_xy;

use crate::record::{merge_gacha_records, BannerType, GachaRecord, RecordScreen};

mod ocr_server;
mod record;

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    if !is_admin() {
        log::warn!("not run as admin, rerun as admin");
        match run_as_admin() {
            Ok(_) => {
                log::info!("rerun as admin successfully");
            }
            Err(e) => {
                log::error!("{:?}", e);
            }
        };
        return;
    }

    println!("仅支持1920x1080窗口化/无边框");
    println!("先打开游戏抽卡记录界面，然后运行本程序");

    let china_window_title = "尘白禁区";
    let global_window_title = "Snowbreak: Containment Zone";
    let window_class = "UnrealWindow";
    
    let (is_global, hwnd) = match get_game_window_info() {
        Ok((is_global, hwnd)) => (is_global, hwnd),
        Err(e) => {
            log::error!("{:?}", e);
            return;
        }
    };
    
    print!("输入数字选择卡池类型（1：角色活动，2：武器活动，3：角色常驻，4：武器常驻，5：新手）：");
    std::io::stdout().flush().unwrap();
    
    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();
    let input = input.trim();
    let banner_type = match input {
        "1" => BannerType::LimitedTimeCharacter,
        "2" => BannerType::LimitedTimeWeapon,
        "3" => BannerType::StandardCharacter,
        "4" => BannerType::StandardWeapon,
        "5" => BannerType::Novice,
        _ => {
            log::error!("Invalid input: {}", input);
            return;
        }
    };

    // sleep(std::time::Duration::from_secs(1)).await;

    set_foreground_window(hwnd).unwrap();

    if is_global {
        log::info!("Global window title: {}", global_window_title);
    } else {
        log::info!("China window title: {}", china_window_title);
    }

    let (mut client_capture, mut client_capture_controller) = ClientCapture::new(
        window_class,
        if is_global {
            global_window_title
        } else {
            china_window_title
        },
        Some(false),
        None,
        None,
    );
    spawn(async move { client_capture.run().await });

    log::info!("sleep 1s, waiting for the client capture to start");
    sleep(std::time::Duration::from_secs(1)).await;

    let mut record_screens = vec![];

    match client_capture_controller.get_img() {
        Some(img) => {
            let record_screen = RecordScreen::new(DynamicImage::ImageRgba8(img.0.as_ref().clone()));
            match record_screen.index().await {
                Ok(index) => {
                    if index == 1 {
                        record_screens.push(record_screen);
                        log::info!("now in the first screen");
                    } else {
                        log::warn!("now in the screen {}", index);
                        log::info!("click to back to the first screen");
                        // back to the first screen
                        let mut click_time = 0;
                        let mut e = Enigo::new(&Settings::default()).unwrap();
                        let screen_size = e.main_display().unwrap();
                        loop {
                            let client_xy = get_client_xy(hwnd).unwrap();
                            e.move_mouse(
                                ((1665.0 + client_xy.0 as f32) / screen_size.0 as f32 * 65535.0)
                                    .round() as i32,
                                ((425.0 + client_xy.1 as f32) / screen_size.1 as f32 * 65535.0)
                                    .round() as i32,
                                Abs,
                            )
                            .unwrap();
                            e.button(Button::Left, Click).unwrap();
                            click_time += 1;
                            sleep(std::time::Duration::from_secs_f32(0.2)).await;
                            let img = client_capture_controller.get_img().unwrap();
                            let record_screen =
                                RecordScreen::new(DynamicImage::ImageRgba8(img.0.as_ref().clone()));
                            let index = record_screen.index().await.unwrap();
                            if index == 1 {
                                record_screens.push(record_screen);
                                log::info!("back to the first screen");
                                break;
                            }
                            if click_time > 20 {
                                log::error!("Failed to back to the first screen");
                                client_capture_controller.stop();
                                return;
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("{:?}", e);
                    return;
                }
            }
        }
        None => {
            log::error!("Failed to get img");
            return;
        }
    };

    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    let screen_size = enigo.main_display().unwrap();
    'outer: loop {
        let client_xy = get_client_xy(hwnd).unwrap();
        enigo
            .move_mouse(
                ((1665.0 + client_xy.0 as f32) / screen_size.0 as f32 * 65535.0).round() as i32,
                ((616.0 + client_xy.1 as f32) / screen_size.1 as f32 * 65535.0).round() as i32,
                Abs,
            )
            .unwrap();
        enigo.button(Button::Left, Click).unwrap();
        sleep(std::time::Duration::from_secs_f32(0.2)).await;
        let start = Instant::now();
        loop {
            let img = client_capture_controller.get_img().unwrap();
            let record_screen = RecordScreen::new(DynamicImage::ImageRgba8(img.0.as_ref().clone()));
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

    client_capture_controller.stop();

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
    let file_name = match banner_type {
        BannerType::LimitedTimeCharacter => "limited_time_character",
        BannerType::LimitedTimeWeapon => "limited_time_weapon",
        BannerType::StandardCharacter => "standard_character",
        BannerType::StandardWeapon => "standard_weapon",
        BannerType::Novice => "novice",
    };
    // 文件是否存在
    let gacha_records =
        if std::path::Path::new(&format!("{}/{}.csv", save_floder, file_name)).exists() {
            // 读取抽卡记录
            let mut reader =
                csv::Reader::from_path(format!("{}/{}.csv", save_floder, file_name)).unwrap();
            let mut old_gacha_records = vec![];
            for result in reader.deserialize() {
                let record: GachaRecord = result.unwrap();
                old_gacha_records.push(record);
            }
            // 合并抽卡记录
            merge_gacha_records(&gacha_records, &old_gacha_records)
        } else {
            gacha_records
        };
    // 保存抽卡记录
    let mut writer = csv::Writer::from_path(format!("{}/{}.csv", save_floder, file_name)).unwrap();
    for record in &gacha_records {
        writer.serialize(record).unwrap();
    }
    writer.flush().unwrap();
    log::info!("Save gacha records to {}/{}.csv", save_floder, file_name);
    // 再保存一份带时间戳的抽卡记录
    let timestamp = chrono::Local::now().timestamp();
    let mut writer =
        csv::Writer::from_path(format!("{}/{}_{}.csv", save_floder, file_name, timestamp)).unwrap();
    for record in gacha_records {
        writer.serialize(record).unwrap();
    }
    writer.flush().unwrap();
    log::info!(
        "Save gacha records to {}/{}_{}.csv",
        save_floder,
        file_name,
        timestamp
    );
    // 按任意键退出
    println!("按任意键退出");
    loop {
        match crossterm::event::read() {
            Ok(crossterm::event::Event::Key(_)) => break,
            Ok(_) => {}
            Err(e) => {
                log::error!("{:?}", e);
            }
        }
    }
}

fn get_game_window_info() -> Result<(bool, isize)> {
    let china_window_title = "尘白禁区";
    let global_window_title = "Snowbreak: Containment Zone";
    let window_class = "UnrealWindow";

    if let Ok(hwnd) = get_hwnd_ref_cache(window_class, global_window_title) {
        return Ok((true, hwnd));
    }
    if let Ok(hwnd) = get_hwnd_ref_cache(window_class, china_window_title) {
        return Ok((false, hwnd));
    }
    Err(anyhow!("未找到游戏窗口"))
}
