mod commands;
mod config_view;
// Used by the Linux launch at login; compiled and tested on every system.
#[cfg_attr(not(target_os = "linux"), allow(dead_code))]
mod desktop_entry;
mod i18n;
mod login_item;
mod notify;
mod settings;
mod tray;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime};

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, RunEvent, WebviewWindow, Wry};
use tauri_plugin_log::{RotationStrategy, Target, TargetKind};
use tauri_plugin_notification::NotificationExt;
use tunnelbar_core::error::TunnelError;
use tunnelbar_core::manager::TunnelManager;
use tunnelbar_core::orphans::{PidRegistry, SSH_SIGNATURE, SystemInspector, clean_orphans};
use tunnelbar_core::paths::AppPaths;
use tunnelbar_core::process::{SystemSpawner, TcpPortProber};
use tunnelbar_core::state::TunnelState;
use tunnelbar_core::store::{ConfigError, ConfigStore, FileConfigStore};
use tunnelbar_core::supervisor::TunnelStatus;
use tunnelbar_core::system_events::{NetworkWatcher, WakeDetector};

use i18n::{Translator, available_locales, detect_locale};
use settings::Settings;
use tray::{MenuAction, PanelAction, PanelToggle, Rect, TrayLook};

pub(crate) const PANEL: &str = "panel";
const TRAY: &str = "main";

pub type SystemManager = TunnelManager<SystemSpawner, TcpPortProber>;

/// Shared by the tray, the Tauri commands and the status forwarding task.
pub struct AppState {
    pub manager: tokio::sync::Mutex<SystemManager>,
    pub store: FileConfigStore,
    /// Why the configuration could not be loaded, if it could not.
    pub config_error: Mutex<Option<ConfigError>>,
    pub translator: Mutex<Translator>,
    pub settings: Mutex<Settings>,
    pub settings_path: PathBuf,
    /// Preferred locale of macOS, used when the language setting is `auto`.
    pub system_locale: String,
    /// Modification time of the tunnels file when TunnelBar last read or wrote it.
    pub config_seen: Mutex<Option<SystemTime>>,
    panel_toggle: Mutex<PanelToggle>,
    /// Tunnel names and states the tray menu was last built from.
    menu_shown: Mutex<Option<Vec<(String, TunnelState)>>>,
}

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                .targets([
                    Target::new(TargetKind::Stdout),
                    // ~/Library/Logs/io.github.gduprez.tunnelbar/tunnelbar.log
                    Target::new(TargetKind::LogDir {
                        file_name: Some("tunnelbar".into()),
                    }),
                ])
                .max_file_size(1024 * 1024)
                .rotation_strategy(RotationStrategy::KeepSome(5))
                .build(),
        )
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            commands::list_tunnels,
            commands::start_tunnel,
            commands::stop_tunnel,
            commands::shell_command,
            commands::get_config_info,
            commands::validate_tunnel,
            commands::save_tunnel,
            commands::delete_tunnel,
            commands::open_config,
            commands::get_settings,
            commands::set_language,
            commands::get_login_item,
            commands::set_login_item,
            commands::fit_panel,
        ])
        .setup(|app| {
            // Menu bar only: no Dock icon, not listed in Cmd+Tab.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            let state = load_state(app.handle())?;
            app.manage(state);
            build_tray(app.handle())?;
            forward_statuses(app.handle().clone());
            watch_config_file(app.handle().clone());
            watch_system_events(app.handle().clone());
            Ok(())
        })
        .on_window_event(|window, event| {
            // The panel behaves like a popover: it closes when it loses focus.
            if window.label() == PANEL && matches!(event, tauri::WindowEvent::Focused(false)) {
                let _ = window.hide();
                window
                    .state::<AppState>()
                    .panel_toggle
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .hidden_on_blur(Instant::now());
            }
        })
        .build(tauri::generate_context!())
        .expect("error while building TunnelBar");

    app.run(|app, event| {
        if let RunEvent::Exit = event {
            // Quitting TunnelBar closes every tunnel.
            let state = app.state::<AppState>();
            tauri::async_runtime::block_on(async { state.manager.lock().await.shutdown().await });
        }
    });
}

fn load_state(app: &AppHandle) -> tauri::Result<AppState> {
    let paths = AppPaths::current(&app.path().home_dir()?);
    let store = FileConfigStore::new(paths.tunnels.clone());
    let locales = sys_locale::get_locales().collect::<Vec<_>>();
    let system_locale = detect_locale(locales.iter().map(String::as_str), &available_locales());
    let settings_path = paths.settings.clone();
    let settings = settings::load(&settings_path);
    let locale = settings::resolve_locale(&settings, &system_locale, &available_locales());

    // ssh processes of a TunnelBar that crashed would otherwise hold their ports forever.
    let registry = PidRegistry::new(paths.pids.clone());
    let killed = clean_orphans(&registry, &SystemInspector, SSH_SIGNATURE);
    if !killed.is_empty() {
        log::warn!("stopped orphaned ssh processes: {killed:?}");
    }
    let spawner = SystemSpawner::new().with_registry(Arc::new(registry));

    let mut manager = TunnelManager::new(spawner, TcpPortProber);
    let config_error = tauri::async_runtime::block_on(async {
        match store.load() {
            Ok(config) => {
                manager.apply_config(&config).await;
                manager.autostart();
                None
            }
            Err(error) => Some(error),
        }
    });
    let state = AppState {
        manager: tokio::sync::Mutex::new(manager),
        store,
        config_error: Mutex::new(config_error),
        translator: Mutex::new(Translator::new(&locale)),
        settings: Mutex::new(settings),
        settings_path,
        system_locale,
        config_seen: Mutex::new(None),
        panel_toggle: Mutex::new(PanelToggle::default()),
        menu_shown: Mutex::new(None),
    };
    state.remember_config_file();
    Ok(state)
}

/// Hand edits of the tunnels file are picked up within two seconds.
fn watch_config_file(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(2)).await;
            commands::reload_if_changed(&app).await;
        }
    });
}

/// Sends every status change to the UI and keeps the tray up to date.
fn forward_statuses(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut updates = app.state::<AppState>().manager.lock().await.subscribe();
        let mut previous: HashMap<String, TunnelState> = HashMap::new();
        loop {
            match updates.recv().await {
                Ok(status) => {
                    let _ = app.emit("tunnel-status", &status);
                    let before = previous.insert(status.id.clone(), status.state.clone());
                    if let Some(error) = notify::newly_failed(before.as_ref(), &status.state) {
                        notify_failure(&app, &status.id, error).await;
                    }
                    refresh_tray(&app).await;
                }
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    refresh_tray(&app).await
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return,
            }
        }
    });
}

async fn notify_failure(app: &AppHandle, id: &str, error: &TunnelError) {
    let state = app.state::<AppState>();
    let name = state
        .manager
        .lock()
        .await
        .tunnels()
        .into_iter()
        .find(|tunnel| tunnel.id == id)
        .map_or_else(|| id.to_owned(), |tunnel| tunnel.name);
    let body = notify::error_message(
        &state.translator.lock().unwrap_or_else(|e| e.into_inner()),
        error,
    );
    let shown = app.notification().builder().title(name).body(body).show();
    if let Err(error) = shown {
        log::warn!("cannot show a notification: {error}");
    }
}

/// After a wake-up or a network change, pending reconnections are attempted at once.
fn watch_system_events(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut wake = WakeDetector::new(SystemTime::now(), std::time::Instant::now());
        let mut network = NetworkWatcher::default();
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let woke_up = wake.woke_up(SystemTime::now(), std::time::Instant::now());
            let addresses = if_addrs::get_if_addrs()
                .unwrap_or_default()
                .into_iter()
                .filter(|interface| !interface.is_loopback())
                .map(|interface| interface.ip());
            let network_changed = network.changed(addresses);
            if woke_up || network_changed {
                app.state::<AppState>()
                    .manager
                    .lock()
                    .await
                    .network_changed();
            }
        }
    });
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    TrayIconBuilder::with_id(TRAY)
        .icon(look_icon(TrayLook::Idle)?)
        .icon_as_template(true)
        .tooltip(format!("TunnelBar {}", tunnelbar_core::version()))
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if let Some(action) = MenuAction::from_id(event.id().as_ref()) {
                on_menu_action(app, action);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                rect,
                ..
            } = event
            {
                toggle_panel(tray.app_handle(), icon_rect(tray, rect));
            }
        })
        .build(app)?;
    let app = app.clone();
    tauri::async_runtime::spawn(async move { refresh_tray(&app).await });
    Ok(())
}

fn on_menu_action(app: &AppHandle, action: MenuAction) {
    match action {
        MenuAction::Toggle(id) => {
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                let _ = commands::toggle(&app.state::<AppState>(), &id).await;
            });
        }
        MenuAction::OpenPanel => show_panel(app, None),
        MenuAction::EditConfig => commands::open_config_file(&app.state::<AppState>()),
        MenuAction::Quit => app.exit(0),
    }
}

fn look_icon(look: TrayLook) -> tauri::Result<Image<'static>> {
    Image::from_bytes(match look {
        TrayLook::Idle => include_bytes!("../icons/tray/idle.png"),
        TrayLook::Active => include_bytes!("../icons/tray/active.png"),
        TrayLook::Problem => include_bytes!("../icons/tray/problem.png"),
    })
}

/// Rebuilds the tray menu and icon when a tunnel's state (not just its logs) changed.
pub async fn refresh_tray(app: &AppHandle) {
    let state = app.state::<AppState>();
    let (tunnels, statuses) = {
        let manager = state.manager.lock().await;
        (manager.tunnels(), manager.statuses())
    };
    let shown: Vec<(String, TunnelState)> = tunnels
        .iter()
        .zip(&statuses)
        .map(|(tunnel, status)| (tunnel.name.clone(), status.state.clone()))
        .collect();
    {
        let mut last = state.menu_shown.lock().unwrap_or_else(|e| e.into_inner());
        if last.as_ref() == Some(&shown) {
            return;
        }
        *last = Some(shown.clone());
    }
    let Some(tray) = app.tray_by_id(TRAY) else {
        return;
    };
    if let Err(error) = update_tray(app, &tray, &state, &shown, &statuses) {
        log::error!("cannot update the tray: {error}");
    }
}

/// Forces the next refresh to rebuild the menu, e.g. after a configuration change.
pub fn invalidate_tray(state: &AppState) {
    *state.menu_shown.lock().unwrap_or_else(|e| e.into_inner()) = None;
}

fn update_tray(
    app: &AppHandle,
    tray: &TrayIcon,
    state: &AppState,
    shown: &[(String, TunnelState)],
    statuses: &[TunnelStatus],
) -> tauri::Result<()> {
    let t = &*state.translator.lock().unwrap_or_else(|e| e.into_inner());
    let mut items: Vec<Box<dyn IsMenuItem<Wry>>> = Vec::new();
    for ((name, tunnel_state), status) in shown.iter().zip(statuses) {
        items.push(Box::new(CheckMenuItem::with_id(
            app,
            MenuAction::Toggle(status.id.clone()).id(),
            tray::menu_label(name, tunnel_state, t),
            true,
            tray::is_running(tunnel_state),
            None::<&str>,
        )?));
    }
    let has_config_error = state
        .config_error
        .lock()
        .map(|error| error.is_some())
        .unwrap_or(false);
    if has_config_error {
        let text = t.t("tray.config_error", &[]);
        items.push(Box::new(MenuItem::new(app, text, false, None::<&str>)?));
    } else if shown.is_empty() {
        let text = t.t("tray.no_tunnel", &[]);
        items.push(Box::new(MenuItem::new(app, text, false, None::<&str>)?));
    }
    items.push(Box::new(PredefinedMenuItem::separator(app)?));
    for (action, key) in [
        (MenuAction::OpenPanel, "tray.open_panel"),
        (MenuAction::EditConfig, "tray.edit_config"),
    ] {
        let text = t.t(key, &[]);
        items.push(Box::new(MenuItem::with_id(
            app,
            action.id(),
            text,
            true,
            None::<&str>,
        )?));
    }
    items.push(Box::new(PredefinedMenuItem::separator(app)?));
    let quit = t.t("tray.quit", &[]);
    items.push(Box::new(MenuItem::with_id(
        app,
        MenuAction::Quit.id(),
        quit,
        true,
        None::<&str>,
    )?));

    let refs: Vec<&dyn IsMenuItem<Wry>> = items.iter().map(|item| item.as_ref()).collect();
    tray.set_menu(Some(Menu::with_items(app, &refs)?))?;
    tray.set_icon(Some(look_icon(tray::tray_look(statuses))?))?;
    tray.set_icon_as_template(true)
}

fn icon_rect(tray: &TrayIcon, rect: tauri::Rect) -> Rect {
    let scale = tray
        .app_handle()
        .get_webview_window(PANEL)
        .and_then(|panel| panel.scale_factor().ok())
        .unwrap_or(1.0);
    let position = rect.position.to_physical::<f64>(scale);
    let size = rect.size.to_physical::<f64>(scale);
    Rect {
        x: position.x,
        y: position.y,
        width: size.width,
        height: size.height,
    }
}

/// A click on the icon shows the panel, or hides it when it is open.
fn toggle_panel(app: &AppHandle, icon: Rect) {
    let Some(panel) = app.get_webview_window(PANEL) else {
        return;
    };
    let visible = panel.is_visible().unwrap_or(false);
    let action = app
        .state::<AppState>()
        .panel_toggle
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .on_icon_click(Instant::now(), visible);
    match action {
        PanelAction::Show => show_panel(app, Some(icon)),
        PanelAction::Hide => {
            let _ = panel.hide();
        }
        PanelAction::Nothing => {}
    }
}

/// Shows the panel, right below the tray icon when its position is known.
fn show_panel(app: &AppHandle, icon: Option<Rect>) {
    let Some(panel) = app.get_webview_window(PANEL) else {
        return;
    };
    match icon {
        Some(icon) => {
            if let Err(err) = place_below(&panel, icon) {
                log::warn!("cannot position the panel: {err}");
            }
        }
        None => {
            let _ = panel.center();
        }
    }
    let _ = panel.show();
    let _ = panel.set_focus();
}

fn place_below(panel: &WebviewWindow, icon: Rect) -> tauri::Result<()> {
    let panel_width = f64::from(panel.outer_size()?.width);
    // Without a known monitor, the icon itself is the only bound: the panel is just centred on it.
    let screen = match panel.monitor_from_point(icon.x, icon.y)? {
        Some(monitor) => Rect {
            x: f64::from(monitor.position().x),
            y: f64::from(monitor.position().y),
            width: f64::from(monitor.size().width),
            height: f64::from(monitor.size().height),
        },
        None => Rect {
            x: icon.x + icon.width / 2.0 - panel_width / 2.0,
            width: panel_width,
            ..icon
        },
    };
    let (x, y) = tray::panel_origin(icon, panel_width, screen);
    panel.set_position(PhysicalPosition::new(x, y))
}
