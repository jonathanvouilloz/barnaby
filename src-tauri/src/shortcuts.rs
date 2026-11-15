//! Global shortcuts (spec 10.2), re-registered whenever settings change.

use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use barnaby_core::{DictationMode, Settings};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::{
    dictation, meeting,
    state::{lock, App},
};

/// Hold mode: a release sooner than this after the press is a tap.
const TAP: Duration = Duration::from_millis(300);
/// When the press that started the current hold-mode dictation happened.
static HOLD_START: Mutex<Option<Instant>> = Mutex::new(None);

/// Registers all shortcuts or none: on a conflict, the error names the faulty accelerator.
pub fn register(handle: &AppHandle, settings: &Settings) -> Result<(), String> {
    let gs = handle.global_shortcut();
    gs.unregister_all().map_err(|e| e.to_string())?;
    let s = &settings.shortcuts;
    let hold = settings.dictation.mode == DictationMode::Hold;
    let bindings: [(&str, Action); 4] = [
        (&s.dictation, Action::Dictation { hold }),
        (&s.meeting_toggle, Action::Meeting),
        (&s.marker, Action::Marker),
        (&s.review_queue, Action::ReviewQueue),
    ];
    #[cfg(target_os = "linux")]
    if crate::portal::wanted() {
        register_portal(handle, &bindings);
        return Ok(());
    }
    for (accel, action) in bindings {
        let result = gs.on_shortcut(accel, move |handle, _, event| {
            let app = handle.state::<Arc<App>>().inner().clone();
            action.run(&app, event.state);
        });
        if let Err(e) = result {
            let _ = gs.unregister_all();
            return Err(format!("shortcut_conflict:{accel}: {e}"));
        }
    }
    Ok(())
}

/// Wayland: the compositor owns the keys. The id carries the accelerator, so a changed one
/// is a new shortcut and gets its preferred trigger (the portal keeps the user's choice per id).
#[cfg(target_os = "linux")]
fn register_portal(handle: &AppHandle, bindings: &[(&str, Action)]) {
    static LISTENING: std::sync::Once = std::sync::Once::new();
    let actions: Vec<(String, Action)> = bindings.iter().map(|(accel, action)| (format!("{}:{accel}", action.name()), *action)).collect();
    let shortcuts = actions.iter().map(|(id, a)| (id.clone(), crate::portal::trigger(id.split_once(':').map_or("", |p| p.1)), a.description())).collect();
    *lock(&PORTAL_ACTIONS) = actions;
    let handle = handle.clone();
    LISTENING.call_once(|| {
        let result = crate::portal::listen(move |id, pressed| {
            let action = lock(&PORTAL_ACTIONS).iter().find(|(i, _)| i == id).map(|(_, a)| *a);
            if let Some(action) = action {
                let app = handle.state::<Arc<App>>().inner().clone();
                action.run(&app, if pressed { ShortcutState::Pressed } else { ShortcutState::Released });
            }
        });
        if let Err(e) = result {
            log::warn!("shortcut portal listen: {e:#}");
        }
    });
    // Off the caller's thread: the compositor may show a dialog and wait for the user.
    std::thread::spawn(move || {
        if let Err(e) = crate::portal::bind(shortcuts) {
            log::warn!("shortcut portal bind: {e:#}");
        }
    });
}

/// Ids bound through the portal and their actions.
#[cfg(target_os = "linux")]
static PORTAL_ACTIONS: Mutex<Vec<(String, Action)>> = Mutex::new(Vec::new());

/// Lets the settings capture a key combination that is currently bound (it would never reach the webview).
pub fn suspend(handle: &AppHandle) {
    let _ = handle.global_shortcut().unregister_all();
    #[cfg(target_os = "linux")]
    crate::portal::unbind();
}

/// Whether another application already holds `accel` system-wide. Call while suspended,
/// or Barnaby's own binding would count. Wayland and macOS may not report every conflict.
pub fn taken_elsewhere(handle: &AppHandle, accel: &str) -> bool {
    let gs = handle.global_shortcut();
    let taken = gs.register(accel).is_err();
    if !taken {
        let _ = gs.unregister(accel);
    }
    taken
}

#[derive(Clone, Copy)]
enum Action {
    Dictation { hold: bool },
    Meeting,
    Marker,
    ReviewQueue,
}

impl Action {
    #[cfg(target_os = "linux")]
    fn name(self) -> &'static str {
        match self {
            Action::Dictation { .. } => "dictation",
            Action::Meeting => "meeting",
            Action::Marker => "marker",
            Action::ReviewQueue => "review",
        }
    }

    /// Shown by the compositor (GNOME Settings › Apps), in the interface's default language.
    #[cfg(target_os = "linux")]
    fn description(self) -> &'static str {
        match self {
            Action::Dictation { .. } => "Dictée",
            Action::Meeting => "Démarrer ou arrêter une réunion",
            Action::Marker => "Marqueur de réunion",
            Action::ReviewQueue => "File de relecture",
        }
    }

    fn run(self, app: &Arc<App>, state: ShortcutState) {
        let pressed = state == ShortcutState::Pressed;
        match self {
            // Tap or hold: holding dictates until release; a tap keeps recording until the next press.
            Action::Dictation { hold: true } if pressed => {
                if dictation::is_recording(app) {
                    dictation::stop(app);
                } else {
                    dictation::start(app);
                    *lock(&HOLD_START) = Some(Instant::now());
                }
            }
            Action::Dictation { hold: true } => {
                if lock(&HOLD_START).take().is_some_and(|t| t.elapsed() >= TAP) {
                    dictation::stop(app);
                }
            }
            Action::Dictation { hold: false } if pressed => dictation::toggle(app),
            Action::Meeting if pressed => log_err(meeting::toggle(app)),
            Action::Marker if pressed => log_err(meeting::add_note(app, "", "marker")),
            Action::ReviewQueue if pressed => crate::show_main(&app.handle, Some("review")),
            _ => {}
        }
    }
}

fn log_err(r: anyhow::Result<()>) {
    if let Err(e) = r {
        log::warn!("shortcut action: {e:#}");
    }
}
