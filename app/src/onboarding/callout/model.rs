use crate::onboarding::OnboardingIntention;
use warpui::{Entity, ModelContext};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinalState {
    Submit,
    Skip,
    Finish,
    Initialize,
    BackToTerminal,
}

impl std::fmt::Display for FinalState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FinalState::Submit => write!(f, "submitted"),
            FinalState::Skip => write!(f, "skipped"),
            FinalState::Finish => write!(f, "finished"),
            FinalState::Initialize => write!(f, "initialize"),
            FinalState::BackToTerminal => write!(f, "back_to_terminal"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OnboardingQuery {
    TerminalCommand(String),
    AgentPrompt(String),
    None,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum OnboardingCalloutModelEvent {
    StateUpdated,
    Completed(FinalState),
    EnterAgentModality,
    NaturalLanguageDetectionToggled(bool),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) enum UniversalInputCalloutState {
    #[default]
    Off,
    MeetInput,
    TalkToAgent,
    Complete(FinalState),
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) enum AgentModalityCalloutState {
    #[default]
    Off,
    MeetTerminalInput,
    NaturalLanguageSupport,
    IntroducingAgentExperience,
    UpdatedAgentInput,
    Complete(FinalState),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OnboardingCalloutState {
    UniversalInput(UniversalInputCalloutState),
    AgentModality(AgentModalityCalloutState),
}

pub(super) struct OnboardingCalloutModel {
    state: OnboardingCalloutState,
    intention: OnboardingIntention,
    has_project: bool,
    initial_natural_language_detection_enabled: bool,
    natural_language_detection_enabled: bool,
}

impl OnboardingCalloutModel {
    pub fn new_universal_input(
        has_project: bool,
        initial_natural_language_detection_enabled: bool,
    ) -> Self {
        Self {
            state: OnboardingCalloutState::UniversalInput(UniversalInputCalloutState::default()),
            intention: OnboardingIntention::AgentDrivenDevelopment,
            has_project,
            initial_natural_language_detection_enabled,
            natural_language_detection_enabled: initial_natural_language_detection_enabled,
        }
    }

    pub fn new_agent_modality(
        has_project: bool,
        intention: OnboardingIntention,
        initial_natural_language_detection_enabled: bool,
    ) -> Self {
        Self {
            state: OnboardingCalloutState::AgentModality(AgentModalityCalloutState::default()),
            intention,
            has_project,
            initial_natural_language_detection_enabled,
            natural_language_detection_enabled: initial_natural_language_detection_enabled,
        }
    }

    pub fn has_project(&self) -> bool {
        self.has_project
    }

    pub fn intention(&self) -> OnboardingIntention {
        self.intention
    }

    pub fn initial_natural_language_detection_enabled(&self) -> bool {
        self.initial_natural_language_detection_enabled
    }

    pub fn natural_language_detection_enabled(&self) -> bool {
        self.natural_language_detection_enabled
    }

    pub fn next(&mut self, ctx: &mut ModelContext<Self>) {
        match self.state {
            OnboardingCalloutState::UniversalInput(s) => self.next_universal_input(s, ctx),
            OnboardingCalloutState::AgentModality(s) => self.next_agent_modality(s, ctx),
        }
    }

    fn next_universal_input(&mut self, state: UniversalInputCalloutState, ctx: &mut ModelContext<Self>) {
        let next_state = match state {
            UniversalInputCalloutState::Off => Some(UniversalInputCalloutState::MeetInput),
            UniversalInputCalloutState::MeetInput => Some(UniversalInputCalloutState::TalkToAgent),
            UniversalInputCalloutState::TalkToAgent => Some(UniversalInputCalloutState::Complete(FinalState::Submit)),
            UniversalInputCalloutState::Complete(_) => None,
        };
        if let Some(ns) = next_state {
            self.set_state(OnboardingCalloutState::UniversalInput(ns), ctx);
        }
    }

    fn next_agent_modality(&mut self, state: AgentModalityCalloutState, ctx: &mut ModelContext<Self>) {
        let (next_state, emit_enter_agent_modality) = match state {
            AgentModalityCalloutState::Off => (Some(AgentModalityCalloutState::MeetTerminalInput), false),
            AgentModalityCalloutState::MeetTerminalInput => (Some(AgentModalityCalloutState::NaturalLanguageSupport), false),
            AgentModalityCalloutState::NaturalLanguageSupport => match self.intention {
                OnboardingIntention::Terminal => (Some(AgentModalityCalloutState::Complete(FinalState::Finish)), false),
                OnboardingIntention::AgentDrivenDevelopment => (Some(AgentModalityCalloutState::IntroducingAgentExperience), true),
            },
            AgentModalityCalloutState::IntroducingAgentExperience => (Some(AgentModalityCalloutState::UpdatedAgentInput), false),
            AgentModalityCalloutState::UpdatedAgentInput => {
                let fs = if self.has_project { FinalState::Initialize } else { FinalState::Finish };
                (Some(AgentModalityCalloutState::Complete(fs)), false)
            }
            AgentModalityCalloutState::Complete(_) => (None, false),
        };

        if let Some(ns) = next_state {
            self.set_state(OnboardingCalloutState::AgentModality(ns), ctx);
        }
        if emit_enter_agent_modality {
            ctx.emit(OnboardingCalloutModelEvent::EnterAgentModality);
        }
    }

    pub fn skip(&mut self, ctx: &mut ModelContext<Self>) {
        match self.state {
            OnboardingCalloutState::UniversalInput(UniversalInputCalloutState::TalkToAgent) => {
                self.set_state(
                    OnboardingCalloutState::UniversalInput(UniversalInputCalloutState::Complete(FinalState::Skip)),
                    ctx,
                );
            }
            OnboardingCalloutState::AgentModality(AgentModalityCalloutState::UpdatedAgentInput) => {
                self.set_state(
                    OnboardingCalloutState::AgentModality(AgentModalityCalloutState::Complete(FinalState::Skip)),
                    ctx,
                );
            }
            _ => {}
        }
    }

    pub fn finish(&mut self, ctx: &mut ModelContext<Self>) {
        match self.state {
            OnboardingCalloutState::UniversalInput(UniversalInputCalloutState::TalkToAgent) => {
                self.set_state(
                    OnboardingCalloutState::UniversalInput(UniversalInputCalloutState::Complete(FinalState::Finish)),
                    ctx,
                );
            }
            OnboardingCalloutState::AgentModality(AgentModalityCalloutState::NaturalLanguageSupport)
            | OnboardingCalloutState::AgentModality(AgentModalityCalloutState::UpdatedAgentInput) => {
                self.set_state(
                    OnboardingCalloutState::AgentModality(AgentModalityCalloutState::Complete(FinalState::Finish)),
                    ctx,
                );
            }
            _ => {}
        }
    }

    pub fn back_to_terminal(&mut self, ctx: &mut ModelContext<Self>) {
        match self.state {
            OnboardingCalloutState::AgentModality(AgentModalityCalloutState::UpdatedAgentInput) => {
                self.set_state(
                    OnboardingCalloutState::AgentModality(AgentModalityCalloutState::Complete(FinalState::BackToTerminal)),
                    ctx,
                );
            }
            _ => {}
        }
    }

    pub fn is_onboarding_active(&self) -> bool {
        match self.state {
            OnboardingCalloutState::UniversalInput(state) => !matches!(
                state,
                UniversalInputCalloutState::Off | UniversalInputCalloutState::Complete(_)
            ),
            OnboardingCalloutState::AgentModality(state) => !matches!(
                state,
                AgentModalityCalloutState::Off | AgentModalityCalloutState::Complete(_)
            ),
        }
    }

    pub fn state(&self) -> OnboardingCalloutState {
        self.state
    }

    pub fn prompt_string(&self) -> String {
        match self.prompt() {
            OnboardingQuery::TerminalCommand(s) | OnboardingQuery::AgentPrompt(s) => s,
            OnboardingQuery::None => String::new(),
        }
    }

    pub fn prompt(&self) -> OnboardingQuery {
        match self.state {
            OnboardingCalloutState::UniversalInput(s) => self.prompt_for_universal_input(s),
            OnboardingCalloutState::AgentModality(s) => self.prompt_for_agent_modality(s),
        }
    }

    fn prompt_for_universal_input(&self, state: UniversalInputCalloutState) -> OnboardingQuery {
        match state {
            UniversalInputCalloutState::MeetInput => OnboardingQuery::TerminalCommand("git status".to_string()),
            UniversalInputCalloutState::TalkToAgent | UniversalInputCalloutState::Complete(FinalState::Submit) => {
                OnboardingQuery::AgentPrompt(
                    "What tests exist in this repo, how are they structured, and what do they cover?".to_string(),
                )
            }
            _ => OnboardingQuery::None,
        }
    }

    fn prompt_for_agent_modality(&self, state: AgentModalityCalloutState) -> OnboardingQuery {
        match state {
            AgentModalityCalloutState::MeetTerminalInput => {
                OnboardingQuery::TerminalCommand("Run a command...".to_string())
            }
            AgentModalityCalloutState::NaturalLanguageSupport => {
                OnboardingQuery::AgentPrompt("help me terraform my Gcloud setup".to_string())
            }
            AgentModalityCalloutState::IntroducingAgentExperience => {
                OnboardingQuery::AgentPrompt("Tell the agent what to build...".to_string())
            }
            AgentModalityCalloutState::UpdatedAgentInput => {
                if self.has_project {
                    OnboardingQuery::AgentPrompt("/init".to_string())
                } else {
                    OnboardingQuery::AgentPrompt("Tell the agent what to build...".to_string())
                }
            }
            _ => OnboardingQuery::None,
        }
    }

    pub fn start_onboarding(&mut self, ctx: &mut ModelContext<Self>) {
        match self.state {
            OnboardingCalloutState::UniversalInput(_) => {
                self.set_state(
                    OnboardingCalloutState::UniversalInput(UniversalInputCalloutState::MeetInput),
                    ctx,
                );
            }
            OnboardingCalloutState::AgentModality(_) => {
                self.set_state(
                    OnboardingCalloutState::AgentModality(AgentModalityCalloutState::MeetTerminalInput),
                    ctx,
                );
            }
        }
    }

    pub fn toggle_natural_language_detection(&mut self, ctx: &mut ModelContext<Self>) {
        self.natural_language_detection_enabled = !self.natural_language_detection_enabled;
        ctx.emit(OnboardingCalloutModelEvent::NaturalLanguageDetectionToggled(
            self.natural_language_detection_enabled,
        ));
        ctx.emit(OnboardingCalloutModelEvent::StateUpdated);
        ctx.notify();
    }

    fn set_state(&mut self, new_state: OnboardingCalloutState, ctx: &mut ModelContext<Self>) {
        if self.state == new_state {
            return;
        }
        self.state = new_state;
        ctx.emit(OnboardingCalloutModelEvent::StateUpdated);

        let final_state = match new_state {
            OnboardingCalloutState::UniversalInput(UniversalInputCalloutState::Complete(fs)) => Some(fs),
            OnboardingCalloutState::AgentModality(AgentModalityCalloutState::Complete(fs)) => Some(fs),
            _ => None,
        };
        if let Some(fs) = final_state {
            ctx.emit(OnboardingCalloutModelEvent::Completed(fs));
        }
    }
}

impl Entity for OnboardingCalloutModel {
    type Event = OnboardingCalloutModelEvent;
}
