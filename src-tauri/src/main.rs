use std::{fs, thread};

use std::str::FromStr;
use std::sync::{mpsc, RwLock};

use anyhow::anyhow;

use chrono::Local;
use log::LevelFilter;
use maguromate_core::app_config::AppConfig;
use maguromate_core::grab::grab;
use maguromate_core::js::{ConfigSchemaList, JS};
use tauri::{
    CustomMenuItem, Manager, SystemTray, SystemTrayEvent, SystemTrayMenu, SystemTrayMenuItem,
    WindowBuilder,
};

const APP_NAME: &str = "onemoretime";

static mut LOG_LEVEL: RwLock<LevelFilter> = RwLock::new(LevelFilter::Info);

#[tauri::command]
fn get_config_schema() -> Result<ConfigSchemaList, String> {
    let mut js = JS::new(None).map_err(|err| format!("{:?}", err))?;
    js.get_config_schema().map_err(|err| format!("{:?}", err))
}

#[tauri::command]
fn load_config() -> Result<AppConfig, String> {
    AppConfig::load().map_err(|err| format!("{:?}", err))
}

#[tauri::command]
fn save_config(config: AppConfig) -> Result<(), String> {
    let result = config.save().map_err(|err| format!("{:?}", err));
    set_log_level_by_config(&config);
    result
}

fn set_log_level_by_config(app_config: &AppConfig) {
    let level_filter = match LevelFilter::from_str(app_config.log_level.as_str()) {
        Ok(level) => level,
        Err(err) => {
            log::error!(
                "Unknown log level in configuration: {:?},{:?}",
                app_config.log_level,
                err
            );
            LevelFilter::Info
        }
    };
    set_log_level(level_filter);
}
fn set_log_level(level_filter: LevelFilter) {
    unsafe {
        eprintln!("Setting log level to {:?}", level_filter);
        *LOG_LEVEL.write().unwrap() = level_filter;
        log::info!("Set log level to `{}`", level_filter);
    }
}

fn logger() -> anyhow::Result<()> {
    let log_path = dirs::data_dir()
        .unwrap()
        .join(APP_NAME)
        .join("onemoretime.log");
    log::info!("Logging file is output to {:?}", log_path);
    fs::create_dir_all(log_path.parent().unwrap())
        .map_err(|err| anyhow!("Cannot create {:?}: {:?}", log_path, err))?;

    fern::Dispatch::new()
        .format(|out, message, record| {
            out.finish(format_args!(
                "[{} {} {}] {}",
                Local::now().to_rfc3339(),
                record.level(),
                record.target(),
                message
            ))
        })
        .filter(|metadata| unsafe { metadata.level() <= *LOG_LEVEL.read().unwrap() })
        .chain(std::io::stdout())
        .chain(fern::log_file(log_path)?)
        .apply()?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    logger()?;

    let app_config = AppConfig::load()?;
    set_log_level_by_config(&app_config);

    let (tx, rx) = mpsc::channel::<bool>();

    thread::spawn(move || {
        log::debug!("Starting handler thread: {:?}", thread::current().id());
        let js = JS::new(Some(rx)).expect("Cannot create JS instance");
        if let Err(err) = grab(js) {
            log::error!("Cannot run handler: {:?}", err);
        }
    });

    log::debug!("Creating menu object");

    let quit = CustomMenuItem::new("quit".to_string(), "Quit");
    let configuration = CustomMenuItem::new("configuration".to_string(), "Configuration");
    let tray_menu = SystemTrayMenu::new()
        .add_item(configuration)
        .add_native_item(SystemTrayMenuItem::Separator) // separator
        .add_item(quit);

    let tray = SystemTray::new().with_menu(tray_menu);

    log::debug!("Building tauri");

    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .setup(|app| {
            app.listen_global("update-config", move |event| {
                log::info!("update-config: {:?}", event);
                tx.send(true).expect("Send message");
            });
            Ok(())
        })
        .system_tray(tray)
        .on_system_tray_event(|app, event| {
            tauri_plugin_positioner::on_tray_event(app, &event);

            #[allow(clippy::single_match)]
            match event {
                SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
                    "quit" => {
                        std::process::exit(0);
                    }
                    "configuration" => {
                        log::info!("Got configuration event");
                        if let Err(err) = WindowBuilder::new(
                            app,
                            "config-window".to_string(),
                            tauri::WindowUrl::App("configuration.html".into()),
                        )
                        .build()
                        {
                            log::error!("Cannot open configuration window: {:?}", err);
                        };
                    }
                    _ => {}
                },
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_config_schema,
            load_config,
            save_config,
        ])
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                api.prevent_exit();
            }
        });

    Ok(())
}
