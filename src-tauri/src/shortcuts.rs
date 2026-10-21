//! Global shortcuts (spec 10.2), re-registered whenever settings change.

use std::sync::Arc;

use barnaby_core::{DictationMode, Settings};
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use crate::{dictation, meeting, state::App};

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

#[derive(Clone, Copy)]
enum Action {
    Dictation { hold: bool },
    Meeting,
    Marker,
    ReviewQueue,
}

impl Action {
    fn run(self, app: &Arc<App>, state: ShortcutState) {
        let pressed = state == ShortcutState::Pressed;
        match self {
            Action::Dictation { hold: true } if pressed => dictation::start(app),
            Action::Dictation { hold: true } => dictation::stop(app),
            Action::Dictation { hold: false } if pressed => dictation::toggle(app),
            Action::Meeting if pressed => log_err(meeting::toggle(app)),
            Action::Marker if pressed => log_err(meeting::add_marker(app)),
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
