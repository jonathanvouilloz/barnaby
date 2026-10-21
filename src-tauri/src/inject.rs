//! Text injection at the cursor (spec 7.2): clipboard, simulated paste, then restore.

use std::{process::Command, thread, time::Duration};

use anyhow::{Context, Result};

pub struct Delays {
    pub before_paste: Duration,
    pub before_restore: Duration,
}

pub fn paste_text(text: &str, delays: &Delays) -> Result<()> {
    let mut clipboard = arboard::Clipboard::new().context("clipboard_unavailable")?;
    let previous = clipboard.get_text().ok();
    clipboard.set_text(text).context("clipboard_write")?;
    thread::sleep(delays.before_paste);
    let pasted = simulate_paste();
    thread::sleep(delays.before_restore);
    // Restore even when the paste failed, so the user never loses their clipboard.
    if let Some(prev) = previous {
        let _ = clipboard.set_text(prev);
    }
    pasted
}

#[cfg(target_os = "linux")]
fn simulate_paste() -> Result<()> {
    // Native tools first: enigo cannot synthesize keys on most Wayland compositors.
    let wayland = std::env::var_os("WAYLAND_DISPLAY").is_some();
    let attempts: &[(&str, &[&str])] = if wayland {
        &[("wtype", &["-M", "ctrl", "v", "-m", "ctrl"]), ("dotool", &[]), ("xdotool", &["key", "--clearmodifiers", "ctrl+v"])]
    } else {
        &[("xdotool", &["key", "--clearmodifiers", "ctrl+v"])]
    };
    for (bin, args) in attempts {
        if run_tool(bin, args) {
            return Ok(());
        }
    }
    enigo_paste()
}

#[cfg(target_os = "linux")]
fn run_tool(bin: &str, args: &[&str]) -> bool {
    use std::io::Write;
    if bin == "dotool" {
        // dotool reads commands on stdin.
        let Ok(mut child) = Command::new(bin).stdin(std::process::Stdio::piped()).spawn() else { return false };
        let wrote = child.stdin.take().is_some_and(|mut s| s.write_all(b"key ctrl+v\n").is_ok());
        return wrote && child.wait().is_ok_and(|s| s.success());
    }
    Command::new(bin).args(args).status().is_ok_and(|s| s.success())
}

#[cfg(not(target_os = "linux"))]
fn simulate_paste() -> Result<()> {
    enigo_paste()
}

fn enigo_paste() -> Result<()> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    let mut enigo = Enigo::new(&Settings::default()).context("input_unavailable")?;
    let modifier = if cfg!(target_os = "macos") { Key::Meta } else { Key::Control };
    enigo.key(modifier, Direction::Press).context("paste_failed")?;
    let click = enigo.key(Key::Unicode('v'), Direction::Click);
    enigo.key(modifier, Direction::Release).context("paste_failed")?;
    click.context("paste_failed")
}

/// Process name of the focused application, best effort, for `Dictation::injected_into`.
/// Not the window title: titles carry content (mail subjects, document names).
pub fn active_app() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let out = Command::new("xdotool").args(["getactivewindow", "getwindowpid"]).output().ok()?;
        let pid = String::from_utf8(out.stdout).ok()?.trim().to_string();
        let name = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?.trim().to_string();
        (!name.is_empty()).then_some(name)
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}
