//! Menu-bar indicator (spec 7.1): lit while dictating or recording, tooltip carries the review count.

use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager,
};

use crate::state::{lock, App};
use barnaby_core::MeetingState;

const TRAY_ID: &str = "main";
const ICON_IDLE: &[u8] = include_bytes!("../icons/tray.png");
const ICON_ACTIVE: &[u8] = include_bytes!("../icons/tray-active.png");

pub fn create(handle: &AppHandle) -> tauri::Result<()> {
    let open = MenuItem::with_id(handle, "open", "Barnaby", true, None::<&str>)?;
    let meeting = MenuItem::with_id(handle, "meeting", "Réunion : démarrer / arrêter", true, None::<&str>)?;
    let quit = MenuItem::with_id(handle, "quit", "Quitter", true, None::<&str>)?;
    let menu = Menu::with_items(handle, &[&open, &meeting, &quit])?;
    TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(ICON_IDLE)?)
        .tooltip("Barnaby")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|handle, event| match event.id().as_ref() {
            "open" => crate::show_main(handle, None),
            "meeting" => {
                let app = handle.state::<std::sync::Arc<App>>().inner().clone();
                if let Err(e) = crate::meeting::toggle(&app) {
                    log::warn!("meeting toggle: {e:#}");
                }
            }
            "quit" => handle.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                crate::show_main(tray.app_handle(), None);
            }
        })
        .build(handle)?;
    Ok(())
}

pub fn refresh(app: &App) {
    let Some(tray) = app.handle.tray_by_id(TRAY_ID) else { return };
    let active = lock(&app.recording).is_some() || lock(&app.dictation).is_some();
    let icon = if active { ICON_ACTIVE } else { ICON_IDLE };
    if let Ok(img) = Image::from_bytes(icon) {
        let _ = tray.set_icon(Some(img));
    }
    let pending = app.db().list_meetings_in(&[MeetingState::NeedsReview]).map(|m| m.len()).unwrap_or(0);
    let tooltip = if pending > 0 { format!("Barnaby · {pending} à relire") } else { "Barnaby".into() };
    let _ = tray.set_tooltip(Some(tooltip));
}
