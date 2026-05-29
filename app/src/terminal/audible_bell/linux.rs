//! Module containing an implementation of an audible bell for Linux backends.

use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use anyhow::{bail, Context};
use x11rb::protocol::xproto::ConnectionExt;
use x11rb::rust_connection::RustConnection;

const FALLBACK_BELL_MIN_INTERVAL: Duration = Duration::from_millis(80);
const FREEDESKTOP_BELL_SOUND: &str = "/usr/share/sounds/freedesktop/stereo/bell.oga";

#[derive(Clone, Copy)]
struct FallbackBellCommand {
    program: &'static str,
    args: &'static [&'static str],
}

/// A Linux implementation of an audible bell.
///
/// On X11 we use the X server bell. On native Wayland there may be no X11
/// connection at all, so fall back to the desktop sound theme / PulseAudio /
/// PipeWire helpers when available.
pub(super) struct AudibleBell {
    connection: Option<RustConnection>,
    fallback_command: Option<FallbackBellCommand>,
    last_fallback_bell_at: Mutex<Option<Instant>>,
}

impl AudibleBell {
    pub fn new() -> Self {
        let connection = RustConnection::connect(None)
            .ok()
            .map(|(connection, _)| connection);
        Self {
            connection,
            fallback_command: fallback_bell_command(),
            last_fallback_bell_at: Mutex::new(None),
        }
    }

    pub fn ring(&self) -> anyhow::Result<()> {
        if is_probably_wayland_session() {
            match self.ring_fallback() {
                Ok(()) => return Ok(()),
                Err(error) => {
                    log::debug!("Wayland audible bell fallback failed; trying X11 bell: {error:#}");
                }
            }
        }

        if let Some(connection) = &self.connection {
            // Play the bell at 0%. By using 0%, we indicate to the x server that the bell should be played at the user's
            // current volume. See https://www.x.org/releases/X11R7.7/doc/xproto/x11protocol.html#requests:Bell for more
            // details.
            match connection.bell(0) {
                Ok(cookie) => match cookie.check() {
                    Ok(()) => return Ok(()),
                    Err(error) => {
                        log::debug!("X11 audible bell failed; trying Linux fallback: {error:#}");
                    }
                },
                Err(error) => {
                    log::debug!("X11 audible bell failed; trying Linux fallback: {error:#}");
                }
            }
        }

        self.ring_fallback()
    }

    fn ring_fallback(&self) -> anyhow::Result<()> {
        let Some(command) = self.fallback_command else {
            bail!(
                "Unable to establish X11 bell connection and no Linux audible bell fallback command was found"
            );
        };

        let now = Instant::now();
        if let Ok(mut last) = self.last_fallback_bell_at.lock() {
            if let Some(last_at) = *last {
                if now.duration_since(last_at) < FALLBACK_BELL_MIN_INTERVAL {
                    return Ok(());
                }
            }
            *last = Some(now);
        }

        Command::new(command.program)
            .args(command.args)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .with_context(|| format!("Unable to spawn audible bell command {}", command.program))?;

        Ok(())
    }
}

fn is_probably_wayland_session() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some()
        || std::env::var_os("XDG_SESSION_TYPE")
            .and_then(|value| value.into_string().ok())
            .is_some_and(|value| value.eq_ignore_ascii_case("wayland"))
}

fn fallback_bell_command() -> Option<FallbackBellCommand> {
    if command_exists("canberra-gtk-play") {
        return Some(FallbackBellCommand {
            program: "canberra-gtk-play",
            args: &["-i", "bell"],
        });
    }

    if Path::new(FREEDESKTOP_BELL_SOUND).exists() {
        if command_exists("paplay") {
            return Some(FallbackBellCommand {
                program: "paplay",
                args: &[FREEDESKTOP_BELL_SOUND],
            });
        }

        if command_exists("pw-play") {
            return Some(FallbackBellCommand {
                program: "pw-play",
                args: &[FREEDESKTOP_BELL_SOUND],
            });
        }
    }

    None
}

fn command_exists(program: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };

    std::env::split_paths(&path).any(|dir| dir.join(program).is_file())
}
