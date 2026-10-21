mod commands;
mod dictation;
mod inject;
mod meeting;
mod overlay;
mod sched;
mod shortcuts;
mod state;
mod tray;

use std::sync::{mpsc, Arc};

use barnaby_core::store::Store;
use tauri::{AppHandle, Emitter, Manager, WindowEvent};

use state::App;

pub fn show_main(handle: &AppHandle, route: Option<&str>) {
    if let Some(w) = handle.get_webview_window("main") {
        let _ = w.show();
        let _ = w.unminimize();
        let _ = w.set_focus();
    }
    if let Some(route) = route {
        let _ = handle.emit("navigate", route);
    }
}

/// `barnaby --toggle` / `--cancel` sent to the running instance (spec 10.2).
fn handle_cli(handle: &AppHandle, args: &[String]) {
    let app = handle.state::<Arc<App>>().inner().clone();
    if args.iter().any(|a| a == "--toggle") {
        dictation::toggle(&app);
    } else if args.iter().any(|a| a == "--cancel") {
        dictation::cancel(&app);
    } else {
        show_main(handle, None);
    }
}

pub fn run() {
    tauri::Builder::default()
        // Must be the first plugin so a second launch is forwarded before anything else starts.
        .plugin(tauri_plugin_single_instance::init(|handle, args, _cwd| handle_cli(handle, &args)))
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .build(),
        )
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|tauri_app| {
            let handle = tauri_app.handle().clone();
            let data_dir = handle.path().app_data_dir()?;
            std::fs::create_dir_all(data_dir.join("models"))?;
            let store = Store::open(&data_dir.join("barnaby.db"))?;
            let (jobs, rx) = mpsc::channel();
            let app = Arc::new(App::new(handle.clone(), store, data_dir, jobs));
            handle.manage(app.clone());

            tray::create(&handle)?;
            overlay::create(&handle)?;
            app.preload_dictation_model();
            if let Err(e) = shortcuts::register(&handle, &app.settings()) {
                log::warn!("shortcuts: {e}");
            }
            if let Err(e) = meeting::recover(&app) {
                log::warn!("recover: {e:#}");
            }
            let worker_app = app.clone();
            std::thread::spawn(move || meeting::worker(worker_app, rx));
            tray::refresh(&app);
            Ok(())
        })
        .on_window_event(|window, event| {
            // Closing hides to the tray: shortcuts must keep working.
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::set_webhook_secret,
            commands::has_webhook_secret,
            commands::set_llm_api_key,
            commands::has_llm_api_key,
            commands::list_meetings,
            commands::get_meeting,
            commands::start_meeting,
            commands::stop_meeting,
            commands::current_recording,
            commands::add_note,
            commands::resolve_review_item,
            commands::rename_speaker,
            commands::update_segment_text,
            commands::validate_review,
            commands::retry_meeting,
            commands::delete_meeting,
            commands::list_dictations,
            commands::delete_dictation,
            commands::clear_dictations,
            commands::list_models,
            commands::download_model,
            commands::delete_model,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Barnaby");
}
