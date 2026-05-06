//! Minimal subset of the onboarding slide types.
//!
//! The main app depends on a few data types that originated in the onboarding
//! slide implementations (model picker info, autonomy presets, project settings
//! etc.). We keep them here without the heavy UI.

pub mod layout;
pub mod slide_content;
mod types;

pub use types::{
    AgentAutonomy, AgentDevelopmentSettings, OnboardingModelInfo, ProjectOnboardingSettings,
};
