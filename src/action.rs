use enigo::Button::Left;
use enigo::Coordinate::Abs;
use enigo::Direction::Click;
use enigo::{Enigo, Mouse, Settings};
use window_inspector::position_size::get_client_xywh;

static PAGE_BUTTON_X: u32 = 1664;
static PREVIOUS_PAGE_BUTTON_Y: u32 = 435;
static NEXT_PAGE_BUTTON_Y: u32 = 616;

pub fn next_page(hwnd: isize) {
    let (client_x, client_y, client_width, client_height) = get_client_xywh(hwnd).unwrap();
    let screen_x = client_x + (client_width as f32 * PAGE_BUTTON_X as f32 / 1920.0).round() as i32;
    let screen_y =
        client_y + (client_height as f32 * NEXT_PAGE_BUTTON_Y as f32 / 1080.0).round() as i32;
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    enigo.move_mouse(screen_x, screen_y, Abs).unwrap();
    enigo.button(Left, Click).unwrap();
}

pub fn previous_page(hwnd: isize) {
    let (client_x, client_y, client_width, client_height) = get_client_xywh(hwnd).unwrap();
    let screen_x = client_x + (client_width as f32 * PAGE_BUTTON_X as f32 / 1920.0).round() as i32;
    let screen_y =
        client_y + (client_height as f32 * PREVIOUS_PAGE_BUTTON_Y as f32 / 1080.0).round() as i32;
    let mut enigo = Enigo::new(&Settings::default()).unwrap();
    enigo.move_mouse(screen_x, screen_y, Abs).unwrap();
    enigo.button(Left, Click).unwrap();
}
