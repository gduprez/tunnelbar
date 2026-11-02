//! Pure decisions behind the menu bar icon, kept free of Tauri types so they can be unit tested.

use std::time::{Duration, Instant};

use tunnelbar_core::state::TunnelState;
use tunnelbar_core::supervisor::TunnelStatus;

use crate::i18n::Translator;

const TOGGLE_PREFIX: &str = "toggle:";

/// Actions offered by the tray icon's menu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuAction {
    /// Starts the tunnel with this id if it is stopped or failed, stops it otherwise.
    Toggle(String),
    OpenPanel,
    EditConfig,
    Quit,
}

impl MenuAction {
    pub fn id(&self) -> String {
        match self {
            MenuAction::Toggle(id) => format!("{TOGGLE_PREFIX}{id}"),
            MenuAction::OpenPanel => "open-panel".into(),
            MenuAction::EditConfig => "edit-config".into(),
            MenuAction::Quit => "quit".into(),
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "open-panel" => Some(MenuAction::OpenPanel),
            "edit-config" => Some(MenuAction::EditConfig),
            "quit" => Some(MenuAction::Quit),
            _ => id
                .strip_prefix(TOGGLE_PREFIX)
                .filter(|tunnel| !tunnel.is_empty())
                .map(|tunnel| MenuAction::Toggle(tunnel.to_owned())),
        }
    }
}

/// Look of the menu bar icon, summarising every tunnel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayLook {
    /// No tunnel running.
    Idle,
    /// Tunnels running, none in trouble.
    Active,
    /// At least one tunnel failed or is reconnecting.
    Problem,
}

pub fn tray_look(statuses: &[TunnelStatus]) -> TrayLook {
    let states = || statuses.iter().map(|status| &status.state);
    if states().any(|state| {
        matches!(
            state,
            TunnelState::Failed { .. } | TunnelState::Reconnecting { .. }
        )
    }) {
        TrayLook::Problem
    } else if states().any(is_running) {
        TrayLook::Active
    } else {
        TrayLook::Idle
    }
}

/// Whether a tunnel counts as running: its menu item is checked and a click stops it.
pub fn is_running(state: &TunnelState) -> bool {
    matches!(
        state,
        TunnelState::Connecting { .. } | TunnelState::Up | TunnelState::Reconnecting { .. }
    )
}

/// Text of a tunnel's menu item.
pub fn menu_label(name: &str, state: &TunnelState, t: &Translator) -> String {
    match state {
        TunnelState::Stopped | TunnelState::Up => name.to_owned(),
        TunnelState::Connecting { .. } => t.t("tray.connecting", &[("name", name)]),
        TunnelState::Reconnecting { delay, .. } => t.t(
            "tray.retrying",
            &[("name", name), ("seconds", &delay.as_secs().to_string())],
        ),
        TunnelState::Failed { .. } => t.t("tray.failed", &[("name", name)]),
    }
}

/// A click on the icon hiding the panel first makes it lose focus, which hides it already;
/// a hide this recent is that same click, which must not show the panel again.
pub const CLICK_HIDE_WINDOW: Duration = Duration::from_millis(400);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelAction {
    Show,
    Hide,
    /// The panel was just hidden by this very click.
    Nothing,
}

/// Makes a click on the icon toggle the panel.
#[derive(Debug, Default)]
pub struct PanelToggle {
    hidden_on_blur_at: Option<Instant>,
}

impl PanelToggle {
    /// The panel lost focus and was hidden.
    pub fn hidden_on_blur(&mut self, now: Instant) {
        self.hidden_on_blur_at = Some(now);
    }

    pub fn on_icon_click(&mut self, now: Instant, visible: bool) -> PanelAction {
        let just_hidden = self
            .hidden_on_blur_at
            .take()
            .is_some_and(|hidden| now.saturating_duration_since(hidden) < CLICK_HIDE_WINDOW);
        if visible {
            PanelAction::Hide
        } else if just_hidden {
            PanelAction::Nothing
        } else {
            PanelAction::Show
        }
    }
}

/// Width of the panel, in logical pixels.
pub const PANEL_WIDTH: f64 = 380.0;
/// Bounds of the panel height: it follows its content between them.
pub const PANEL_MIN_HEIGHT: f64 = 160.0;
pub const PANEL_MAX_HEIGHT: f64 = 640.0;

/// The panel height for a content height requested by the webview, or None if it is not a number.
pub fn panel_height(requested: f64) -> Option<f64> {
    requested
        .is_finite()
        .then(|| requested.ceil().clamp(PANEL_MIN_HEIGHT, PANEL_MAX_HEIGHT))
}

/// A rectangle in physical pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// Top-left corner placing the panel centred right below the tray icon, kept inside the screen.
pub fn panel_origin(icon: Rect, panel_width: f64, screen: Rect) -> (f64, f64) {
    let centred = icon.x + icon.width / 2.0 - panel_width / 2.0;
    let max_x = screen.x + screen.width - panel_width;
    (centred.min(max_x).max(screen.x), icon.y + icon.height)
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::time::Duration;
    use tunnelbar_core::error::TunnelError;

    fn status(state: TunnelState) -> TunnelStatus {
        TunnelStatus {
            id: "prod".into(),
            state,
            up_since: None,
            reconnections: 0,
            logs: vec![],
        }
    }

    fn reconnecting(seconds: u64) -> TunnelState {
        TunnelState::Reconnecting {
            attempt: 1,
            delay: Duration::from_secs(seconds),
            last_error: TunnelError::ConnectionLost,
        }
    }

    fn failed() -> TunnelState {
        TunnelState::Failed {
            error: TunnelError::AuthFailed,
        }
    }

    #[test]
    fn menu_action_ids_round_trip() {
        for action in [
            MenuAction::Toggle("pg-prod_2".into()),
            MenuAction::OpenPanel,
            MenuAction::EditConfig,
            MenuAction::Quit,
        ] {
            assert_eq!(MenuAction::from_id(&action.id()), Some(action));
        }
    }

    #[test]
    fn unknown_menu_id_is_ignored() {
        assert_eq!(MenuAction::from_id("something"), None);
        assert_eq!(MenuAction::from_id("toggle:"), None);
    }

    #[test]
    fn icon_is_idle_without_running_tunnel() {
        assert_eq!(tray_look(&[]), TrayLook::Idle);
        assert_eq!(tray_look(&[status(TunnelState::Stopped)]), TrayLook::Idle);
    }

    #[test]
    fn icon_is_active_while_tunnels_run_fine() {
        let statuses = [
            status(TunnelState::Up),
            status(TunnelState::Connecting { attempt: 0 }),
            status(TunnelState::Stopped),
        ];
        assert_eq!(tray_look(&statuses), TrayLook::Active);
    }

    #[test]
    fn icon_shows_a_problem_when_any_tunnel_is_in_trouble() {
        assert_eq!(
            tray_look(&[status(TunnelState::Up), status(reconnecting(4))]),
            TrayLook::Problem
        );
        assert_eq!(tray_look(&[status(failed())]), TrayLook::Problem);
    }

    #[test]
    fn running_states_are_checked() {
        assert!(is_running(&TunnelState::Up));
        assert!(is_running(&TunnelState::Connecting { attempt: 0 }));
        assert!(is_running(&reconnecting(1)));
        assert!(!is_running(&TunnelState::Stopped));
        assert!(!is_running(&failed()));
    }

    #[test]
    fn labels_describe_transient_and_failed_states() {
        let t = Translator::new("en");
        assert_eq!(menu_label("Prod", &TunnelState::Up, &t), "Prod");
        assert_eq!(menu_label("Prod", &TunnelState::Stopped, &t), "Prod");
        assert_eq!(
            menu_label("Prod", &TunnelState::Connecting { attempt: 0 }, &t),
            "Prod — connecting…"
        );
        assert_eq!(
            menu_label("Prod", &reconnecting(8), &t),
            "Prod — retrying in 8 s"
        );
        assert_eq!(menu_label("Prod", &failed(), &t), "Prod — error");
    }

    #[test]
    fn a_click_shows_a_hidden_panel() {
        let mut toggle = PanelToggle::default();
        assert_eq!(
            toggle.on_icon_click(Instant::now(), false),
            PanelAction::Show
        );
    }

    #[test]
    fn a_click_hides_a_visible_panel() {
        let mut toggle = PanelToggle::default();
        assert_eq!(
            toggle.on_icon_click(Instant::now(), true),
            PanelAction::Hide
        );
    }

    #[test]
    fn a_click_that_just_blurred_the_panel_leaves_it_hidden() {
        let mut toggle = PanelToggle::default();
        let now = Instant::now();
        toggle.hidden_on_blur(now);
        assert_eq!(
            toggle.on_icon_click(now + Duration::from_millis(50), false),
            PanelAction::Nothing
        );
        // The next click shows it again.
        assert_eq!(
            toggle.on_icon_click(now + Duration::from_millis(100), false),
            PanelAction::Show
        );
    }

    #[test]
    fn an_old_blur_does_not_prevent_showing() {
        let mut toggle = PanelToggle::default();
        let now = Instant::now();
        toggle.hidden_on_blur(now);
        assert_eq!(
            toggle.on_icon_click(now + Duration::from_secs(5), false),
            PanelAction::Show
        );
    }

    #[test]
    fn panel_height_follows_the_content_within_bounds() {
        assert_eq!(panel_height(300.4), Some(301.0));
        assert_eq!(panel_height(10.0), Some(PANEL_MIN_HEIGHT));
        assert_eq!(panel_height(5000.0), Some(PANEL_MAX_HEIGHT));
    }

    #[test]
    fn panel_height_rejects_values_that_are_not_numbers() {
        assert_eq!(panel_height(f64::NAN), None);
        assert_eq!(panel_height(f64::INFINITY), None);
        assert_eq!(panel_height(f64::NEG_INFINITY), None);
    }

    const SCREEN: Rect = Rect {
        x: 0.0,
        y: 0.0,
        width: 3000.0,
        height: 2000.0,
    };

    #[test]
    fn panel_is_centred_below_the_icon() {
        let icon = Rect {
            x: 1000.0,
            y: 0.0,
            width: 40.0,
            height: 48.0,
        };
        assert_eq!(panel_origin(icon, 720.0, SCREEN), (660.0, 48.0));
    }

    #[test]
    fn panel_does_not_overflow_the_right_edge() {
        let icon = Rect {
            x: 2950.0,
            y: 0.0,
            width: 40.0,
            height: 48.0,
        };
        assert_eq!(panel_origin(icon, 720.0, SCREEN), (2280.0, 48.0));
    }

    #[test]
    fn panel_does_not_overflow_the_left_edge_of_a_secondary_screen() {
        let screen = Rect {
            x: -1920.0,
            ..SCREEN
        };
        let icon = Rect {
            x: -1910.0,
            y: 0.0,
            width: 40.0,
            height: 48.0,
        };
        assert_eq!(panel_origin(icon, 720.0, screen), (-1920.0, 48.0));
    }
}
