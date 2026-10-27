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

fn bottom_right(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if let Ok(Some(mon)) = w.current_monitor() {
            let ms = mon.size();
            if let Ok(ws) = w.outer_size() {
                let x = ms.width as i32 - ws.width as i32 - 60;
                let y = ms.height as i32 - ws.height as i32 - 90;
                let _ = w.set_position(PhysicalPosition::new(x, y));
            }
        }
    }
}

fn top_left(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if let Ok(Some(mon)) = w.current_monitor() {
            let mp = mon.position();
            let _ = w.set_position(PhysicalPosition::new(mp.x + 40, mp.y + 60));
        }
    }
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

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let show = MenuItem::with_id(app, "show", "显示 / 隐藏", true, None::<&str>)?;
            let thru = MenuItem::with_id(app, "thru", "👻 穿透模式（切换）", true, None::<&str>)?;
            let dodge = MenuItem::with_id(app, "dodge", "↖ 移到左上角（躲开审批窗）", true, None::<&str>)?;
            let reset = MenuItem::with_id(app, "reset", "↘ 复位到右下角", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &thru, &dodge, &reset, &quit])?;

            if let Some(icon) = app.default_window_icon().cloned() {
                let _ = TrayIconBuilder::new()
                    .icon(icon)
                    .tooltip("鲸鱼娘 · QwenPaw")
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "show" => toggle_window(app),
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
                    Some((x, y)) => {
                        let _ = w.set_position(PhysicalPosition::new(x, y));
                    }
                    None => bottom_right(app.handle()),
                }
                let _ = w.show();
            }
            let dir = app
                .path()
                .app_data_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("."));
            let _ = fs::create_dir_all(&dir);
            bridge::start(dir);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![drag_start])
        .on_window_event(|window, event| {
            if let WindowEvent::Moved(pos) = event {
                save_pos(window.app_handle(), pos.x, pos.y);
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
