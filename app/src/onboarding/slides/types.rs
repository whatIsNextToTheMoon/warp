use ai::LLMId;
use warp_core::ui::icons::Icon;

use super::super::SessionDefault;

/// Information about a model displayed during onboarding.
#[derive(Clone, Debug)]
pub struct OnboardingModelInfo {
    pub id: LLMId,
    pub title: String,
    pub icon: Icon,
    pub requires_upgrade: bool,
    pub is_default: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum AgentAutonomy {
    Full,
    #[default]
    Partial,
    None,
}

impl std::fmt::Display for AgentAutonomy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AgentAutonomy::Full => write!(f, "full"),
            AgentAutonomy::Partial => write!(f, "partial"),
            AgentAutonomy::None => write!(f, "none"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentDevelopmentSettings {
    /// The selected model's ID.
    pub selected_model_id: LLMId,
    pub autonomy: Option<AgentAutonomy>,
    /// Whether the CLI agent toolbar is enabled.
    pub cli_agent_toolbar_enabled: bool,
    /// The default session mode chosen during onboarding.
    pub session_default: SessionDefault,
    /// Whether the user chose to disable the Oz AI assistant.
    pub disable_oz: bool,
    /// Whether agent notifications are shown.
    pub show_agent_notifications: bool,
}

impl AgentDevelopmentSettings {
    pub fn new(default_model_id: LLMId) -> Self {
        Self {
            selected_model_id: default_model_id,
            autonomy: Some(AgentAutonomy::default()),
            cli_agent_toolbar_enabled: true,
            session_default: SessionDefault::Agent,
            disable_oz: false,
            show_agent_notifications: true,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum ProjectOnboardingSettings {
    #[default]
    NoProject,
    Project {
        selected_local_folder: String,
        initialize_projects_automatically: bool,
    },
}

impl ProjectOnboardingSettings {
    pub fn from_path(path: Option<String>) -> Self {
        match path {
            None => ProjectOnboardingSettings::NoProject,
            Some(path) => ProjectOnboardingSettings::Project {
                selected_local_folder: path,
                initialize_projects_automatically: true,
            },
        }
    }
}
