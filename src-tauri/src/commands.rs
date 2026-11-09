//! Tauri commands called by the panel, and the actions they share with the tray menu.

use std::ffi::OsString;
use std::path::Path;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};
use tunnelbar_core::command::SshCommand;
use tunnelbar_core::manager::{ManagerError, TunnelManager};
use tunnelbar_core::model::{CONFIG_VERSION, Config, Tunnel};
use tunnelbar_core::paths::Os;
use tunnelbar_core::process::{PortProber, ProcessSpawner};
use tunnelbar_core::store::{ConfigError, ConfigStore};
use tunnelbar_core::supervisor::TunnelStatus;
use tunnelbar_core::validation::{ValidationError, ValidationErrorKind, validate};

use crate::config_view::ConfigProblem;
use crate::i18n::{Translator, available_locales};
use crate::login_item::{self, LoginItem};
use crate::settings::{self, Settings};
use crate::tray::{PANEL_WIDTH, is_running, panel_height};
use crate::{AppState, invalidate_tray, refresh_tray};

/// A tunnel definition with its live status, as sent to the panel.
#[derive(Debug, Clone, Serialize)]
pub struct TunnelView {
    #[serde(flatten)]
    pub tunnel: Tunnel,
    pub status: TunnelStatus,
}

#[tauri::command]
pub async fn list_tunnels(state: State<'_, AppState>) -> Result<Vec<TunnelView>, String> {
    let manager = state.manager.lock().await;
    Ok(manager
        .tunnels()
        .into_iter()
        .zip(manager.statuses())
        .map(|(tunnel, status)| TunnelView { tunnel, status })
        .collect())
}

#[tauri::command]
pub async fn start_tunnel(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .manager
        .lock()
        .await
        .start(&id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn stop_tunnel(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .manager
        .lock()
        .await
        .stop(&id)
        .map_err(|error| error.to_string())
}

/// The equivalent `ssh` command line, to copy into a terminal.
#[tauri::command]
pub async fn shell_command(state: State<'_, AppState>, id: String) -> Result<String, String> {
    let manager = state.manager.lock().await;
    let tunnel = manager
        .tunnels()
        .into_iter()
        .find(|tunnel| tunnel.id == id)
        .ok_or_else(|| ManagerError::UnknownTunnel(id).to_string())?;
    Ok(SshCommand::for_tunnel(&tunnel).to_shell_line())
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigInfo {
    pub path: String,
    pub problem: Option<ConfigProblem>,
}

#[tauri::command]
pub fn get_config_info(state: State<'_, AppState>) -> ConfigInfo {
    ConfigInfo {
        path: state.store.path().display().to_string(),
        problem: state.config_error().as_ref().map(ConfigProblem::from),
    }
}

/// Problems of `tunnel` if it were saved in place of `previous_id`, for live form feedback.
#[tauri::command]
pub async fn validate_tunnel(
    state: State<'_, AppState>,
    tunnel: Tunnel,
    previous_id: Option<String>,
) -> Result<Vec<ValidationError>, ()> {
    let current = state.current_config().await;
    Ok(tunnel_errors(&current, tunnel, previous_id.as_deref()))
}

#[tauri::command]
pub async fn save_tunnel(
    app: AppHandle,
    state: State<'_, AppState>,
    tunnel: Tunnel,
    previous_id: Option<String>,
) -> Result<(), ConfigProblem> {
    let current = editable_config(&state).await?;
    write_config(
        &app,
        &state,
        current.with_tunnel(tunnel, previous_id.as_deref()),
    )
    .await
}

#[tauri::command]
pub async fn delete_tunnel(
    app: AppHandle,
    state: State<'_, AppState>,
    id: String,
) -> Result<(), ConfigProblem> {
    let current = editable_config(&state).await?;
    write_config(&app, &state, current.without_tunnel(&id)).await
}

#[tauri::command]
pub fn open_config(state: State<'_, AppState>) {
    open_config_file(&state);
}

#[derive(Debug, Clone, Serialize)]
pub struct SettingsView {
    /// `"auto"` or a locale code.
    pub language: String,
    /// The locale actually displayed.
    pub locale: String,
    pub available: Vec<String>,
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> SettingsView {
    state.settings_view()
}

#[tauri::command]
pub async fn set_language(
    app: AppHandle,
    state: State<'_, AppState>,
    language: String,
) -> Result<SettingsView, String> {
    let settings = Settings { language };
    settings::save(&state.settings_path, &settings).map_err(|error| error.to_string())?;
    *state.settings.lock().unwrap_or_else(|e| e.into_inner()) = settings;
    let view = state.settings_view();
    *state.translator.lock().unwrap_or_else(|e| e.into_inner()) = Translator::new(&view.locale);
    invalidate_tray(&state);
    refresh_tray(&app).await;
    Ok(view)
}

/// Sizes the panel to its content. The height is bounded here, whatever the webview sends.
#[tauri::command]
pub fn fit_panel(window: tauri::WebviewWindow, height: f64) -> Result<(), String> {
    if window.label() != crate::PANEL {
        return Err("only the panel can be resized".into());
    }
    let height = panel_height(height).ok_or("invalid height")?;
    window
        .set_size(tauri::LogicalSize::new(PANEL_WIDTH, height))
        .map_err(|error| error.to_string())
}

#[derive(Debug, Clone, Serialize)]
pub struct PlatformView {
    pub os: &'static str,
    /// Whether the desktop shows tray icons; None when unknown.
    pub tray_available: Option<bool>,
}

#[tauri::command]
pub fn get_platform(state: State<'_, AppState>) -> PlatformView {
    PlatformView {
        os: match Os::current() {
            Os::MacOs => "macos",
            Os::Linux => "linux",
        },
        tray_available: state.tray_available,
    }
}

#[tauri::command]
pub fn get_login_item() -> LoginItem {
    login_item::status()
}

#[tauri::command]
pub fn set_login_item(enabled: bool) -> Result<LoginItem, String> {
    login_item::set(enabled)
}

/// Errors of the configuration where `tunnel` replaces `previous_id`, that concern `tunnel`.
pub fn tunnel_errors(
    current: &Config,
    tunnel: Tunnel,
    previous_id: Option<&str>,
) -> Vec<ValidationError> {
    let id = tunnel.id.clone();
    let candidate = current.with_tunnel(tunnel, previous_id);
    let index = candidate
        .tunnels
        .iter()
        .rposition(|t| t.id == id)
        .expect("the tunnel was just inserted");
    validate(&candidate)
        .err()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|error| match error.kind {
            _ if error.tunnel == Some(index) => Some(error),
            // A port clash is reported on its second occurrence, which may be another
            // tunnel: turn it around so that the edited tunnel shows it.
            ValidationErrorKind::LocalPortConflict {
                port,
                tunnel,
                forward,
            } if tunnel == index => Some(ValidationError {
                tunnel: Some(index),
                forward: Some(forward),
                kind: ValidationErrorKind::LocalPortConflict {
                    port,
                    tunnel: error.tunnel?,
                    forward: error.forward?,
                },
            }),
            _ => None,
        })
        .collect()
}

/// The configuration the panel may modify: never a broken file, which only a human can fix.
async fn editable_config(state: &AppState) -> Result<Config, ConfigProblem> {
    if state.config_error().is_some() {
        return Err(ConfigProblem::FileBroken);
    }
    Ok(state.current_config().await)
}

async fn write_config(
    app: &AppHandle,
    state: &AppState,
    config: Config,
) -> Result<(), ConfigProblem> {
    state
        .store
        .save(&config)
        .map_err(|error| ConfigProblem::from(&error))?;
    state.remember_config_file();
    adopt(app, state, Ok(config)).await;
    Ok(())
}

/// Reloads the file if it changed since TunnelBar last read or wrote it.
pub async fn reload_if_changed(app: &AppHandle) {
    let state = app.state::<AppState>();
    if state.config_file_changed() {
        state.remember_config_file();
        adopt(app, &state, state.store.load()).await;
    }
}

/// Runs a loaded configuration (or records why it could not be loaded) and tells the UI.
async fn adopt(app: &AppHandle, state: &AppState, loaded: Result<Config, ConfigError>) {
    match loaded {
        Ok(config) => {
            state.manager.lock().await.apply_config(&config).await;
            state.set_config_error(None);
        }
        Err(error) => state.set_config_error(Some(error)),
    }
    invalidate_tray(state);
    refresh_tray(app).await;
    let _ = app.emit("tunnels-changed", ());
}

impl AppState {
    pub async fn current_config(&self) -> Config {
        Config {
            version: CONFIG_VERSION,
            tunnels: self.manager.lock().await.tunnels(),
        }
    }

    pub fn config_error(&self) -> std::sync::MutexGuard<'_, Option<ConfigError>> {
        self.config_error.lock().unwrap_or_else(|e| e.into_inner())
    }

    fn set_config_error(&self, error: Option<ConfigError>) {
        *self.config_error() = error;
    }

    pub fn settings_view(&self) -> SettingsView {
        let settings = self
            .settings
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        let available = available_locales();
        SettingsView {
            locale: settings::resolve_locale(&settings, &self.system_locale, &available),
            language: settings.language,
            available: available.into_iter().map(str::to_owned).collect(),
        }
    }

    fn config_modified(&self) -> Option<std::time::SystemTime> {
        std::fs::metadata(self.store.path())
            .and_then(|meta| meta.modified())
            .ok()
    }

    fn config_file_changed(&self) -> bool {
        *self.config_seen.lock().unwrap_or_else(|e| e.into_inner()) != self.config_modified()
    }

    /// Records the file's current version as known, so that our own writes are not reloaded.
    pub fn remember_config_file(&self) {
        *self.config_seen.lock().unwrap_or_else(|e| e.into_inner()) = self.config_modified();
    }
}

pub async fn toggle(state: &AppState, id: &str) -> Result<(), ManagerError> {
    toggle_tunnel(&*state.manager.lock().await, id)
}

/// Stops a running tunnel, starts a stopped or failed one.
pub fn toggle_tunnel<S: ProcessSpawner, P: PortProber>(
    manager: &TunnelManager<S, P>,
    id: &str,
) -> Result<(), ManagerError> {
    let running = manager
        .statuses()
        .iter()
        .any(|status| status.id == id && is_running(&status.state));
    if running {
        manager.stop(id)
    } else {
        manager.start(id)
    }
}

/// The program and arguments opening `path` in a text editor; never through a shell.
pub fn editor_command(os: Os, path: &Path) -> (&'static str, Vec<OsString>) {
    match os {
        Os::MacOs => ("/usr/bin/open", vec!["-t".into(), path.into()]),
        Os::Linux => ("/usr/bin/xdg-open", vec![path.into()]),
    }
}

/// Opens the configuration file in the system text editor, creating an empty one first if needed.
pub fn open_config_file(state: &AppState) {
    let path = state.store.path();
    if !path.exists()
        && let Err(error) = state.store.save(&Config::default())
    {
        log::error!("cannot create {}: {error}", path.display());
        return;
    }
    let (program, args) = editor_command(Os::current(), path);
    if let Err(error) = std::process::Command::new(program).args(args).spawn() {
        log::error!("cannot open {}: {error}", path.display());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tunnelbar_core::fake::FakeNetwork;
    use tunnelbar_core::model::{CONFIG_VERSION, Forward};
    use tunnelbar_core::state::TunnelState;

    async fn manager(
        network: &FakeNetwork,
    ) -> TunnelManager<tunnelbar_core::fake::FakeSpawner, tunnelbar_core::fake::FakeProber> {
        let mut manager = TunnelManager::new(network.spawner(), network.prober());
        let tunnel = Tunnel {
            id: "prod".into(),
            name: "Prod".into(),
            host: "203.0.113.10".into(),
            user: None,
            port: None,
            autostart: false,
            auto_reconnect: true,
            accept_new_host_key: false,
            forwards: vec![Forward::Local {
                local_port: 54321,
                remote_host: "127.0.0.1".into(),
                remote_port: 5432,
            }],
        };
        manager
            .apply_config(&Config {
                version: CONFIG_VERSION,
                tunnels: vec![tunnel],
            })
            .await;
        manager
    }

    async fn settle() {
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    #[tokio::test(start_paused = true)]
    async fn toggle_starts_a_stopped_tunnel_then_stops_it() {
        let network = FakeNetwork::new();
        let manager = manager(&network).await;

        toggle_tunnel(&manager, "prod").unwrap();
        settle().await;
        assert_eq!(manager.statuses()[0].state, TunnelState::Up);

        toggle_tunnel(&manager, "prod").unwrap();
        settle().await;
        assert_eq!(manager.statuses()[0].state, TunnelState::Stopped);
        assert_eq!(network.running(), 0);
    }

    fn sample(id: &str, local_port: u16) -> Tunnel {
        Tunnel {
            id: id.into(),
            name: id.into(),
            host: "203.0.113.10".into(),
            user: None,
            port: None,
            autostart: false,
            auto_reconnect: true,
            accept_new_host_key: false,
            forwards: vec![Forward::Local {
                local_port,
                remote_host: "127.0.0.1".into(),
                remote_port: 5432,
            }],
        }
    }

    fn config_of(tunnels: Vec<Tunnel>) -> Config {
        Config {
            version: CONFIG_VERSION,
            tunnels,
        }
    }

    #[test]
    fn opens_the_configuration_with_the_system_editor() {
        let path = Path::new("/home/alice/.config/tunnelbar/tunnels.toml");
        assert_eq!(
            editor_command(Os::MacOs, path),
            ("/usr/bin/open", vec!["-t".into(), path.into()])
        );
        assert_eq!(
            editor_command(Os::Linux, path),
            ("/usr/bin/xdg-open", vec![path.into()])
        );
    }

    #[test]
    fn a_valid_new_tunnel_has_no_error() {
        let current = config_of(vec![sample("a", 1)]);
        assert_eq!(tunnel_errors(&current, sample("b", 2), None), vec![]);
    }

    #[test]
    fn reports_conflicts_with_other_tunnels_on_the_edited_one() {
        let current = config_of(vec![sample("a", 1), sample("b", 2)]);
        let errors = tunnel_errors(&current, sample("c", 1), None);
        assert_eq!(errors.len(), 1);
        assert_eq!(errors[0].tunnel, Some(2));
        assert!(matches!(
            errors[0].kind,
            tunnelbar_core::validation::ValidationErrorKind::LocalPortConflict { port: 1, .. }
        ));
    }

    #[test]
    fn editing_a_tunnel_does_not_conflict_with_its_former_self() {
        let current = config_of(vec![sample("a", 1)]);
        assert_eq!(tunnel_errors(&current, sample("a", 1), Some("a")), vec![]);
        assert_eq!(
            tunnel_errors(&current, sample("renamed", 1), Some("a")),
            vec![]
        );
    }

    #[test]
    fn a_conflict_with_a_later_tunnel_is_reported_on_the_edited_one() {
        let current = config_of(vec![sample("a", 1), sample("b", 2)]);
        let errors = tunnel_errors(&current, sample("a", 2), Some("a"));
        assert_eq!(errors.len(), 1, "{errors:?}");
        assert_eq!((errors[0].tunnel, errors[0].forward), (Some(0), Some(0)));
        assert!(matches!(
            errors[0].kind,
            tunnelbar_core::validation::ValidationErrorKind::LocalPortConflict {
                port: 2,
                tunnel: 1,
                forward: 0
            }
        ));
    }

    #[test]
    fn a_new_tunnel_reusing_an_id_is_reported() {
        let current = config_of(vec![sample("a", 1)]);
        let errors = tunnel_errors(&current, sample("a", 2), None);
        assert_eq!(errors.len(), 1);
        assert!(matches!(
            errors[0].kind,
            tunnelbar_core::validation::ValidationErrorKind::DuplicateId { .. }
        ));
    }

    #[tokio::test(start_paused = true)]
    async fn toggle_rejects_an_unknown_tunnel() {
        let manager = manager(&FakeNetwork::new()).await;
        assert_eq!(
            toggle_tunnel(&manager, "nope"),
            Err(ManagerError::UnknownTunnel("nope".into()))
        );
    }
}
