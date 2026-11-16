// Prevents an extra console window on Windows in release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // GNOME Wayland gives the focus to every window that shows, so the recording pill stole it;
    // under XWayland the pill stays unfocusable, on top and placed. The user's choice wins.
    #[cfg(target_os = "linux")]
    if std::env::var_os("GDK_BACKEND").is_none()
        && std::env::var("XDG_CURRENT_DESKTOP").is_ok_and(|d| d.contains("GNOME"))
        && std::env::var_os("WAYLAND_DISPLAY").is_some()
    {
        // SAFETY: first thing in main, before any thread exists.
        unsafe { std::env::set_var("GDK_BACKEND", "x11") };
    }
    barnaby_lib::run()
}
