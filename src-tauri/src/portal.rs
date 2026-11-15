//! Wayland global shortcuts through xdg-desktop-portal. The X11 grab of
//! tauri-plugin-global-shortcut only sees keys while an XWayland window has focus: never in
//! Barnaby's own window, nor in native Wayland apps.

use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicU32, Ordering},
    Mutex, OnceLock,
};

use anyhow::{bail, Result};
use zbus::blocking::{Connection, MessageIterator};
use zbus::zvariant::{ObjectPath, OwnedValue, Value};
use zbus::MatchRule;

const DEST: &str = "org.freedesktop.portal.Desktop";
const PATH: &str = "/org/freedesktop/portal/desktop";
const IFACE: &str = "org.freedesktop.portal.GlobalShortcuts";
/// Name of the installed `.desktop` file: the portal refuses an app without one.
const APP_ID: &str = "Barnaby";

struct Portal {
    conn: Connection,
    /// The bound session, replaced on every `bind`.
    session: Mutex<Option<String>>,
}

static PORTAL: OnceLock<Option<Portal>> = OnceLock::new();
static TOKEN: AtomicU32 = AtomicU32::new(0);

fn portal() -> Option<&'static Portal> {
    PORTAL.get_or_init(|| connect().map_err(|e| log::info!("shortcut portal unavailable: {e:#}")).ok()).as_ref()
}

fn connect() -> Result<Portal> {
    let conn = Connection::session()?;
    // Must come before any other portal call; fails outside an installed package, where the
    // launching scope may still give the app id.
    if let Err(e) = conn.call_method(Some(DEST), PATH, Some("org.freedesktop.host.portal.Registry"), "Register", &(APP_ID, HashMap::<&str, Value>::new())) {
        log::info!("portal register: {e}");
    }
    Ok(Portal { conn, session: Mutex::new(None) })
}

/// Whether shortcuts must go through the portal: a Wayland session with the portal running.
pub fn wanted() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some() && portal().is_some()
}

/// Starts the thread that turns portal signals into `on(id, pressed)`, once.
pub fn listen(on: impl Fn(&str, bool) + Send + 'static) -> Result<()> {
    let Some(p) = portal() else { bail!("no portal") };
    let rule = MatchRule::builder().msg_type(zbus::message::Type::Signal).interface(IFACE)?.path(PATH)?.build();
    let signals = MessageIterator::for_match_rule(rule, &p.conn, None)?;
    std::thread::spawn(move || {
        for msg in signals.flatten() {
            let pressed = match msg.header().member().map(|m| m.as_str()) {
                Some("Activated") => true,
                Some("Deactivated") => false,
                _ => continue,
            };
            let body = msg.body();
            let Ok((session, id, _, _)) = body.deserialize::<(ObjectPath, &str, u64, HashMap<&str, OwnedValue>)>() else { continue };
            if p.session.lock().unwrap_or_else(|e| e.into_inner()).as_deref() == Some(session.as_str()) {
                on(id, pressed);
            }
        }
    });
    Ok(())
}

/// Replaces the bound shortcuts with `(id, trigger in xdg format, description)`. The compositor may ask the
/// user to confirm or pick another trigger (taken, invalid): the call then waits for them.
pub fn bind(shortcuts: Vec<(String, String, &str)>) -> Result<()> {
    let Some(p) = portal() else { bail!("no portal") };
    let mut slot = p.session.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(old) = slot.take() {
        close(p, &old);
    }
    let token = format!("barnaby{}", TOKEN.fetch_add(1, Ordering::Relaxed));
    let session = format!("{PATH}/session/{}/{token}", sender(p));
    let opts = HashMap::from([("handle_token", Value::from(token.as_str())), ("session_handle_token", Value::from(token.as_str()))]);
    request(p, &token, "CreateSession", &(opts,))?;
    *slot = Some(session.clone());
    drop(slot);
    let list: Vec<(&str, HashMap<&str, Value>)> = shortcuts
        .iter()
        .map(|(id, trigger, description)| (id.as_str(), HashMap::from([("description", Value::from(*description)), ("preferred_trigger", Value::from(trigger.as_str()))])))
        .collect();
    let token = format!("barnaby{}", TOKEN.fetch_add(1, Ordering::Relaxed));
    let opts = HashMap::from([("handle_token", Value::from(token.as_str()))]);
    request(p, &token, "BindShortcuts", &(ObjectPath::try_from(session.as_str())?, list, "", opts))
}

pub fn unbind() {
    if let Some(p) = portal() {
        if let Some(old) = p.session.lock().unwrap_or_else(|e| e.into_inner()).take() {
            close(p, &old);
        }
    }
}

fn close(p: &Portal, session: &str) {
    let _ = p.conn.call_method(Some(DEST), session, Some("org.freedesktop.portal.Session"), "Close", &());
}

/// Unique bus name as the portal writes it in request and session paths: `:1.42` → `1_42`.
fn sender(p: &Portal) -> String {
    p.conn.unique_name().map(|n| n.trim_start_matches(':').replace('.', "_")).unwrap_or_default()
}

/// Calls `method` and waits for its `Request.Response`, subscribed before the call so it cannot be missed.
fn request<B: serde::Serialize + zbus::zvariant::DynamicType>(p: &Portal, token: &str, method: &str, body: &B) -> Result<()> {
    let path = format!("{PATH}/request/{}/{token}", sender(p));
    let rule = MatchRule::builder()
        .msg_type(zbus::message::Type::Signal)
        .interface("org.freedesktop.portal.Request")?
        .member("Response")?
        .path(path)?
        .build();
    let mut responses = MessageIterator::for_match_rule(rule, &p.conn, Some(1))?;
    p.conn.call_method(Some(DEST), PATH, Some(IFACE), method, body)?;
    let Some(msg) = responses.next() else { bail!("{method}: no response") };
    let (code, _) = msg?.body().deserialize::<(u32, HashMap<String, OwnedValue>)>()?;
    if code != 0 {
        bail!("{method}: response {code}");
    }
    Ok(())
}

/// Tauri accelerator (`Ctrl+Alt+Space`, `Super+Shift+R`) → xdg shortcut (`CTRL+ALT+space`).
/// Key names are xkb keysyms; an unknown one is passed as is and the compositor asks the user.
pub fn trigger(accel: &str) -> String {
    let parts: Vec<&str> = accel.split('+').collect();
    let (key, mods) = parts.split_last().unwrap_or((&"", &[]));
    let mut out: Vec<String> = mods
        .iter()
        .map(|m| match *m {
            "Ctrl" | "Control" | "CommandOrControl" | "CmdOrCtrl" => "CTRL",
            "Alt" | "Option" => "ALT",
            "Shift" => "SHIFT",
            "Super" | "Meta" | "Cmd" | "Command" => "LOGO",
            other => other,
        })
        .map(String::from)
        .collect();
    let named = match *key {
        "Space" => "space",
        "Enter" => "Return",
        "Escape" => "Escape",
        "Backspace" => "BackSpace",
        "Tab" => "Tab",
        "ArrowUp" => "Up",
        "ArrowDown" => "Down",
        "ArrowLeft" => "Left",
        "ArrowRight" => "Right",
        "PrintScreen" => "Print",
        "ScrollLock" => "Scroll_Lock",
        "PageUp" => "Page_Up",
        "PageDown" => "Page_Down",
        "Comma" => "comma",
        "Period" => "period",
        "Slash" => "slash",
        "Backslash" => "backslash",
        "Semicolon" => "semicolon",
        "Quote" => "apostrophe",
        "Minus" => "minus",
        "Equal" => "equal",
        "Backquote" => "grave",
        "BracketLeft" => "bracketleft",
        "BracketRight" => "bracketright",
        _ => "",
    };
    out.push(match () {
        _ if !named.is_empty() => named.to_string(),
        _ if key.len() == 1 => key.to_ascii_lowercase(),
        _ if key.starts_with("Numpad") => format!("KP_{}", &key[6..]),
        _ => key.to_string(),
    });
    out.join("+")
}

#[cfg(test)]
mod tests {
    use super::trigger;

    #[test]
    fn accelerators_become_xdg_triggers() {
        assert_eq!(trigger("Ctrl+Alt+Space"), "CTRL+ALT+space");
        assert_eq!(trigger("Super+Shift+R"), "LOGO+SHIFT+r");
        assert_eq!(trigger("Ctrl+Shift+1"), "CTRL+SHIFT+1");
        assert_eq!(trigger("F9"), "F9");
        assert_eq!(trigger("Alt+ArrowUp"), "ALT+Up");
        assert_eq!(trigger("Ctrl+Numpad1"), "CTRL+KP_1");
    }
}
