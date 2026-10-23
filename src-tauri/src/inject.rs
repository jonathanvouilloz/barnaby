//! Text injection at the cursor (spec 7.2): clipboard, then simulated paste. The text stays
//! on the clipboard, so it can still be pasted by hand when the paste lands nowhere.

use std::{sync::Mutex, thread, time::Duration};

use anyhow::{Context, Result};

/// Two dictations finishing close together must not interleave their set/paste.
static PASTE: Mutex<()> = Mutex::new(());

/// Puts `text` on the clipboard, then pastes it after `before_paste` when `paste` is set.
pub fn paste_text(text: &str, before_paste: Duration, paste: bool) -> Result<()> {
    let _one_at_a_time = PASTE.lock().unwrap_or_else(|e| e.into_inner());
    let mut clipboard = arboard::Clipboard::new().context("clipboard_unavailable")?;
    set_private(&mut clipboard, text).context("clipboard_write")?;
    // Linux (X11, Wayland): the text lives as long as its owner; keep it for a manual paste.
    // arboard shares one clipboard, so this keeps a single reference alive, not one per dictation.
    #[cfg(target_os = "linux")]
    std::mem::forget(clipboard);
    if !paste {
        return Ok(());
    }
    thread::sleep(before_paste);
    simulate_paste()
}

/// Keeps dictated text out of clipboard managers and Windows cloud sync (local-first).
fn set_private(clipboard: &mut arboard::Clipboard, text: &str) -> Result<(), arboard::Error> {
    #[cfg(target_os = "windows")]
    use arboard::SetExtWindows;
    #[cfg(target_os = "linux")]
    use arboard::SetExtLinux;
    #[cfg(target_os = "macos")]
    use arboard::SetExtApple;
    let set = clipboard.set().exclude_from_history();
    #[cfg(target_os = "windows")]
    let set = set.exclude_from_cloud();
    set.text(text)
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
    use std::process::Command;
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
    // macOS: the V key by position (kVK_ANSI_V). `Unicode('v')` looks the layout up through
    // text-input APIs that abort when called off the main thread, as here.
    let v = if cfg!(target_os = "macos") { Key::Other(9) } else { Key::Unicode('v') };
    let click = enigo.key(v, Direction::Click);
    enigo.key(modifier, Direction::Release).context("paste_failed")?;
    click.context("paste_failed")
}

/// `Some(true)` when no window can take the paste (desktop, taskbar, nothing focused);
/// `None` when this platform cannot tell.
pub fn no_target() -> Option<bool> {
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow};
        // SAFETY: plain Win32 calls; the buffer outlives the call and its length is passed.
        let class = unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return Some(true);
            }
            let mut buf = [0u16; 64];
            let n = GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
            String::from_utf16_lossy(&buf[..n.max(0) as usize])
        };
        Some(matches!(class.as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd"))
    }
    #[cfg(target_os = "linux")]
    {
        // Without xdotool, or on Wayland (XWayland sees no native window), the answer is unknown.
        if std::env::var_os("WAYLAND_DISPLAY").is_some() {
            return None;
        }
        let out = std::process::Command::new("xdotool").arg("getactivewindow").output().ok()?;
        out.status.success().then_some(false).or(Some(true))
    }
    #[cfg(target_os = "macos")]
    {
        None
    }
}

/// `Some(true)` when the focus is on a field that takes typed text, so the paste landed;
/// `None` when this platform or the focused app cannot tell.
pub fn text_field() -> Option<bool> {
    #[cfg(target_os = "windows")]
    {
        windows_field()
    }
    #[cfg(target_os = "linux")]
    {
        // shortcut: any focused X11 window counts as a field (AT-SPI would tell, if ever needed).
        no_target().map(|none| !none)
    }
    #[cfg(target_os = "macos")]
    {
        mac_field()
    }
}

/// UI Automation (what screen readers use), then the Win32 caret for apps that skip it.
#[cfg(target_os = "windows")]
fn windows_field() -> Option<bool> {
    use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED};
    use windows::Win32::UI::Accessibility::{
        CUIAutomation8, IUIAutomation, IUIAutomation2, IUIAutomationValuePattern, UIA_EditControlTypeId, UIA_ValuePatternId,
    };
    use windows::core::Interface;
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetGUIThreadInfo, GUITHREADINFO};

    // SAFETY: plain Win32 call on a zeroed struct whose size is set, as the API requires.
    let caret = unsafe {
        let mut info: GUITHREADINFO = std::mem::zeroed();
        info.cbSize = size_of::<GUITHREADINFO>() as u32;
        GetGUIThreadInfo(0, &mut info) != 0 && !info.hwndCaret.is_null()
    };
    if caret {
        return Some(true);
    }
    // SAFETY: COM on this dictation thread (MTA; an already initialised thread is fine too).
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let uia: IUIAutomation = CoCreateInstance(&CUIAutomation8, None, CLSCTX_INPROC_SERVER).ok()?;
        // A hung app must not hold the paste: give up after half a second.
        if let Ok(uia2) = uia.cast::<IUIAutomation2>() {
            let _ = uia2.SetTransactionTimeout(500);
        }
        let el = uia.GetFocusedElement().ok()?;
        let value = el.GetCurrentPatternAs::<IUIAutomationValuePattern>(UIA_ValuePatternId);
        if let Some(read_only) = value.ok().and_then(|v| v.CurrentIsReadOnly().ok()) {
            return Some(!read_only.as_bool());
        }
        Some(el.CurrentControlType().ok()? == UIA_EditControlTypeId)
    }
}

/// Accessibility API: the role of the focused element, or whether its caret can move.
#[cfg(target_os = "macos")]
fn mac_field() -> Option<bool> {
    use core_foundation::base::{CFRelease, CFTypeRef, TCFType};
    use core_foundation::string::{CFString, CFStringRef};
    use std::ffi::c_void;

    type Element = *const c_void;
    #[link(name = "ApplicationServices", kind = "framework")]
    unsafe extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXUIElementCreateSystemWide() -> Element;
        fn AXUIElementCopyAttributeValue(element: Element, attribute: CFStringRef, value: *mut CFTypeRef) -> i32;
        fn AXUIElementIsAttributeSettable(element: Element, attribute: CFStringRef, settable: *mut u8) -> i32;
    }
    const NO_VALUE: i32 = -25212; // kAXErrorNoValue: nothing has the focus

    // SAFETY: every Create/Copy result is released once; attributes are live CFStrings.
    unsafe {
        if !AXIsProcessTrusted() {
            return None;
        }
        let system = AXUIElementCreateSystemWide();
        let mut focused: CFTypeRef = std::ptr::null();
        let err = AXUIElementCopyAttributeValue(system, CFString::new("AXFocusedUIElement").as_concrete_TypeRef(), &mut focused);
        CFRelease(system);
        if err == NO_VALUE {
            return Some(false);
        }
        if err != 0 || focused.is_null() {
            return None;
        }
        let mut role: CFTypeRef = std::ptr::null();
        let typed = AXUIElementCopyAttributeValue(focused, CFString::new("AXRole").as_concrete_TypeRef(), &mut role) == 0 && !role.is_null() && {
            let role = CFString::wrap_under_create_rule(role as CFStringRef).to_string();
            matches!(role.as_str(), "AXTextField" | "AXTextArea" | "AXComboBox")
        };
        // Web editors (contenteditable) carry another role but a settable text selection.
        let mut settable = 0u8;
        let caret = AXUIElementIsAttributeSettable(focused, CFString::new("AXSelectedTextRange").as_concrete_TypeRef(), &mut settable) == 0 && settable != 0;
        CFRelease(focused);
        // shortcut: Electron apps hide their tree until asked (AXManualAccessibility); they read as "no field".
        Some(typed || caret)
    }
}

/// Process name of the focused application, best effort, for `Dictation::injected_into`.
/// Not the window title: titles carry content (mail subjects, document names).
pub fn active_app() -> Option<String> {
    #[cfg(target_os = "linux")]
    {
        let out = std::process::Command::new("xdotool").args(["getactivewindow", "getwindowpid"]).output().ok()?;
        let pid = String::from_utf8(out.stdout).ok()?.trim().to_string();
        let name = std::fs::read_to_string(format!("/proc/{pid}/comm")).ok()?.trim().to_string();
        (!name.is_empty()).then_some(name)
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}
