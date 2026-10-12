use warpui::AppContext;

use crate::context_chips::display_chip::GitLineChanges;
use crate::context_chips::{ContextChipKind, git_line_changes_from_chips};
use crate::terminal::TerminalView;

impl TerminalView {
    pub(crate) fn custom_title_with_status(&self, custom_title: &str) -> String {
        custom_title_with_status(&self.terminal_title, custom_title)
    }

    fn prompt_chip_value(&self, chip_kind: &ContextChipKind, ctx: &AppContext) -> Option<String> {
        self.current_prompt
            .as_ref(ctx)
            .latest_chip_value(chip_kind, ctx)
            .map(|v| v.to_string())
            .filter(|value| !value.trim().is_empty())
    }

    pub fn display_working_directory(&self, ctx: &AppContext) -> Option<String> {
        let raw = self
            .prompt_chip_value(&ContextChipKind::WorkingDirectory, ctx)
            .or_else(|| self.pwd())?;
        let home_dir = self
            .active_block_session_id()
            .and_then(|session_id| self.sessions.as_ref(ctx).get(session_id))
            .and_then(|session| session.home_dir().map(str::to_owned));
        Some(warp_util::path::user_friendly_path(&raw, home_dir.as_deref()).to_string())
    }

    pub fn terminal_title_from_shell(&self) -> String {
        let model = self.model.lock();
        let fallback_title = model.shell_launch_state().display_name().to_owned();
        model
            .terminal_title()
            .filter(|title| !title.trim().is_empty())
            .unwrap_or(fallback_title)
    }

    pub fn current_git_branch(&self, ctx: &AppContext) -> Option<String> {
        self.prompt_chip_value(&ContextChipKind::ShellGitBranch, ctx)
            .or_else(|| {
                self.git_status_metadata(ctx)
                    .map(|metadata| metadata.current_branch_name.clone())
                    .filter(|branch| !branch.trim().is_empty())
            })
    }

    pub fn last_completed_command_text(&self) -> Option<String> {
        let model = self.model.lock();
        model.block_list().blocks().iter().rev().find_map(|block| {
            if block.finished()
                && !block.is_background()
                && !block.is_static()
                && !block.is_hidden()
                && !block.is_in_band_command_block()
                && (block.bootstrap_stage().is_done() || block.is_restored())
            {
                let cmd = block.command_to_string();
                if cmd.trim().is_empty() {
                    None
                } else {
                    Some(cmd)
                }
            } else {
                None
            }
        })
    }

    pub fn terminal_title_text(&self) -> String {
        if !self.terminal_title.trim().is_empty() {
            return self.terminal_title.clone();
        }
        self.terminal_title_from_shell()
    }

    pub fn current_pull_request_url(&self, ctx: &AppContext) -> Option<String> {
        self.current_prompt
            .as_ref(ctx)
            .latest_chip_value(&ContextChipKind::GithubPullRequest, ctx)
            .map(|v| v.to_string())
            .filter(|value| !value.trim().is_empty())
    }

    pub fn current_diff_line_changes(&self, ctx: &AppContext) -> Option<GitLineChanges> {
        // Prefer the externally-updated GitRepoStatusModel (local filesystem
        // watcher or remote daemon push receiver) over parsing the raw shell
        // chip output. This matches the preference order used by the prompt
        // chip display (display.rs) and agent footer (chips.rs).
        let from_model = self
            .git_status_metadata(ctx)
            .map(|metadata| GitLineChanges::from_diff_stats(&metadata.stats_against_head));

        from_model
            .or_else(|| {
                git_line_changes_from_chips(&self.current_prompt.as_ref(ctx).agent_view_chips(ctx))
            })
            .filter(|line_changes| {
                line_changes.files_changed > 0
                    || line_changes.lines_added > 0
                    || line_changes.lines_removed > 0
            })
    }
}

/// Characters that CLI coding agents use as the leading frame of the spinner they
/// write into the terminal title (OSC 0/2) while they are working.
///
/// Keep this aligned with the actual agents instead of with every glyph that could
/// look spinner-ish:
/// - Codex (TUI) cycles through braille frames (`\u{280b}` `\u{2819}` ...).
/// - Claude Code alternates `◐`/`◑` and uses `✳` while it is busy.
/// - `●` is used by the voice/recording indicator.
fn is_agent_spinner_char(c: char) -> bool {
    matches!(
        c,
        '\u{2801}'..='\u{28ff}'        // Braille patterns
            | '\u{25cf}'               // ●
            | '\u{25d0}'..='\u{25d3}'  // ◐ ◑ ◒ ◓
            | '\u{2733}'               // ✳
    )
}

/// Returns the leading spinner frame(s) of `title`, which must already be
/// trimmed of leading whitespace. Variation selectors following a frame are
/// included in the prefix.
///
/// The frame run must be followed by whitespace, so a title that merely starts
/// with a bullet (or a shell title such as `-zsh`) is not mistaken for a spinner.
fn leading_agent_spinner_prefix(title: &str) -> Option<&str> {
    let mut cursor = 0;
    let mut saw_frame = false;
    for c in title.chars() {
        if is_agent_spinner_char(c) {
            saw_frame = true;
        } else if c == '\u{fe0f}' {
            // A variation selector can follow a frame; it never starts one.
        } else {
            break;
        }
        cursor += c.len_utf8();
    }

    if !saw_frame || !title[cursor..].starts_with(char::is_whitespace) {
        return None;
    }
    Some(&title[..cursor])
}

/// Strips any leading whitespace and spinner frames from a title.
///
/// Used when persisting a custom tab name and when prefilling the rename editor:
/// the frame is re-attached from the live terminal title at render time, so
/// storing it would stack duplicate frames (and keep animating a stale one).
pub(crate) fn strip_agent_spinner_prefixes(mut title: &str) -> &str {
    loop {
        let trimmed = title.trim_start();
        match leading_agent_spinner_prefix(trimmed) {
            Some(prefix) => title = trimmed[prefix.len()..].trim_start(),
            None => return trimmed,
        }
    }
}

fn custom_title_with_status(terminal_title: &str, custom_title: &str) -> String {
    let custom_title = strip_agent_spinner_prefixes(custom_title.trim_start());
    if custom_title.is_empty() {
        return custom_title.to_owned();
    }
    match leading_agent_spinner_prefix(terminal_title.trim_start()) {
        Some(prefix) => format!("{prefix} {custom_title}"),
        None => custom_title.to_owned(),
    }
}

#[cfg(test)]
#[path = "tab_metadata_tests.rs"]
mod tests;
