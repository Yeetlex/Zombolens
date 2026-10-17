// ZomboLens: your Project Zomboid character's status, read from the network.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod net;
mod pz;
mod raknet;

use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, State, WebviewUrl, WebviewWindowBuilder, WindowEvent};

#[derive(Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
struct Settings {
    mode: String,
    show_values: bool,
    minimized: bool,
    x: Option<i32>,
    y: Option<i32>,
}

impl Default for Settings {
    fn default() -> Self {
        Self { mode: "basic".into(), show_values: true, minimized: false, x: None, y: None }
    }
}

struct Store {
    path: PathBuf,
    settings: Mutex<Settings>,
}

impl Store {
    fn save(&self) {
        if let Ok(t) = serde_json::to_string_pretty(&*self.settings.lock()) {
            let _ = std::fs::write(&self.path, t);
        }
    }
}

struct Shared {
    tracker: Mutex<pz::Tracker>,
    capture_error: Mutex<Option<String>>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct View {
    #[serde(flatten)]
    state: pz::State,
    capture_error: Option<String>,
    replay: bool,
}

#[tauri::command]
fn fit_window(window: tauri::WebviewWindow, width: f64, height: f64) {
    let _ = window.set_size(tauri::LogicalSize::new(width, height));
    // Hyprland ignores a floating window resizing itself: ask the compositor too.
    let (w, h, pid) = (width.round() as i64, height.round() as i64, std::process::id());
    let run = |arg: String| {
        std::process::Command::new("hyprctl").args(["dispatch", &arg]).output().ok()
            .is_some_and(|o| String::from_utf8_lossy(&o.stdout).trim() == "ok")
    };
    // Where the user left it: Hyprland resizes from the centre, so put the top-left corner back.
    let at = std::process::Command::new("hyprctl").args(["-j", "clients"]).output().ok()
        .and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok())
        .and_then(|v| v.as_array()?.iter().find(|c| c["pid"].as_u64() == Some(pid as u64))
            .map(|c| (c["at"][0].as_i64().unwrap_or(0), c["at"][1].as_i64().unwrap_or(0))));
    if !run(format!("resizewindowpixel exact {w} {h},pid:{pid}")) {
        let _ = run(format!("hl.dsp.window.resize({{ window = \"pid:{pid}\", x = {w}, y = {h} }})"));
    }
    if let Some((x, y)) = at {
        if !run(format!("movewindowpixel exact {x} {y},pid:{pid}")) {
            let _ = run(format!("hl.dsp.window.move({{ window = \"pid:{pid}\", x = {x}, y = {y} }})"));
        }
    }
}

#[tauri::command]
fn get_settings(store: State<Arc<Store>>) -> Settings {
    store.settings.lock().clone()
}

#[tauri::command]
fn save_settings(store: State<Arc<Store>>, settings: Settings) {
    let mut s = store.settings.lock();
    s.mode = settings.mode;
    s.show_values = settings.show_values;
    s.minimized = settings.minimized;
    drop(s);
    store.save();
}

fn now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

fn start_capture(shared: Arc<Shared>, source: Option<String>) {
    std::thread::spawn(move || {
        let mut reasm = raknet::Reassembler::default();
        let result = net::run(source, true, |d| {
            for m in reasm.feed(d.src, d.dst, d.time, &d.payload) {
                shared.tracker.lock().on_message(d.src, d.dst, d.time, &m);
            }
        });
        if let Err(e) = result {
            tracing::error!("capture: {e}");
            *shared.capture_error.lock() = Some(e);
        }
    });
}

/// Hyprland: a fullscreen window covers floating windows mapped before it.
/// Show the overlay only on the game's workspace, moved there, so it is
/// mapped again (above the game) whenever you look at it.
#[cfg(target_os = "linux")]
fn start_hyprland_follow(app: AppHandle) {
    if std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none() {
        return;
    }
    let hyprctl = |args: &[&str]| -> Option<String> {
        let out = std::process::Command::new("hyprctl").args(args).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
    };
    std::thread::spawn(move || {
        let mut shown = true;
        loop {
            std::thread::sleep(Duration::from_millis(500));
            let Some(clients) = hyprctl(&["-j", "clients"]).and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) else { continue };
            let list = clients.as_array().cloned().unwrap_or_default();
            let game = list.iter().find(|c| {
                let class = c["class"].as_str().unwrap_or("").to_lowercase();
                let title = c["title"].as_str().unwrap_or("").to_lowercase();
                class.contains("zomboid") || title == "project zomboid"
            });
            let Some(game_ws) = game.and_then(|g| g["workspace"]["name"].as_str()).map(String::from) else { continue };
            let active = hyprctl(&["-j", "activeworkspace"])
                .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
                .and_then(|v| v["name"].as_str().map(String::from));
            let here = active.as_deref() == Some(game_ws.as_str());
            let Some(w) = app.get_webview_window("main") else { continue };
            if here && !shown {
                let _ = w.show();
                shown = true;
                // Move it to the game's workspace if it opened elsewhere.
                if let Some(me) = list.iter().find(|c| c["title"].as_str() == Some("ZomboLens")) {
                    if me["workspace"]["name"].as_str() != Some(game_ws.as_str()) {
                        let addr = me["address"].as_str().unwrap_or("");
                        let classic = format!("{game_ws},address:{addr}");
                        let ok = hyprctl(&["dispatch", "movetoworkspacesilent", &classic]).is_some_and(|o| o.trim() == "ok");
                        if !ok {
                            let lua = format!("hl.dsp.window.move({{ window = \"address:{addr}\", workspace = \"{game_ws}\", follow = false }})");
                            let _ = hyprctl(&["dispatch", &lua]);
                        }
                    }
                }
            } else if !here && shown {
                let _ = w.hide();
                shown = false;
            }
        }
    });
}

fn start_emitter(app: AppHandle, shared: Arc<Shared>, replay: bool) {
    std::thread::spawn(move || {
        let mut replay_clock: Option<(f64, f64)> = None;
        loop {
            std::thread::sleep(Duration::from_millis(200));
            let mut state = shared.tracker.lock().state.clone();
            // In a replay the packets carry old times: measure age on their clock.
            let t = if replay {
                let (first_now, first_pkt) = *replay_clock.get_or_insert((now(), state.last_ms));
                if first_pkt == 0.0 { replay_clock = None; now() } else { first_pkt + (now() - first_now) }
            } else {
                now()
            };
            state.age_s = (state.last_ms > 0.0).then(|| (t - state.last_ms).max(0.0));
            let view = View { state, capture_error: shared.capture_error.lock().clone(), replay };
            let _ = app.emit("state", &view);
        }
    });
}

/// On Wayland, a window cannot keep itself above a fullscreen game; through
/// XWayland it can. Players can override with GDK_BACKEND. Hyprland is the
/// exception: there the overlay is handled natively (it follows the game's
/// workspace and resizes through hyprctl), and under XWayland it cannot be
/// dragged without holding the mod key.
#[cfg(target_os = "linux")]
fn prefer_xwayland() {
    if std::env::var_os("WAYLAND_DISPLAY").is_some()
        && std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_none()
        && std::env::var_os("DISPLAY").is_some()
        && std::env::var_os("GDK_BACKEND").is_none()
    {
        // SAFETY: first thing in main, no other thread yet.
        unsafe { std::env::set_var("GDK_BACKEND", "x11") };
    }
    if std::env::var_os("WEBKIT_DMABUF_RENDERER_FORCE_SHM").is_none() {
        unsafe { std::env::set_var("WEBKIT_DMABUF_RENDERER_FORCE_SHM", "1") };
    }
}

fn main() {
    #[cfg(target_os = "linux")]
    prefer_xwayland();
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()))
        .init();
    let args: Vec<String> = std::env::args().collect();
    let replay = args.iter().position(|a| a == "--replay").and_then(|i| args.get(i + 1).cloned());
    // --dump <pcap>: decode a capture as fast as possible and print the final state.
    if let Some(path) = args.iter().position(|a| a == "--dump").and_then(|i| args.get(i + 1).cloned()) {
        let mut reasm = raknet::Reassembler::default();
        let mut tracker = pz::Tracker::default();
        let mut count = std::collections::HashMap::<u16, u32>::new();
        net::run(Some(path), false, |d| {
            for m in reasm.feed(d.src, d.dst, d.time, &d.payload) {
                if m.len() > 2 && m[0] == 0x86 {
                    *count.entry(u16::from_be_bytes([m[1], m[2]])).or_default() += 1;
                }
                tracker.on_message(d.src, d.dst, d.time, &m);
            }
        })
        .expect("dump");
        println!("{}", serde_json::to_string_pretty(&tracker.state).unwrap());
        let mut c: Vec<_> = count.into_iter().collect();
        c.sort_by_key(|x| std::cmp::Reverse(x.1));
        eprintln!("mensajes por tipo (top 8): {:?}", &c[..c.len().min(8)]);
        return;
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
            }
        }))
        .invoke_handler(tauri::generate_handler![get_settings, save_settings, fit_window])
        .setup(move |app| {
            let dir = app.path().app_config_dir().unwrap_or_else(|_| ".".into());
            let _ = std::fs::create_dir_all(&dir);
            let path = dir.join("settings.json");
            let settings: Settings = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
            let store = Arc::new(Store { path, settings: Mutex::new(settings.clone()) });
            app.manage(store.clone());

            let shared = Arc::new(Shared { tracker: Mutex::new(pz::Tracker::default()), capture_error: Mutex::new(None) });
            start_capture(shared.clone(), replay.clone());
            start_emitter(app.handle().clone(), shared, replay.is_some());
            #[cfg(target_os = "linux")]
            if replay.is_none() {
                start_hyprland_follow(app.handle().clone());
            }

            let window = WebviewWindowBuilder::new(app, "main", WebviewUrl::App("index.html".into()))
                .title("ZomboLens")
                .decorations(false)
                .transparent(true)
                .shadow(false)
                .always_on_top(true)
                .skip_taskbar(true)
                .resizable(true)
                .focused(false)
                .inner_size(300.0, 420.0)
                .build()?;
            if let (Some(x), Some(y)) = (settings.x, settings.y) {
                let _ = window.set_position(PhysicalPosition::new(x, y));
            }
            window.on_window_event(move |e| {
                if let WindowEvent::Moved(p) = e {
                    let mut s = store.settings.lock();
                    s.x = Some(p.x);
                    s.y = Some(p.y);
                    drop(s);
                    store.save();
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running ZomboLens");
}
