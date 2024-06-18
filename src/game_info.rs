use anyhow::anyhow;
use window_inspector::find::get_hwnd_ref_cache;

pub fn get_game_window_info() -> anyhow::Result<(isize, String)> {
    let window_class = "UnrealWindow";
    let possible_window_titles = ["尘白禁区", "Snowbreak: Containment Zone"];

    for title in possible_window_titles.iter() {
        if let Ok(hwnd) = get_hwnd_ref_cache(window_class, title) {
            return Ok((hwnd, title.to_string()));
        }
    }

    Err(anyhow!("Failed to get game window info"))
}
