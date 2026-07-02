mod capture;
mod commands;
mod models;
mod position;
mod restore;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            setup_tray(app)?;
            setup_shortcuts(app)?;
            setup_window_close(app);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::capture_snapshot,
            commands::restore_workspace,
            commands::get_screen_size,
            commands::close_apps,
            commands::write_file,
            commands::read_file,
            commands::show_window,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Stash");
}

fn setup_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::{
        menu::{Menu, MenuItem, PredefinedMenuItem},
        tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
        Manager,
    };

    let show_item = MenuItem::with_id(app, "show", "Open Stash", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let quit_item = MenuItem::with_id(app, "quit", "Quit Stash", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &sep, &quit_item])?;

    TrayIconBuilder::new()
        .icon(app.default_window_icon().cloned().expect("app icon missing"))
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("Stash – Workspace Manager")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "quit" => {
                std::process::exit(0);
            }
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
        })
        .build(app)?;

    Ok(())
}

fn setup_shortcuts(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri::{Emitter, Manager};
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    // Unregister first — a previous crashed/hot-reloaded instance may have left it registered.
    let _ = app.global_shortcut().unregister("Ctrl+Shift+S");

    let handle = app.handle().clone();
    app.global_shortcut()
        .on_shortcut("Ctrl+Shift+S", move |_app, _shortcut, _event| {
            if let Some(w) = handle.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
                let _ = handle.emit("palette-open", ());
            }
        })?;

    Ok(())
}

fn setup_window_close(app: &tauri::App) {
    use tauri::Manager;
    if let Some(window) = app.get_webview_window("main") {
        let win2 = window.clone();
        window.on_window_event(move |event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = win2.hide();
            }
        });
    }
}
