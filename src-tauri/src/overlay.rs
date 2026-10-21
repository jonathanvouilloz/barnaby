//! Recording indicator: a small always-on-top window that must never take the focus,
//! or the dictation would be pasted into it.

use tauri::{AppHandle, Manager, PhysicalPosition, WebviewUrl, WebviewWindowBuilder};

const LABEL: &str = "overlay";
const WIDTH: f64 = 200.0;
const HEIGHT: f64 = 56.0;
/// Gap above the taskbar, in logical pixels.
const MARGIN: f64 = 32.0;

pub fn create(handle: &AppHandle) -> tauri::Result<()> {
    WebviewWindowBuilder::new(handle, LABEL, WebviewUrl::App("index.html#overlay".into()))
        .title("Barnaby")
        .inner_size(WIDTH, HEIGHT)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .resizable(false)
        .shadow(false)
        .focused(false)
        .focusable(false)
        // Windows: shown once here, never re-shown (see `show`).
        .visible(cfg!(target_os = "windows"))
        .build()?;
    // Not `.position(OFF_SCREEN)`: tao ignores a start position outside every monitor.
    hide(handle);
    Ok(())
}

/// Windows can activate a window on `ShowWindow` even with WS_EX_NOACTIVATE, which would
/// send the paste into the overlay; moving an already-visible window never activates it.
const OFF_SCREEN: (f64, f64) = (-32000.0, -32000.0);

/// Bottom centre of the primary monitor's work area, so it clears the taskbar. Never focuses.
pub fn show(handle: &AppHandle) {
    let Some(w) = handle.get_webview_window(LABEL) else { return };
    if let Ok(Some(m)) = w.primary_monitor() {
        let (area, scale) = (m.work_area(), m.scale_factor());
        let (width, height, margin) = ((WIDTH * scale) as i32, (HEIGHT * scale) as i32, (MARGIN * scale) as i32);
        let x = area.position.x + (area.size.width as i32 - width) / 2;
        let y = area.position.y + area.size.height as i32 - height - margin;
        let _ = w.set_position(PhysicalPosition::new(x, y));
    }
    if !cfg!(target_os = "windows") {
        let _ = w.show();
    }
}

pub fn hide(handle: &AppHandle) {
    let Some(w) = handle.get_webview_window(LABEL) else { return };
    if cfg!(target_os = "windows") {
        let _ = w.set_position(tauri::LogicalPosition::new(OFF_SCREEN.0, OFF_SCREEN.1));
    } else {
        let _ = w.hide();
    }
}
