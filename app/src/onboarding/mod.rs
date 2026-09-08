//! Lightweight replacement for the former standalone `crates/onboarding` crate.
//!
//! We keep only the minimal types and UI pieces that the main app depends on.
//! The original upstream `onboarding` crate included full-screen onboarding slides
//! (AgentOnboardingView, visuals, telemetry, etc.). For the OSS AppImage branch we
//! want to reduce compile time by removing that heavy crate.

pub mod callout;
pub mod components;
pub mod slides;

pub use callout::{
    FinalState, OnboardingCalloutView, OnboardingCalloutViewEvent, OnboardingKeybindings,
    OnboardingQuery,
};
pub use slides::{
    AgentAutonomy, AgentDevelopmentSettings, OnboardingModelInfo, ProjectOnboardingSettings,
};

/// The user's intention selected during onboarding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnboardingIntention {
    Terminal,
    AgentDrivenDevelopment,
}

impl std::fmt::Display for OnboardingIntention {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OnboardingIntention::AgentDrivenDevelopment => write!(f, "agent_driven"),
            OnboardingIntention::Terminal => write!(f, "terminal"),
        }
    }
}

/// The default mode for new sessions, chosen during onboarding.
/// Mapped to `DefaultSessionMode` at the application boundary.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SessionDefault {
    #[default]
    Agent,
    Terminal,
}

impl std::fmt::Display for SessionDefault {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SessionDefault::Agent => write!(f, "agent"),
            SessionDefault::Terminal => write!(f, "terminal"),
        }
    }
}

/// User-facing names of the AI features enabled when the agent intention is selected.
/// Shared by the login slide's skip-login confirmation dialog.
pub const AI_FEATURES: &[&str] = &[
    "Warp agents",
    "Oz cloud agents platform",
    "Next command predictions",
    "Prompt suggestions",
    "Codebase context",
    "Remote control with Claude Code, Codex, and other agents",
    "Agents over SSH",
];

/// User-facing names of the Warp Drive features enabled when the terminal
/// intention is selected with Warp Drive turned on.
pub const WARP_DRIVE_FEATURES: &[&str] = &["Warp Drive", "Session Sharing"];

/// UI customization settings chosen during the "Customize your UI" onboarding step.
#[derive(Clone, Debug)]
pub struct UICustomizationSettings {
    pub use_vertical_tabs: bool,
    pub show_conversation_history: bool,
    pub show_project_explorer: bool,
    pub show_global_search: bool,
    pub show_warp_drive: bool,
    pub show_code_review_button: bool,
}

/// Auth/billing state of the user for gating onboarding choices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OnboardingAuthState {
    LoggedOut,
    FreeUser,
    PayingUser,
}

/// Settings selected by the user during onboarding.
#[derive(Clone, Debug)]
pub enum SelectedSettings {
    Terminal {
        ui_customization: Option<UICustomizationSettings>,
        cli_agent_toolbar_enabled: bool,
        show_agent_notifications: bool,
    },
    AgentDrivenDevelopment {
        agent_settings: slides::AgentDevelopmentSettings,
        project_settings: slides::ProjectOnboardingSettings,
        ui_customization: Option<UICustomizationSettings>,
    },
}

impl SelectedSettings {
    pub fn is_ai_enabled(&self) -> bool {
        match self {
            SelectedSettings::AgentDrivenDevelopment { agent_settings, .. } => {
                !agent_settings.disable_oz
            }
            SelectedSettings::Terminal { .. } => false,
        }
    }

    pub fn is_warp_drive_enabled(&self) -> bool {
        match self {
            SelectedSettings::AgentDrivenDevelopment {
                ui_customization, ..
            } => ui_customization
                .as_ref()
                .map(|ui| ui.show_warp_drive)
                .unwrap_or(true),
            SelectedSettings::Terminal {
                ui_customization, ..
            } => ui_customization
                .as_ref()
                .map(|ui| ui.show_warp_drive)
                .unwrap_or(false),
        }
    }
}

/// No-op init hook kept for compatibility with `onboarding::init(ctx)`.
///
/// The original `onboarding` crate registered onboarding-related views and
/// callout keybindings here. We keep only the callout init.
pub fn init(app: &mut warpui::AppContext) {
    callout::init(app);
}
