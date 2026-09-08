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

fn custom_title_with_status(terminal_title: &str, custom_title: &str) -> String {
    let title = terminal_title.trim_start();
    let Some(first) = title.chars().next() else {
        return custom_title.to_owned();
    };
    if !matches!(
        first,
        '\u{2801}'
            ..='\u{28ff}'
                | '\u{2733}'
                | '\u{2736}'
                | '\u{273b}'
                | '\u{273d}'
                | '\u{2722}'
                | '\u{00b7}'
                | '\u{25d0}'
                | '\u{25d1}'
                | '\u{25d2}'
                | '\u{25d3}'
                | '\u{231b}'
                | '\u{23f3}'
                | '|'
                | '/'
                | '-'
                | '\\'
    ) {
        return custom_title.to_owned();
    }
    let mut prefix_len = first.len_utf8();
    if title[prefix_len..].starts_with('\u{fe0f}') {
        prefix_len += '\u{fe0f}'.len_utf8();
    }
    if !title[prefix_len..].starts_with(char::is_whitespace) {
        return custom_title.to_owned();
    }
    let prefix = &title[..prefix_len];
    if custom_title.starts_with(&format!("{prefix} ")) {
        custom_title.to_owned()
    } else {
        format!("{prefix} {custom_title}")
    }
}

#[cfg(test)]
#[path = "tab_metadata_tests.rs"]
mod tests;
