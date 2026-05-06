use crate::appearance::Appearance;
use crate::onboarding::OnboardingIntention;
use warpui::elements::Empty;
use warpui::{AppContext, Element, Entity, ModelHandle, SingletonEntity, UpdateModel, View, ViewContext};

use super::model::{
    AgentModalityCalloutState, FinalState, OnboardingCalloutModel, OnboardingCalloutModelEvent,
    OnboardingCalloutState, OnboardingQuery, UniversalInputCalloutState,
};

#[derive(Clone, Debug)]
pub struct OnboardingKeybindings {
    pub toggle_input_mode: String,
    pub submit_to_local_agent: String,
    pub submit_to_cloud_agent: String,
}

#[derive(Clone, Debug)]
pub enum OnboardingCalloutViewEvent {
    StateUpdated,
    Completed { final_state: FinalState },
    EnterAgentModality,
    NaturalLanguageDetectionToggled(bool),
}

pub struct OnboardingCalloutView {
    model: ModelHandle<OnboardingCalloutModel>,
}

impl OnboardingCalloutView {
    pub fn new_universal_input(
        has_project: bool,
        initial_natural_language_detection_enabled: bool,
        _keybindings: OnboardingKeybindings,
        ctx: &mut ViewContext<Self>,
    ) -> Self {
        let model = ctx.add_model(|_| {
            OnboardingCalloutModel::new_universal_input(
                has_project,
                initial_natural_language_detection_enabled,
            )
        });
        Self::with_model(model, ctx)
    }

    pub fn new_agent_modality(
        has_project: bool,
        intention: OnboardingIntention,
        initial_natural_language_detection_enabled: bool,
        _keybindings: OnboardingKeybindings,
        ctx: &mut ViewContext<Self>,
    ) -> Self {
        let model = ctx.add_model(|_| {
            OnboardingCalloutModel::new_agent_modality(
                has_project,
                intention,
                initial_natural_language_detection_enabled,
            )
        });
        Self::with_model(model, ctx)
    }

    fn with_model(model: ModelHandle<OnboardingCalloutModel>, ctx: &mut ViewContext<Self>) -> Self {
        ctx.subscribe_to_model(&model, |_me, _model, event, ctx| {
            let event = match event {
                OnboardingCalloutModelEvent::StateUpdated => OnboardingCalloutViewEvent::StateUpdated,
                OnboardingCalloutModelEvent::Completed(final_state) => {
                    OnboardingCalloutViewEvent::Completed {
                        final_state: *final_state,
                    }
                }
                OnboardingCalloutModelEvent::EnterAgentModality => {
                    OnboardingCalloutViewEvent::EnterAgentModality
                }
                OnboardingCalloutModelEvent::NaturalLanguageDetectionToggled(enabled) => {
                    OnboardingCalloutViewEvent::NaturalLanguageDetectionToggled(*enabled)
                }
            };
            ctx.emit(event);
            ctx.notify();
        });

        Self { model }
    }

    pub fn has_project(&self, app: &AppContext) -> bool {
        self.model.as_ref(app).has_project()
    }

    pub fn start_onboarding(&mut self, ctx: &mut ViewContext<Self>) {
        self.model.update(ctx, |model, ctx| {
            model.start_onboarding(ctx);
        });
        ctx.notify();
    }

    pub fn is_onboarding_active(&self, app: &AppContext) -> bool {
        self.model.as_ref(app).is_onboarding_active()
    }

    pub fn prompt_string(&self, app: &AppContext) -> String {
        self.model.as_ref(app).prompt_string()
    }

    pub fn prompt(&self, app: &AppContext) -> OnboardingQuery {
        self.model.as_ref(app).prompt()
    }

    pub fn should_position_above_zero_state(&self, app: &AppContext) -> bool {
        !matches!(
            self.model.as_ref(app).state(),
            OnboardingCalloutState::AgentModality(AgentModalityCalloutState::UpdatedAgentInput)
        )
    }
}

impl Entity for OnboardingCalloutView {
    type Event = OnboardingCalloutViewEvent;
}

impl View for OnboardingCalloutView {
    fn ui_name() -> &'static str {
        "OnboardingCalloutView"
    }

    fn render(&self, app: &AppContext) -> Box<dyn Element> {
        let _appearance = Appearance::as_ref(app);
        let state = self.model.as_ref(app).state();
        if !self.model.as_ref(app).is_onboarding_active() {
            return Empty::new().finish();
        }

        match state {
            OnboardingCalloutState::UniversalInput(UniversalInputCalloutState::Off)
            | OnboardingCalloutState::AgentModality(AgentModalityCalloutState::Off) => {
                Empty::new().finish()
            }
            _ => Empty::new().finish(),
        }
    }
}

impl SingletonEntity for OnboardingCalloutView {}

pub fn init(_app: &mut AppContext) {}
