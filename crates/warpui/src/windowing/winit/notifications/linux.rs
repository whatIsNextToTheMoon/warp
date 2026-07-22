use futures::FutureExt;
use winit::event_loop::EventLoopProxy;

use crate::notification::{NotificationResponse, NotificationSendError};
use crate::WindowId;
use crate::windowing::winit::app::CustomEvent;
use crate::windowing::winit::notifications::NotificationInfo;

pub(super) async fn send_notification(
    notification_info: NotificationInfo,
    window_id: WindowId,
    proxy: EventLoopProxy<CustomEvent>,
) {
    let NotificationInfo {
        notification_content,
        on_error,
    } = notification_info;

    let mut notification = notify_rust::Notification::new();
    notification
        .summary(notification_content.title())
        .body(notification_content.body());
    if let Some(label) = notification_content.default_action_label() {
        notification.action("default", label);
    }

    let sent_date = crate::time::get_current_time().naive_utc();
    let notification_data = notification_content.data().map(str::to_owned);
    let action_proxy = proxy.clone();

    notification
        .show_async()
        .then(|handle| async move {
            match handle {
                Ok(handle) => {
                    // Waiting for an action blocks until the notification is clicked or closed,
                    // so use the blocking threadpool rather than the shared background executor.
                    blocking::unblock(move || {
                        // Keeping the handle alive is required for the notification and its
                        // actions to remain available on some desktop environments.
                        handle.wait_for_action(|action| {
                            if action == "default" {
                                let response = NotificationResponse::new(
                                    sent_date,
                                    notification_data,
                                );
                                if let Err(error) = action_proxy.send_event(
                                    CustomEvent::NotificationClicked {
                                        window_id,
                                        response,
                                    },
                                ) {
                                    log::warn!(
                                        "Unable to handle notification click after event loop closed: {error:?}"
                                    );
                                }
                            } else {
                                log::info!("Notification closed via {action:?}");
                            }
                        })
                    })
                    .await;
                }
                Err(err) => {
                    // Always consider the error to be a `NotificationSendError::Other`.
                    // Dbus does not report if a notification couldn't be shown because
                    // the application didn't have permissions, so we can never return a
                    // `NotificationSendError::PermissionDenied` error.
                    let error = NotificationSendError::Other {
                        error_message: err.to_string(),
                    };

                    let _ = proxy.send_event(CustomEvent::UpdateUIApp(Box::new(|ctx| {
                        on_error(error, ctx);
                    })));
                }
            }
        })
        .await
}
