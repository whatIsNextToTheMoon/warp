use std::fs::remove_file;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use chrono::{LocalResult, TimeZone, Utc};
use warp_core::execution_mode::AppExecutionMode;
use warp_errors::{report_error, report_if_error};
use warpui::r#async::{FutureExt as _, Timer};
use warpui::{App, Entity, ModelContext, SingletonEntity};

use super::{clear_event_queue, rudder_event_file_path, RUDDER_TELEMETRY_EVENTS_FILE_NAME};
use crate::auth::AuthStateProvider;
use crate::channel::ChannelState;
use crate::features::FeatureFlag;
use crate::server::server_api::ServerApi;
use crate::settings::{PrivacySettings, PrivacySettingsChangedEvent};

// How often we send Active Usage signals.
const ACTIVE_USAGE_DURATION: Duration = Duration::from_secs(60);

/// Duration to wait before flushing the event queue to Rudderstack.
const TELEMETRY_FLUSH_DURATION: Duration = Duration::from_secs(30);

/// Max telemetry events to write to disk. This is bounded to limit the size of the file as well
/// as latency of writing the file.
const MAX_TELEMETRY_EVENTS_TO_STORE: usize = 20;

/// Maximum time to wait for the telemetry flush network request during shutdown.
/// If the network is unavailable or slow, we don't want the CLI process to hang indefinitely.
const TELEMETRY_SHUTDOWN_FLUSH_TIMEOUT: Duration = Duration::from_secs(5);

/// App singleton responsible for scheduling periodic background tasks for sending batches of
/// telemetry events to Rudderstack.  This model respects the user's telemetry enablement setting.
pub struct TelemetryCollector {
    server_api: Arc<ServerApi>,
}

impl TelemetryCollector {
    pub fn new(server_api: Arc<ServerApi>) -> Self {
        Self { server_api }
    }

    pub fn initialize_telemetry_collection(&self, ctx: &mut ModelContext<TelemetryCollector>) {
        let _ = ctx;
        clear_event_queue();
    }

    /// Writes all queued but unsent telemetry telemetry events to disk so that they may be sent
    /// on the next app startup.
    pub fn write_telemetry_events_to_disk(&self, ctx: &mut ModelContext<TelemetryCollector>) {
        let _ = &self.server_api;
        let _ = ctx;
        clear_event_queue();
    }

    /// Flushes telemetry events when the app is shutting down.
    ///
    /// Depending on the app's execution mode, this will either:
    /// * Write events to disk, for sending on the next app startup
    /// * Synchronously send events to rudderstack
    pub fn flush_telemetry_events_for_shutdown(&self, ctx: &mut ModelContext<TelemetryCollector>) {
        let _ = &self.server_api;
        let _ = ctx;
        clear_event_queue();
    }
}

impl Entity for TelemetryCollector {
    type Event = ();
}

impl SingletonEntity for TelemetryCollector {}
