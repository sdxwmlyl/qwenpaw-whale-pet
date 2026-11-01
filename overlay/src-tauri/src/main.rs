#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bridge;

use std::fs;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, PhysicalPosition, WindowEvent,
};

static CLICK_THROUGH: AtomicBool = AtomicBool::new(false);
static LAST_SAVE: Mutex<Option<Instant>> = Mutex::new(None);

fn pos_file(app: &tauri::AppHandle) -> Option<std::path::PathBuf> {
    let dir = app.path().app_data_dir().ok()?;
    let _ = fs::create_dir_all(&dir);
    Some(dir.join("window_pos.txt"))
}

/// 把坐标钳制到当前显示器可见范围内（留 8px 边距，避免整块跑到屏外）
fn on_screen_xy(w: &tauri::WebviewWindow, x: i32, y: i32) -> Option<(i32, i32)> {
    let mon = w.current_monitor().ok().flatten()?;
    let ms = mon.size();
    let mp = mon.position();
    let ws = w.outer_size().ok()?;
    let (mw, mh) = (ms.width as i32, ms.height as i32);
    let (mwid, mhei) = (ws.width as i32, ws.height as i32);
    let inside = x >= mp.x
        && y >= mp.y
        && x + mwid <= mp.x + mw
        && y + mhei <= mp.y + mh;
    if inside {
        Some((x, y))
    } else {
        None
    }
}


fn read_pos(app: &tauri::AppHandle) -> Option<(i32, i32)> {
    let p = pos_file(app)?;
    let s = fs::read_to_string(p).ok()?;
    let mut it = s.trim().split(',');
    let x: i32 = it.next()?.trim().parse().ok()?;
    let y: i32 = it.next()?.trim().parse().ok()?;
    Some((x, y))
}

fn save_pos(app: &tauri::AppHandle, x: i32, y: i32) {
    // 拖动过程中会频繁触发 Moved，做 400ms 节流
    {
        let mut guard = match LAST_SAVE.lock() {
            Ok(g) => g,
            Err(_) => return,
        };
        if let Some(t) = *guard {
            if t.elapsed() < Duration::from_millis(400) {
                return;
            }
        }
        *guard = Some(Instant::now());
    }
    if let Some(p) = pos_file(app) {
        let _ = fs::write(p, format!("{},{}", x, y));
    }
}

fn place_logical(w: &tauri::WebviewWindow, lx: f64, ly: f64) {
    let sf = w.scale_factor().unwrap_or(1.0);
    let _ = w.set_position(PhysicalPosition::new((lx * sf) as i32, (ly * sf) as i32));
}

fn bottom_right(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if let Ok(Some(mon)) = w.current_monitor() {
            let ms = mon.size();
            if let Ok(ws) = w.outer_size() {
                let x = ms.width as f64 - ws.width as f64 - 60.0;
                let y = ms.height as f64 - ws.height as f64 - 90.0;
                place_logical(&w, x, y);
            }
        }
    }
}

fn top_left(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if let Ok(Some(mon)) = w.current_monitor() {
            let _ = mon.position();
            place_logical(&w, 40.0, 60.0);
        }
    }
}

#[tauri::command]
fn pet_set_size(window: tauri::WebviewWindow, size: f64) -> Result<(), String> {
    window
        .set_size(tauri::LogicalSize::new(size, size + 64.0))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn pet_hide(window: tauri::WebviewWindow) -> Result<(), String> {
    window.hide().map_err(|e| e.to_string())
}

#[tauri::command]
fn pet_quit(app: tauri::AppHandle) {
    app.exit(0);
}

#[tauri::command]
fn drag_start(window: tauri::WebviewWindow) -> Result<(), String> {
    window.start_dragging().map_err(|e| e.to_string())
}

fn toggle_window(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let visible = w.is_visible().unwrap_or(false);
        if visible {
            let _ = w.hide();
        } else {
            let _ = w.show();
            let _ = w.set_focus();
        }
    }
}

#[cfg(target_os = "windows")]
fn ensure_single_instance() {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    let name: Vec<u16> = OsStr::new("Global\\QwenPawWhalePetSingleton")
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    extern "system" {
        fn CreateMutexW(
            lp_mutex_attributes: *const std::ffi::c_void,
            b_initial_owner: i32,
            lp_name: *const u16,
        ) -> *mut std::ffi::c_void;
        fn GetLastError() -> u32;
    }
    unsafe {
        let handle = CreateMutexW(std::ptr::null(), 0, name.as_ptr());
        if handle.is_null() || GetLastError() == 183 {
            // 已经有实例在跑，直接退出，避免出现两个鲸鱼娘（其中一个还绑不上 7799）
            std::process::exit(0);
        }
    }
}

fn trace(msg: &str) {
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(r"D:\QwenPawWorkSpace\default\_main_trace.log")
    {
        let _ = writeln!(f, "{}", msg);
    }
}

fn main() {
    trace("main entered");
    #[cfg(target_os = "windows")]
    ensure_single_instance();
    trace("single-instance ok");

    tauri::Builder::default()
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "显示 / 隐藏", true, None::<&str>)?;
            let feed = MenuItem::with_id(app, "feed", "🍚 投喂", true, None::<&str>)?;
            let pat = MenuItem::with_id(app, "headpat", "🖐 摸头", true, None::<&str>)?;
            let thru = MenuItem::with_id(app, "thru", "👻 穿透模式（切换）", true, None::<&str>)?;
            let dodge = MenuItem::with_id(app, "dodge", "↖ 移到左上角（躲开审批窗）", true, None::<&str>)?;
            let reset = MenuItem::with_id(app, "reset", "↘ 复位到右下角", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &feed, &pat, &thru, &dodge, &reset, &quit])?;

            if let Some(icon) = app.default_window_icon().cloned() {
                let _ = TrayIconBuilder::new()
                    .icon(icon)
                    .tooltip("鲸鱼娘 · QwenPaw")
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "show" => toggle_window(app),
                        "feed" => {
                            let _ = ureq::post("http://127.0.0.1:7799/action")
                                .send_json(serde_json::json!({"act": "feed"}));
                        }
                        "headpat" => {
                            let _ = ureq::post("http://127.0.0.1:7799/action")
                                .send_json(serde_json::json!({"act": "headpat"}));
                        }
                        "dodge" => top_left(app),
                        "reset" => bottom_right(app),
                        "thru" => {
                            let on = !CLICK_THROUGH.load(Ordering::Relaxed);
                            CLICK_THROUGH.store(on, Ordering::Relaxed);
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.set_ignore_cursor_events(on);
                            }
                            let _ = app.emit("clickthrough", on);
                        }
                        "quit" => app.exit(0),
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            toggle_window(tray.app_handle());
                        }
                    })
                    .build(app)?;
            }

            if let Some(w) = app.get_webview_window("main") {
                match read_pos(app.handle()) {
                    Some((x, y)) => match on_screen_xy(&w, x, y) {
                        Some((cx, cy)) => {
                            let _ = w.set_position(PhysicalPosition::new(cx, cy));
                        }
                        None => {
                            // 保存的坐标在屏外（换显示器/分辨率/误拖）→ 回右下角并覆盖保存
                            bottom_right(app.handle());
                            let mon_w = w.current_monitor().ok().flatten().map(|m| m.size().width as i32).unwrap_or(1920);
                            let mon_h = w.current_monitor().ok().flatten().map(|m| m.size().height as i32).unwrap_or(1080);
                            let ws = w.outer_size().unwrap_or(tauri::PhysicalSize::new(240, 280));
                            save_pos(app.handle(), mon_w - ws.width as i32 - 60, mon_h - ws.height as i32 - 90);
                        }
                    },
                    None => bottom_right(app.handle()),
                }
                let _ = w.show();
            }
            trace("setup reached bridge start");
            let dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let _ = fs::create_dir_all(&dir);
            trace("calling bridge::start");
            bridge::start(dir);
            trace("bridge::start returned");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![drag_start, pet_set_size, pet_hide, pet_quit])
        .on_window_event(|window, event| {
            if let WindowEvent::Moved(pos) = event {
                save_pos(window.app_handle(), pos.x, pos.y);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
