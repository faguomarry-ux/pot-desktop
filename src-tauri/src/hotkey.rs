use crate::config::{get, set};
use crate::window::{input_translate, ocr_recognize, ocr_translate, selection_translate};
use crate::APP;
use log::{info, warn};
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

fn register<F>(app_handle: &AppHandle, name: &str, handler: F, key: &str) -> Result<(), String>
where
    F: Fn() + Sync + Send + 'static,
{
    let hotkey = {
        if key.is_empty() {
            match get(name) {
                Some(v) => v.as_str().unwrap().to_string(),
                None => {
                    set(name, "");
                    String::new()
                }
            }
        } else {
            key.to_string()
        }
    };

    if !hotkey.is_empty() {
        #[cfg(target_os = "linux")]
        if crate::gnome_hotkey::enabled() {
            return crate::gnome_hotkey::register(name, &hotkey);
        }
        match app_handle
            .global_shortcut()
            .on_shortcut(hotkey.as_str(), move |_, _, event| {
                if event.state == ShortcutState::Pressed {
                    handler();
                }
            }) {
            Ok(()) => {
                info!("Registered global shortcut: {} for {}", hotkey, name);
            }
            Err(e) => {
                warn!("Failed to register global shortcut: {} {:?}", hotkey, e);
                return Err(e.to_string());
            }
        };
    }
    Ok(())
}

// Register global shortcuts
pub fn register_shortcut(shortcut: &str) -> Result<(), String> {
    let app_handle = APP.get().unwrap();
    match shortcut {
        "hotkey_selection_translate" => register(
            app_handle,
            "hotkey_selection_translate",
            selection_translate,
            "",
        )?,
        "hotkey_input_translate" => {
            register(app_handle, "hotkey_input_translate", input_translate, "")?
        }
        "hotkey_ocr_recognize" => register(app_handle, "hotkey_ocr_recognize", ocr_recognize, "")?,
        "hotkey_ocr_translate" => register(app_handle, "hotkey_ocr_translate", ocr_translate, "")?,
        "all" => {
            register(
                app_handle,
                "hotkey_selection_translate",
                selection_translate,
                "",
            )?;
            register(app_handle, "hotkey_input_translate", input_translate, "")?;
            register(app_handle, "hotkey_ocr_recognize", ocr_recognize, "")?;
            register(app_handle, "hotkey_ocr_translate", ocr_translate, "")?;
        }
        _ => {}
    }
    Ok(())
}

#[tauri::command]
pub fn register_shortcut_by_frontend(name: &str, shortcut: &str) -> Result<(), String> {
    let app_handle = APP.get().unwrap();
    match name {
        "hotkey_selection_translate" => register(
            app_handle,
            "hotkey_selection_translate",
            selection_translate,
            shortcut,
        )?,
        "hotkey_input_translate" => register(
            app_handle,
            "hotkey_input_translate",
            input_translate,
            shortcut,
        )?,
        "hotkey_ocr_recognize" => {
            register(app_handle, "hotkey_ocr_recognize", ocr_recognize, shortcut)?
        }
        "hotkey_ocr_translate" => {
            register(app_handle, "hotkey_ocr_translate", ocr_translate, shortcut)?
        }
        _ => {}
    }
    Ok(())
}

// GNOME launches a second process; the single-instance plugin forwards this action.
pub fn dispatch_action(args: &[String]) -> bool {
    let Some(action) = args
        .windows(2)
        .find(|a| a[0] == "--action")
        .map(|a| a[1].as_str())
    else {
        return false;
    };
    let handler: fn() = match action {
        "selection_translate" => selection_translate,
        "input_translate" => input_translate,
        "ocr_recognize" => ocr_recognize,
        "ocr_translate" => ocr_translate,
        _ => return false,
    };
    let action = action.to_owned();
    std::thread::spawn(move || {
        info!("Dispatch desktop shortcut: {action}");
        handler();
    });
    true
}

#[tauri::command]
pub fn shortcut_is_registered(shortcut: &str) -> Result<bool, String> {
    #[cfg(target_os = "linux")]
    if crate::gnome_hotkey::enabled() {
        return crate::gnome_hotkey::is_registered(shortcut);
    }
    Ok(APP.get().unwrap().global_shortcut().is_registered(shortcut))
}

#[tauri::command]
pub fn unregister_shortcut(shortcut: &str) -> Result<(), String> {
    #[cfg(target_os = "linux")]
    if crate::gnome_hotkey::enabled() {
        return crate::gnome_hotkey::unregister(shortcut);
    }
    if shortcut.is_empty() {
        return Ok(());
    }
    APP.get()
        .unwrap()
        .global_shortcut()
        .unregister(shortcut)
        .map_err(|e| e.to_string())
}
