#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{
    menu::{Menu, MenuItem},
    Emitter,
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

static CLICK_THROUGH: AtomicBool = AtomicBool::new(false);

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
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &thru, &quit])?;

            if let Some(icon) = app.default_window_icon().cloned() {
                let _ = TrayIconBuilder::new()
                    .icon(icon)
                    .tooltip("鲸鱼娘 · QwenPaw")
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "show" => toggle_window(app),
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
                if let Ok(Some(mon)) = w.current_monitor() {
                    let ms = mon.size();
                    if let Ok(ws) = w.outer_size() {
                        let x = ms.width as i32 - ws.width as i32 - 60;
                        let y = ms.height as i32 - ws.height as i32 - 90;
                        let _ = w.set_position(tauri::PhysicalPosition::new(x, y));
                    }
                }
                let _ = w.show();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
