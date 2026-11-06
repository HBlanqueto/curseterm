//! Desktop notifications through org.freedesktop.Notifications, and event sounds, as
//! `NSUserNotificationCenter` and `NSSound` give a Cocoa app.

use std::collections::HashMap;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::thread;

use zbus::blocking::{Connection, Proxy};
use zbus::zvariant::Value;

const SERVICE: &str = "org.freedesktop.Notifications";
const PATH: &str = "/org/freedesktop/Notifications";
const DEFAULT_ACTION: &str = "default";

type Clicked = Box<dyn FnOnce(Option<String>) + Send>;

static WAITING: Mutex<Vec<(u32, Clicked)>> = Mutex::new(Vec::new());
static TOKENS: Mutex<Vec<(u32, String)>> = Mutex::new(Vec::new());

pub struct Notice {
    pub app_name: &'static str,
    pub desktop_entry: &'static str,
    pub summary: String,
    pub body: String,
    pub action: String,
}

// `clicked` gets the xdg-activation token the server sends before ActionInvoked (spec 1.2).
pub fn post(notice: Notice, clicked: impl FnOnce(Option<String>) + Send + 'static) {
    thread::spawn(move || {
        let Some(proxy) = proxy() else {
            return;
        };
        let hints = HashMap::from([("desktop-entry", Value::from(notice.desktop_entry))]);
        let sent = proxy.call::<_, _, u32>(
            "Notify",
            &(
                notice.app_name,
                0u32,
                "",
                notice.summary.as_str(),
                notice.body.as_str(),
                vec![DEFAULT_ACTION, notice.action.as_str()],
                hints,
                -1i32,
            ),
        );
        match sent {
            Ok(id) => lock(&WAITING).push((id, Box::new(clicked))),
            Err(error) => tracing::warn!(%error, "could not post a notification"),
        }
    });
}

// `sound` is a name from the freedesktop sound naming specification.
pub fn play_event_sound(sound: &str) {
    match Command::new("canberra-gtk-play")
        .args(["--id", sound])
        .spawn()
    {
        Ok(mut child) => {
            thread::spawn(move || child.wait());
        }
        Err(error) => tracing::warn!(%error, "could not run canberra-gtk-play"),
    }
}

fn proxy() -> Option<&'static Proxy<'static>> {
    static PROXY: OnceLock<Option<Proxy<'static>>> = OnceLock::new();
    PROXY
        .get_or_init(|| {
            let connected = Connection::session()
                .and_then(|connection| Proxy::new(&connection, SERVICE, PATH, SERVICE));
            let proxy = match connected {
                Ok(proxy) => proxy,
                Err(error) => {
                    tracing::warn!(%error, "no notification service");
                    return None;
                }
            };
            match proxy.receive_all_signals() {
                Ok(signals) => {
                    thread::spawn(move || signals.for_each(|message| hear(&message)));
                }
                Err(error) => tracing::warn!(%error, "notification clicks will not be heard"),
            }
            Some(proxy)
        })
        .as_ref()
}

fn hear(message: &zbus::Message) {
    let header = message.header();
    let body = message.body();
    match header.member().map(|member| member.as_str()) {
        Some("ActivationToken") => {
            if let Ok((id, token)) = body.deserialize::<(u32, String)>() {
                lock(&TOKENS).push((id, token));
            }
        }
        Some("ActionInvoked") => {
            if let Ok((id, _)) = body.deserialize::<(u32, String)>()
                && let Some(clicked) = take(&WAITING, id)
            {
                clicked(take(&TOKENS, id));
            }
        }
        Some("NotificationClosed") => {
            if let Ok((id, _)) = body.deserialize::<(u32, u32)>() {
                take(&WAITING, id);
                take(&TOKENS, id);
            }
        }
        _ => {}
    }
}

fn take<T>(pending: &Mutex<Vec<(u32, T)>>, id: u32) -> Option<T> {
    let mut pending = lock(pending);
    let index = pending.iter().position(|(waiting, _)| *waiting == id)?;
    Some(pending.remove(index).1)
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}
