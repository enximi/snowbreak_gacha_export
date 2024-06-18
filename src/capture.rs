use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Result};
use client_capture::ClientCapture;
use image::{DynamicImage, GenericImageView};
use lazy_static::lazy_static;

lazy_static! {
    static ref CLIENT_CAPTURE: Arc<Mutex<Option<ClientCapture>>> = Arc::new(Mutex::new(None));
}

fn is_capture_initialized() -> bool {
    CLIENT_CAPTURE.lock().unwrap().is_some()
}

pub fn init_capture(window_title: String) {
    if !is_capture_initialized() {
        let mut client_capture = ClientCapture::new(
            "UnrealWindow".to_string(),
            window_title,
            None,
            None,
            Some(false),
            None,
        );
        client_capture.start().unwrap();
        let start = Instant::now();
        while client_capture.get_img().is_err() {
            sleep(Duration::from_millis(500));
            if start.elapsed().as_secs_f32() > 5.0 {
                panic!("Capture not started in 5 seconds");
            }
        }
        CLIENT_CAPTURE.lock().unwrap().replace(client_capture);
    }
}

pub fn release_capture() {
    if is_capture_initialized() {
        let client_capture = CLIENT_CAPTURE.lock().unwrap().take().unwrap();
        client_capture.stop();
    }
}

pub fn capture_image() -> Result<DynamicImage> {
    let mut client_capture = CLIENT_CAPTURE.lock().unwrap().take().unwrap();
    let image_result = client_capture.get_img();
    CLIENT_CAPTURE.lock().unwrap().replace(client_capture);
    let image = image_result?;
    let image_size = image.dimensions();
    let ratio = num_rational::Ratio::new(image_size.0 as i64, image_size.1 as i64);
    if ratio != num_rational::Ratio::new(16, 9) {
        return Err(anyhow!("Invalid image ratio: {:?}", ratio));
    }
    let image = if image_size.0 == 1920 {
        image
    } else {
        image.resize_exact(1920, 1080, image::imageops::FilterType::Lanczos3)
    };
    Ok(image)
}
