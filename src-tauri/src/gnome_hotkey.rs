//! GNOME's native custom shortcuts also work when another Wayland app has focus.
//! Only Pot-owned entries are modified; other desktop shortcuts are preserved.
use gio::prelude::*;
use gio::{Settings, SettingsSchemaSource};
use once_cell::sync::Lazy;

const MEDIA: &str = "org.gnome.settings-daemon.plugins.media-keys";
const CUSTOM: &str = "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding";
const PREFIX: &str = "/org/gnome/settings-daemon/plugins/media-keys/pot-";
pub const ACTIONS: [&str; 4] = [
    "selection_translate",
    "input_translate",
    "ocr_recognize",
    "ocr_translate",
];

pub fn enabled() -> bool {
    static ENABLED: Lazy<bool> = Lazy::new(|| {
        std::env::var_os("WAYLAND_DISPLAY").is_some()
            && std::env::var("XDG_CURRENT_DESKTOP")
                .unwrap_or_default()
                .to_lowercase()
                .contains("gnome")
    });
    *ENABLED
}

#[cfg(test)]
thread_local! { static TEST_BACKEND: gio::SettingsBackend = gio::memory_settings_backend_new(); }

fn settings(schema: &str, path: Option<&str>) -> Result<Settings, String> {
    let source = SettingsSchemaSource::default().ok_or("GSettings schemas unavailable")?;
    let schema = source
        .lookup(schema, true)
        .ok_or("GNOME custom shortcut schema unavailable")?;
    #[cfg(test)]
    return Ok(TEST_BACKEND.with(|backend| Settings::new_full(&schema, Some(backend), path)));
    #[cfg(not(test))]
    Ok(Settings::new_full(
        &schema,
        None::<&gio::SettingsBackend>,
        path,
    ))
}

fn accelerator(key: &str) -> Result<String, String> {
    let mut modifiers = String::new();
    let mut symbol = None;
    for part in key.split('+') {
        let modifier = match part.to_lowercase().as_str() {
            "ctrl" | "control" | "commandorcontrol" => Some("<Control>"),
            "shift" => Some("<Shift>"),
            "alt" => Some("<Alt>"),
            "super" | "meta" | "command" => Some("<Super>"),
            _ => None,
        };
        if let Some(value) = modifier {
            modifiers.push_str(value);
        } else if symbol.is_none() {
            symbol = Some(match part.to_lowercase().as_str() {
                "esc" => "Escape".into(),
                "space" => "space".into(),
                "plus" => "plus".into(),
                "pagedown" => "Page_Down".into(),
                "pageup" => "Page_Up".into(),
                "capslock" => "Caps_Lock".into(),
                "printscreen" => "Print".into(),
                "scrolllock" => "Scroll_Lock".into(),
                "contextmenu" => "Menu".into(),
                "`" => "grave".into(),
                "\\" => "backslash".into(),
                "[" => "bracketleft".into(),
                "]" => "bracketright".into(),
                "," => "comma".into(),
                "=" => "equal".into(),
                "-" => "minus".into(),
                "." => "period".into(),
                "'" => "apostrophe".into(),
                ";" => "semicolon".into(),
                "/" => "slash".into(),
                _ if part.len() == 1 => part.to_lowercase(),
                _ => part.to_string(),
            });
        } else {
            return Err("Shortcut must contain one non-modifier key".into());
        }
    }
    let symbol = symbol
        .filter(|s| !s.is_empty())
        .ok_or("Shortcut is missing a key")?;
    Ok(format!("{modifiers}{symbol}"))
}

pub fn register(name: &str, key: &str) -> Result<(), String> {
    let action = name
        .strip_prefix("hotkey_")
        .filter(|a| ACTIONS.contains(a))
        .ok_or("Unknown shortcut action")?;
    let binding = accelerator(key)?;
    let parent = settings(MEDIA, None)?;
    let path = format!("{PREFIX}{action}/");
    let mut paths: Vec<String> = parent
        .strv("custom-keybindings")
        .iter()
        .map(|p| p.to_string())
        .collect();
    // Do not steal an existing user-defined shortcut.
    for other in &paths {
        if other != &path && settings(CUSTOM, Some(other))?.string("binding") == binding {
            return Err(format!("Shortcut {key} is already assigned in GNOME"));
        }
    }
    let item = settings(CUSTOM, Some(&path))?;
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    // GSettings commands use shell syntax; quote the executable even for paths containing spaces.
    let quoted = format!("'{}'", executable.to_string_lossy().replace('\'', "'\\''"));
    item.set_string("name", &format!("Pot: {action}"))
        .map_err(|e| e.to_string())?;
    item.set_string("command", &format!("{quoted} --action {action}"))
        .map_err(|e| e.to_string())?;
    item.set_string("binding", &binding)
        .map_err(|e| e.to_string())?;
    if !paths.contains(&path) {
        paths.push(path);
        parent
            .set_strv("custom-keybindings", paths)
            .map_err(|e| e.to_string())?;
    }
    Settings::sync();
    log::info!("Registered GNOME Wayland shortcut: {key} for {name}");
    Ok(())
}

pub fn is_registered(key: &str) -> Result<bool, String> {
    if key.is_empty() {
        return Ok(false);
    }
    let binding = accelerator(key)?;
    for path in settings(MEDIA, None)?.strv("custom-keybindings").iter() {
        if settings(CUSTOM, Some(path))?.string("binding") == binding {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn unregister(key: &str) -> Result<(), String> {
    if key.is_empty() {
        return Ok(());
    }
    let binding = accelerator(key)?;
    let parent = settings(MEDIA, None)?;
    let mut retained = Vec::new();
    for path in parent.strv("custom-keybindings").iter() {
        if path.starts_with(PREFIX) {
            let item = settings(CUSTOM, Some(path))?;
            if item.string("binding") == binding {
                for field in ["name", "command", "binding"] {
                    item.reset(field);
                }
                continue;
            }
        }
        retained.push(path.to_string());
    }
    parent
        .set_strv("custom-keybindings", retained)
        .map_err(|e| e.to_string())?;
    Settings::sync();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn converts_shortcuts_used_by_the_settings_ui() {
        assert_eq!(accelerator("Shift+Alt+E").unwrap(), "<Shift><Alt>e");
        assert_eq!(
            accelerator("Ctrl+Super+PageDown").unwrap(),
            "<Control><Super>Page_Down"
        );
        assert!(accelerator("Shift+Alt").is_err());
        assert!(accelerator("Ctrl+A+B").is_err());
    }
    #[test]
    fn native_registration_preserves_other_desktop_shortcuts() {
        let parent = settings(MEDIA, None).unwrap();
        let external =
            "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/pot-test-external/";
        parent.set_strv("custom-keybindings", [external]).unwrap();
        let item = settings(CUSTOM, Some(external)).unwrap();
        item.set_string("binding", "<Control>q").unwrap();
        item.set_string("command", "unrelated-command").unwrap();
        register("hotkey_selection_translate", "Shift+Alt+E").unwrap();
        assert!(is_registered("Shift+Alt+E").unwrap());
        let own = settings(CUSTOM, Some(&format!("{PREFIX}selection_translate/"))).unwrap();
        assert!(own
            .string("command")
            .ends_with(" --action selection_translate"));
        assert!(register("hotkey_input_translate", "Ctrl+Q").is_err());
        unregister("Shift+Alt+E").unwrap();
        assert!(!is_registered("Shift+Alt+E").unwrap());
        assert_eq!(
            parent
                .strv("custom-keybindings")
                .iter()
                .map(|p| p.as_str())
                .collect::<Vec<_>>(),
            vec![external]
        );
        assert_eq!(item.string("command"), "unrelated-command");
    }
}
