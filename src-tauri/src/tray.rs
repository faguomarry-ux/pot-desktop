use crate::clipboard::*;
use crate::config::{get, set};
use crate::window::config_window;
use crate::window::input_translate;
use crate::window::ocr_recognize;
use crate::window::ocr_translate;
use crate::window::updater_window;
use log::info;
use tauri::menu::{CheckMenuItem, Menu, MenuBuilder, MenuEvent, MenuItem, SubmenuBuilder};
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::GlobalShortcutExt;

#[tauri::command]
pub fn update_tray(app_handle: tauri::AppHandle, mut language: String, mut copy_mode: String) {
    let tray_handle = app_handle.tray_by_id("main").expect("Tray initialized");

    if language.is_empty() {
        language = match get("app_language") {
            Some(v) => v.as_str().unwrap().to_string(),
            None => {
                set("app_language", "en");
                "en".to_string()
            }
        };
    }
    if copy_mode.is_empty() {
        copy_mode = match get("translate_auto_copy") {
            Some(v) => v.as_str().unwrap().to_string(),
            None => {
                set("translate_auto_copy", "disable");
                "disable".to_string()
            }
        };
    }

    info!(
        "Update tray with language: {}, copy mode: {}",
        language, copy_mode
    );
    tray_handle
        .set_menu(Some(match language.as_str() {
            "en" => tray_menu_en(),
            "zh_cn" => tray_menu_zh_cn(),
            "zh_tw" => tray_menu_zh_tw(),
            "ja" => tray_menu_ja(),
            "ko" => tray_menu_ko(),
            "fr" => tray_menu_fr(),
            "de" => tray_menu_de(),
            "ru" => tray_menu_ru(),
            "pt_br" => tray_menu_pt_br(),
            "fa" => tray_menu_fa(),
            "uk" => tray_menu_uk(),
            _ => tray_menu_en(),
        }))
        .unwrap();
    #[cfg(not(target_os = "linux"))]
    tray_handle
        .set_tooltip(&format!("pot {}", app_handle.package_info().version))
        .unwrap();
}

pub fn tray_event_handler(app: &AppHandle, event: MenuEvent) {
    match event.id().as_ref() {
        "input_translate" => on_input_translate_click(),
        "copy_source" => on_auto_copy_click(app, "source"),
        "clipboard_monitor" => on_clipboard_monitor_click(app),
        "copy_target" => on_auto_copy_click(app, "target"),
        "copy_source_target" => on_auto_copy_click(app, "source_target"),
        "copy_disable" => on_auto_copy_click(app, "disable"),
        "ocr_recognize" => on_ocr_recognize_click(),
        "ocr_translate" => on_ocr_translate_click(),
        "config" => on_config_click(),
        "check_update" => on_check_update_click(),
        "view_log" => on_view_log_click(app),
        "restart" => on_restart_click(app),
        "quit" => on_quit_click(app),
        _ => {}
    }
}

#[cfg(target_os = "windows")]
pub fn on_tray_click() {
    let event = match get("tray_click_event") {
        Some(v) => v.as_str().unwrap().to_string(),
        None => {
            set("tray_click_event", "config");
            "config".to_string()
        }
    };
    match event.as_str() {
        "config" => config_window(),
        "translate" => input_translate(),
        "ocr_recognize" => ocr_recognize(),
        "ocr_translate" => ocr_translate(),
        "disable" => {}
        _ => config_window(),
    }
}
fn on_input_translate_click() {
    input_translate();
}
fn on_clipboard_monitor_click(app: &AppHandle) {
    let enable_clipboard_monitor = match get("clipboard_monitor") {
        Some(v) => v.as_bool().unwrap(),
        None => {
            set("clipboard_monitor", false);
            false
        }
    };
    let current = !enable_clipboard_monitor;
    // Update Config File
    set("clipboard_monitor", current);
    // Update State and Start Monitor
    let state = app.state::<ClipboardMonitorEnableWrapper>();
    state
        .0
        .lock()
        .unwrap()
        .replace_range(.., &current.to_string());
    if current {
        start_clipboard_monitor(app.clone());
    }
    // Update Tray Menu Status
    update_tray(app.clone(), String::new(), String::new());
}
fn on_auto_copy_click(app: &AppHandle, mode: &str) {
    info!("Set copy mode to: {}", mode);
    set("translate_auto_copy", mode);
    app.emit("translate_auto_copy_changed", mode).unwrap();
    update_tray(app.clone(), "".to_string(), mode.to_string());
}
fn on_ocr_recognize_click() {
    ocr_recognize();
}
fn on_ocr_translate_click() {
    ocr_translate();
}

fn on_config_click() {
    config_window();
}

fn on_check_update_click() {
    updater_window();
}
fn on_view_log_click(app: &AppHandle) {
    use tauri_plugin_shell::ShellExt;
    let log_path = app.path().app_log_dir().unwrap();
    app.shell().open(log_path.to_string_lossy(), None).unwrap();
}
fn on_restart_click(app: &AppHandle) {
    info!("============== Restart App ==============");
    app.restart();
}
fn on_quit_click(app: &AppHandle) {
    app.global_shortcut().unregister_all().unwrap();
    info!("============== Quit App ==============");
    app.exit(0);
}

fn tray_menu_en() -> Menu<tauri::Wry> {
    let app = crate::APP.get().unwrap();
    let input_translate = menu_item(app, "input_translate", "Input Translate");
    let copy_source = menu_item(app, "copy_source", "Source");
    let copy_target = menu_item(app, "copy_target", "Target");
    let clipboard_monitor = menu_item(app, "clipboard_monitor", "Clipboard Monitor");
    let copy_source_target = menu_item(app, "copy_source_target", "Source+Target");
    let copy_disable = menu_item(app, "copy_disable", "Disable");
    let ocr_recognize = menu_item(app, "ocr_recognize", "OCR Recognize");
    let ocr_translate = menu_item(app, "ocr_translate", "OCR Translate");
    let config = menu_item(app, "config", "Config");
    let check_update = menu_item(app, "check_update", "Check Update");
    let view_log = menu_item(app, "view_log", "View Log");
    let restart = menu_item(app, "restart", "Restart");
    let quit = menu_item(app, "quit", "Quit");
    MenuBuilder::new(app)
        .item(&input_translate)
        .item(&clipboard_monitor)
        .item(
            &SubmenuBuilder::new(app, "Auto Copy")
                .item(&copy_source)
                .item(&copy_target)
                .item(&copy_source_target)
                .separator()
                .item(&copy_disable)
                .build()
                .unwrap(),
        )
        .separator()
        .item(&ocr_recognize)
        .item(&ocr_translate)
        .separator()
        .item(&config)
        .item(&check_update)
        .item(&view_log)
        .separator()
        .item(&restart)
        .item(&quit)
        .build()
        .unwrap()
}

fn tray_menu_zh_cn() -> Menu<tauri::Wry> {
    let app = crate::APP.get().unwrap();
    let input_translate = menu_item(app, "input_translate", "输入翻译");
    let clipboard_monitor = menu_item(app, "clipboard_monitor", "监听剪切板");
    let copy_source = menu_item(app, "copy_source", "原文");
    let copy_target = menu_item(app, "copy_target", "译文");

    let copy_source_target = menu_item(app, "copy_source_target", "原文+译文");
    let copy_disable = menu_item(app, "copy_disable", "关闭");
    let ocr_recognize = menu_item(app, "ocr_recognize", "文字识别");
    let ocr_translate = menu_item(app, "ocr_translate", "截图翻译");
    let config = menu_item(app, "config", "偏好设置");
    let check_update = menu_item(app, "check_update", "检查更新");
    let restart = menu_item(app, "restart", "重启应用");
    let view_log = menu_item(app, "view_log", "查看日志");
    let quit = menu_item(app, "quit", "退出");
    MenuBuilder::new(app)
        .item(&input_translate)
        .item(&clipboard_monitor)
        .item(
            &SubmenuBuilder::new(app, "自动复制")
                .item(&copy_source)
                .item(&copy_target)
                .item(&copy_source_target)
                .separator()
                .item(&copy_disable)
                .build()
                .unwrap(),
        )
        .separator()
        .item(&ocr_recognize)
        .item(&ocr_translate)
        .separator()
        .item(&config)
        .item(&check_update)
        .item(&view_log)
        .separator()
        .item(&restart)
        .item(&quit)
        .build()
        .unwrap()
}

fn tray_menu_zh_tw() -> Menu<tauri::Wry> {
    let app = crate::APP.get().unwrap();
    let input_translate = menu_item(app, "input_translate", "輸入翻譯");
    let clipboard_monitor = menu_item(app, "clipboard_monitor", "偵聽剪貼簿");
    let copy_source = menu_item(app, "copy_source", "原文");
    let copy_target = menu_item(app, "copy_target", "譯文");

    let copy_source_target = menu_item(app, "copy_source_target", "原文+譯文");
    let copy_disable = menu_item(app, "copy_disable", "關閉");
    let ocr_recognize = menu_item(app, "ocr_recognize", "文字識別");
    let ocr_translate = menu_item(app, "ocr_translate", "截圖翻譯");
    let config = menu_item(app, "config", "偏好設定");
    let check_update = menu_item(app, "check_update", "檢查更新");
    let restart = menu_item(app, "restart", "重啓程式");
    let view_log = menu_item(app, "view_log", "查看日誌");
    let quit = menu_item(app, "quit", "退出");
    MenuBuilder::new(app)
        .item(&input_translate)
        .item(&clipboard_monitor)
        .item(
            &SubmenuBuilder::new(app, "自動複製")
                .item(&copy_source)
                .item(&copy_target)
                .item(&copy_source_target)
                .separator()
                .item(&copy_disable)
                .build()
                .unwrap(),
        )
        .separator()
        .item(&ocr_recognize)
        .item(&ocr_translate)
        .separator()
        .item(&config)
        .item(&check_update)
        .item(&view_log)
        .separator()
        .item(&restart)
        .item(&quit)
        .build()
        .unwrap()
}

fn tray_menu_ja() -> Menu<tauri::Wry> {
    let app = crate::APP.get().unwrap();
    let input_translate = menu_item(app, "input_translate", "翻訳を入力");
    let clipboard_monitor = menu_item(app, "clipboard_monitor", "クリップボードを監視する");
    let copy_source = menu_item(app, "copy_source", "原文");
    let copy_target = menu_item(app, "copy_target", "訳文");

    let copy_source_target = menu_item(app, "copy_source_target", "原文+訳文");
    let copy_disable = menu_item(app, "copy_disable", "閉じる");
    let ocr_recognize = menu_item(app, "ocr_recognize", "テキスト認識");
    let ocr_translate = menu_item(app, "ocr_translate", "スクリーンショットの翻訳");
    let config = menu_item(app, "config", "プリファレンス設定");
    let check_update = menu_item(app, "check_update", "更新を確認する");
    let restart = menu_item(app, "restart", "アプリの再起動");
    let view_log = menu_item(app, "view_log", "ログを見る");
    let quit = menu_item(app, "quit", "退出する");
    MenuBuilder::new(app)
        .item(&input_translate)
        .item(&clipboard_monitor)
        .item(
            &SubmenuBuilder::new(app, "自動コピー")
                .item(&copy_source)
                .item(&copy_target)
                .item(&copy_source_target)
                .separator()
                .item(&copy_disable)
                .build()
                .unwrap(),
        )
        .separator()
        .item(&ocr_recognize)
        .item(&ocr_translate)
        .separator()
        .item(&config)
        .item(&check_update)
        .item(&view_log)
        .separator()
        .item(&restart)
        .item(&quit)
        .build()
        .unwrap()
}

fn tray_menu_ko() -> Menu<tauri::Wry> {
    let app = crate::APP.get().unwrap();
    let input_translate = menu_item(app, "input_translate", "입력 번역");
    let clipboard_monitor = menu_item(app, "clipboard_monitor", "감청 전단판");
    let copy_source = menu_item(app, "copy_source", "원문");
    let copy_target = menu_item(app, "copy_target", "번역문");

    let copy_source_target = menu_item(app, "copy_source_target", "원문+번역문");
    let copy_disable = menu_item(app, "copy_disable", "닫기");
    let ocr_recognize = menu_item(app, "ocr_recognize", "문자인식");
    let ocr_translate = menu_item(app, "ocr_translate", "스크린샷 번역");
    let config = menu_item(app, "config", "기본 설정");
    let check_update = menu_item(app, "check_update", "업데이트 확인");
    let restart = menu_item(app, "restart", "응용 프로그램 다시 시작");
    let view_log = menu_item(app, "view_log", "로그 보기");
    let quit = menu_item(app, "quit", "퇴출");
    MenuBuilder::new(app)
        .item(&input_translate)
        .item(&clipboard_monitor)
        .item(
            &SubmenuBuilder::new(app, "자동 복사")
                .item(&copy_source)
                .item(&copy_target)
                .item(&copy_source_target)
                .separator()
                .item(&copy_disable)
                .build()
                .unwrap(),
        )
        .separator()
        .item(&ocr_recognize)
        .item(&ocr_translate)
        .separator()
        .item(&config)
        .item(&check_update)
        .item(&view_log)
        .separator()
        .item(&restart)
        .item(&quit)
        .build()
        .unwrap()
}

fn tray_menu_fr() -> Menu<tauri::Wry> {
    let app = crate::APP.get().unwrap();
    let input_translate = menu_item(app, "input_translate", "Traduction d'entrée");
    let clipboard_monitor = menu_item(app, "clipboard_monitor", "Surveiller le presse-papiers");
    let copy_source = menu_item(app, "copy_source", "Source");
    let copy_target = menu_item(app, "copy_target", "Cible");

    let copy_source_target = menu_item(app, "copy_source_target", "Source+Cible");
    let copy_disable = menu_item(app, "copy_disable", "Désactiver");
    let ocr_recognize = menu_item(app, "ocr_recognize", "Reconnaissance de texte");
    let ocr_translate = menu_item(app, "ocr_translate", "Traduction d'image");
    let config = menu_item(app, "config", "Paramètres");
    let check_update = menu_item(app, "check_update", "Vérifier les mises à jour");
    let restart = menu_item(app, "restart", "Redémarrer l'application");
    let view_log = menu_item(app, "view_log", "Voir le journal");
    let quit = menu_item(app, "quit", "Quitter");
    MenuBuilder::new(app)
        .item(&input_translate)
        .item(&clipboard_monitor)
        .item(
            &SubmenuBuilder::new(app, "Copier automatiquement")
                .item(&copy_source)
                .item(&copy_target)
                .item(&copy_source_target)
                .separator()
                .item(&copy_disable)
                .build()
                .unwrap(),
        )
        .separator()
        .item(&ocr_recognize)
        .item(&ocr_translate)
        .separator()
        .item(&config)
        .item(&check_update)
        .item(&view_log)
        .separator()
        .item(&restart)
        .item(&quit)
        .build()
        .unwrap()
}
fn tray_menu_de() -> Menu<tauri::Wry> {
    let app = crate::APP.get().unwrap();
    let input_translate = menu_item(app, "input_translate", "Eingabeübersetzung");
    let clipboard_monitor = menu_item(app, "clipboard_monitor", "Zwischenablage überwachen");
    let copy_source = menu_item(app, "copy_source", "Quelle");
    let copy_target = menu_item(app, "copy_target", "Ziel");

    let copy_source_target = menu_item(app, "copy_source_target", "Quelle+Ziel");
    let copy_disable = menu_item(app, "copy_disable", "Deaktivieren");
    let ocr_recognize = menu_item(app, "ocr_recognize", "Texterkennung");
    let ocr_translate = menu_item(app, "ocr_translate", "Bildübersetzung");
    let config = menu_item(app, "config", "Einstellungen");
    let check_update = menu_item(app, "check_update", "Auf Updates prüfen");
    let restart = menu_item(app, "restart", "Anwendung neu starten");
    let view_log = menu_item(app, "view_log", "Protokoll anzeigen");
    let quit = menu_item(app, "quit", "Beenden");
    MenuBuilder::new(app)
        .item(&input_translate)
        .item(&clipboard_monitor)
        .item(
            &SubmenuBuilder::new(app, "Automatisch kopieren")
                .item(&copy_source)
                .item(&copy_target)
                .item(&copy_source_target)
                .separator()
                .item(&copy_disable)
                .build()
                .unwrap(),
        )
        .separator()
        .item(&ocr_recognize)
        .item(&ocr_translate)
        .separator()
        .item(&config)
        .item(&check_update)
        .item(&view_log)
        .separator()
        .item(&restart)
        .item(&quit)
        .build()
        .unwrap()
}

fn tray_menu_ru() -> Menu<tauri::Wry> {
    let app = crate::APP.get().unwrap();
    let input_translate = menu_item(app, "input_translate", "Ввод перевода");
    let clipboard_monitor = menu_item(app, "clipboard_monitor", "Следить за буфером обмена");
    let copy_source = menu_item(app, "copy_source", "Источник");
    let copy_target = menu_item(app, "copy_target", "Цель");

    let copy_source_target = menu_item(app, "copy_source_target", "Источник+Цель");
    let copy_disable = menu_item(app, "copy_disable", "Отключить");
    let ocr_recognize = menu_item(app, "ocr_recognize", "Распознавание текста");
    let ocr_translate = menu_item(app, "ocr_translate", "Перевод изображения");
    let config = menu_item(app, "config", "Настройки");
    let check_update = menu_item(app, "check_update", "Проверить обновления");
    let restart = menu_item(app, "restart", "Перезапустить приложение");
    let view_log = menu_item(app, "view_log", "Просмотр журнала");
    let quit = menu_item(app, "quit", "Выход");
    MenuBuilder::new(app)
        .item(&input_translate)
        .item(&clipboard_monitor)
        .item(
            &SubmenuBuilder::new(app, "Автоматическое копирование")
                .item(&copy_source)
                .item(&copy_target)
                .item(&copy_source_target)
                .separator()
                .item(&copy_disable)
                .build()
                .unwrap(),
        )
        .separator()
        .item(&ocr_recognize)
        .item(&ocr_translate)
        .separator()
        .item(&config)
        .item(&check_update)
        .item(&view_log)
        .separator()
        .item(&restart)
        .item(&quit)
        .build()
        .unwrap()
}

fn tray_menu_fa() -> Menu<tauri::Wry> {
    let app = crate::APP.get().unwrap();
    let input_translate = menu_item(app, "input_translate", "متن");
    let clipboard_monitor = menu_item(app, "clipboard_monitor", "گوش دادن به تخته برش");
    let copy_source = menu_item(app, "copy_source", "منبع");
    let copy_target = menu_item(app, "copy_target", "هدف");

    let copy_source_target = menu_item(app, "copy_source_target", "منبع + هدف");
    let copy_disable = menu_item(app, "copy_disable", "متن");
    let ocr_recognize = menu_item(app, "ocr_recognize", "تشخیص متن");
    let ocr_translate = menu_item(app, "ocr_translate", "ترجمه عکس");
    let config = menu_item(app, "config", "تنظیمات ترجیح");
    let check_update = menu_item(app, "check_update", "بررسی بروزرسانی");
    let restart = menu_item(app, "restart", "راه‌اندازی مجدد برنامه");
    let view_log = menu_item(app, "view_log", "مشاهده گزارشات");
    let quit = menu_item(app, "quit", "خروج");
    MenuBuilder::new(app)
        .item(&input_translate)
        .item(&clipboard_monitor)
        .item(
            &SubmenuBuilder::new(app, "کپی خودکار")
                .item(&copy_source)
                .item(&copy_target)
                .item(&copy_source_target)
                .separator()
                .item(&copy_disable)
                .build()
                .unwrap(),
        )
        .separator()
        .item(&ocr_recognize)
        .item(&ocr_translate)
        .separator()
        .item(&config)
        .item(&check_update)
        .item(&view_log)
        .separator()
        .item(&restart)
        .item(&quit)
        .build()
        .unwrap()
}

fn tray_menu_pt_br() -> Menu<tauri::Wry> {
    let app = crate::APP.get().unwrap();
    let input_translate = menu_item(app, "input_translate", "Traduzir Entrada");
    let clipboard_monitor = menu_item(
        app,
        "clipboard_monitor",
        "Monitorando a área de transferência",
    );
    let copy_source = menu_item(app, "copy_source", "Origem");
    let copy_target = menu_item(app, "copy_target", "Destino");

    let copy_source_target = menu_item(app, "copy_source_target", "Origem+Destino");
    let copy_disable = menu_item(app, "copy_disable", "Desabilitar");
    let ocr_recognize = menu_item(app, "ocr_recognize", "Reconhecimento de Texto");
    let ocr_translate = menu_item(app, "ocr_translate", "Tradução de Imagem");
    let config = menu_item(app, "config", "Configurações");
    let check_update = menu_item(app, "check_update", "Checar por Atualização");
    let restart = menu_item(app, "restart", "Reiniciar aplicativo");
    let view_log = menu_item(app, "view_log", "Exibir Registro");
    let quit = menu_item(app, "quit", "Sair");
    MenuBuilder::new(app)
        .item(&input_translate)
        .item(&clipboard_monitor)
        .item(
            &SubmenuBuilder::new(app, "Copiar Automaticamente")
                .item(&copy_source)
                .item(&copy_target)
                .item(&copy_source_target)
                .separator()
                .item(&copy_disable)
                .build()
                .unwrap(),
        )
        .separator()
        .item(&ocr_recognize)
        .item(&ocr_translate)
        .separator()
        .item(&config)
        .item(&check_update)
        .item(&view_log)
        .separator()
        .item(&restart)
        .item(&quit)
        .build()
        .unwrap()
}

fn tray_menu_uk() -> Menu<tauri::Wry> {
    let app = crate::APP.get().unwrap();
    let input_translate = menu_item(app, "input_translate", "Введення перекладу");
    let clipboard_monitor = menu_item(app, "clipboard_monitor", "Стежити за буфером обміну");
    let copy_source = menu_item(app, "copy_source", "Джерело");
    let copy_target = menu_item(app, "copy_target", "Мета");

    let copy_source_target = menu_item(app, "copy_source_target", "Джерело+Мета");
    let copy_disable = menu_item(app, "copy_disable", "Відключивши");
    let ocr_recognize = menu_item(app, "ocr_recognize", "Розпізнавання тексту");
    let ocr_translate = menu_item(app, "ocr_translate", "Переклад зображення");
    let config = menu_item(app, "config", "Настройка");
    let check_update = menu_item(app, "check_update", "Перевірити оновлення");
    let restart = menu_item(app, "restart", "Перезапустити додаток");
    let view_log = menu_item(app, "view_log", "Перегляд журналу");
    let quit = menu_item(app, "quit", "Вихід");
    MenuBuilder::new(app)
        .item(&input_translate)
        .item(&clipboard_monitor)
        .item(
            &SubmenuBuilder::new(app, "Автоматичне копіювання")
                .item(&copy_source)
                .item(&copy_target)
                .item(&copy_source_target)
                .separator()
                .item(&copy_disable)
                .build()
                .unwrap(),
        )
        .separator()
        .item(&ocr_recognize)
        .item(&ocr_translate)
        .separator()
        .item(&config)
        .item(&check_update)
        .item(&view_log)
        .separator()
        .item(&restart)
        .item(&quit)
        .build()
        .unwrap()
}

fn menu_item(app: &AppHandle, id: &str, text: &str) -> tauri::menu::MenuItemKind<tauri::Wry> {
    if id == "clipboard_monitor" || id.starts_with("copy_") {
        let checked = if id == "clipboard_monitor" {
            get("clipboard_monitor")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
        } else {
            let mode = get("translate_auto_copy")
                .and_then(|v| v.as_str().map(str::to_owned))
                .unwrap_or_else(|| "disable".into());
            id == format!("copy_{}", mode)
        };
        tauri::menu::MenuItemKind::Check(
            CheckMenuItem::with_id(app, id, text, true, checked, None::<&str>).unwrap(),
        )
    } else {
        tauri::menu::MenuItemKind::MenuItem(
            MenuItem::with_id(app, id, text, true, None::<&str>).unwrap(),
        )
    }
}
