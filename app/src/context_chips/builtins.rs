//! Context chips built into Warp

use chrono::Local;
use warp_util::path::user_friendly_path;

use super::context_chip::{GeneratorContext, ShellCommand, ShellCommandGenerator};
use super::ChipValue;
use crate::terminal::shell::ShellType;

#[cfg(test)]
#[path = "builtins_tests.rs"]
mod tests;

/// Generator function for the current working directory.
pub fn working_directory(ctx: &GeneratorContext) -> Option<ChipValue> {
    let pwd = ctx.active_block_metadata.current_working_directory()?;
    let home_dir = ctx.active_session.and_then(|session| session.home_dir());
    Some(ChipValue::Text(
        user_friendly_path(pwd, home_dir).to_string(),
    ))
}

/// Generator function that always shows the username.
pub fn username(ctx: &GeneratorContext) -> Option<ChipValue> {
    ctx.active_session
        .map(|session| ChipValue::Text(session.user().to_owned()))
}

/// Generator function that always shows the host name.
pub fn hostname(ctx: &GeneratorContext) -> Option<ChipValue> {
    ctx.active_session
        .map(|session| ChipValue::Text(session.hostname().to_owned()))
}

/// Generator function that shows the current Python virtual environment.
pub fn virtual_environment(ctx: &GeneratorContext) -> Option<ChipValue> {
    ctx.current_environment
        .python_virtualenv()
        .cloned()
        .map(ChipValue::Text)
}

/// Generator function that shows the current Anaconda/conda environment.
pub fn conda_environment(ctx: &GeneratorContext) -> Option<ChipValue> {
    ctx.current_environment
        .conda_environment()
        .cloned()
        .map(ChipValue::Text)
}

/// Generator function that shows the current Node.js version.
pub fn node_version(ctx: &GeneratorContext) -> Option<ChipValue> {
    ctx.current_environment
        .node_version()
        .cloned()
        .map(ChipValue::Text)
}

/// Generator function that shows the current date.
pub fn date(_: &GeneratorContext) -> Option<ChipValue> {
    Some(ChipValue::Text(
        Local::now().format("%a %b %d %Y").to_string(),
    ))
}

/// Generator function that shows the current time in 12-hour format.
pub fn time12(_: &GeneratorContext) -> Option<ChipValue> {
    Some(ChipValue::Text(Local::now().format("%I:%M %P").to_string()))
}

/// Generator function that shows the current time in 24-hour format.
pub fn time24(_: &GeneratorContext) -> Option<ChipValue> {
    Some(ChipValue::Text(Local::now().format("%H:%M").to_string()))
}

/// Generator function that shows the current 12-hour time with seconds.
pub fn time12_with_seconds(_: &GeneratorContext) -> Option<ChipValue> {
    Some(ChipValue::Text(
        Local::now().format("%I:%M:%S %P").to_string(),
    ))
}

/// Generator function that shows the current 24-hour time with seconds.
pub fn time24_with_seconds(_: &GeneratorContext) -> Option<ChipValue> {
    Some(ChipValue::Text(Local::now().format("%H:%M:%S").to_string()))
}

/// Generator function for SSH session chip.
pub fn ssh_session(ctx: &GeneratorContext) -> Option<ChipValue> {
    let session = ctx.active_session?;
    if session.is_ssh_wrapper_session()
        || matches!(
            session.session_type(),
            crate::terminal::model::session::SessionType::WarpifiedRemote { .. }
        )
    {
        let user = session.user();
        Some(ChipValue::Text(format!("{}@{}", user, session.hostname())))
    } else {
        None
    }
}

/// Generator function for Subshell session chip.
pub fn subshell(ctx: &GeneratorContext) -> Option<ChipValue> {
    let session = ctx.active_session?;
    let subshell_info = session.subshell_info().as_ref()?;

    let session_type = if let Some(env_var_collection_name) = &subshell_info.env_var_collection_name
    {
        env_var_collection_name.clone()
    } else {
        subshell_info
            .spawning_command
            .split_whitespace()
            .next()
            .unwrap_or("subshell")
            .to_string()
    };
    Some(ChipValue::Text(session_type))
}

/// Generator function that shows the current Git branch.
pub fn shell_git_branch() -> ShellCommandGenerator {
    // Note this command must stay in sync with how PrecmdValue::git_branch is generated in the
    // bootstrap scripts, at least until that is removed.
    const SH_COMMAND: &str = "GIT_OPTIONAL_LOCKS=0 git symbolic-ref --short HEAD 2> /dev/null || \
     GIT_OPTIONAL_LOCKS=0 git rev-parse --short HEAD 2> /dev/null";
    let pwsh_command = safe_git_powershell(
        "git symbolic-ref --short HEAD  2>$null; \
            if ($? -eq $false) { \
                git rev-parse --short HEAD 2>$null; \
            }",
    );

    let command = ShellCommand::shell_specific([
        (ShellType::PowerShell, pwsh_command),
        (ShellType::Bash, SH_COMMAND.to_string()),
        (ShellType::Zsh, SH_COMMAND.to_string()),
        (ShellType::Fish, SH_COMMAND.to_string()),
    ]);

    ShellCommandGenerator::new(command, Some(vec!["git".to_owned()]))
}

pub fn shell_other_git_branches() -> ShellCommandGenerator {
    const SH_COMMAND: &str = "git --no-optional-locks branch --no-color --sort=-committerdate; \
        printf '\\036\\n'; \
        git --no-optional-locks worktree list --porcelain";
    let pwsh_command = safe_git_powershell(
        "git --no-optional-locks branch --no-color --sort=-committerdate; \
        [char]30; \
        git --no-optional-locks worktree list --porcelain",
    );

    let command = ShellCommand::shell_specific([
        (ShellType::PowerShell, pwsh_command),
        (ShellType::Bash, SH_COMMAND.to_string()),
        (ShellType::Zsh, SH_COMMAND.to_string()),
        (ShellType::Fish, SH_COMMAND.to_string()),
    ]);

    ShellCommandGenerator::new(command, Some(vec!["git".to_owned()]))
}

pub fn shell_git_branch_status() -> ShellCommandGenerator {
    const SH_COMMAND: &str = "\
        sh -c 'branch=$(GIT_OPTIONAL_LOCKS=0 git symbolic-ref --short HEAD 2>/dev/null || \
        GIT_OPTIONAL_LOCKS=0 git rev-parse --short HEAD 2>/dev/null) || exit 1; \
        [ -n \"$branch\" ] || exit 1; \
        display_count() { if [ \"$1\" -gt 999 ]; then printf \"999+\"; else printf \"%s\" \"$1\"; fi; }; \
        if counts=$(GIT_OPTIONAL_LOCKS=0 git rev-list --left-right --cherry-mark --count HEAD...@{u} 2>/dev/null); then \
            set -- $counts; \
            ahead=${1:-0}; behind=${2:-0}; equivalent=${3:-0}; status=\"\"; \
            if [ \"$ahead\" -eq 0 ] && [ \"$behind\" -eq 0 ] && [ \"$equivalent\" -gt 0 ]; then \
                status=\"⇅\"; \
            else \
                if [ \"$ahead\" -gt 0 ]; then status=\"↑$(display_count \"$ahead\")\"; fi; \
                if [ \"$behind\" -gt 0 ]; then \
                    behind_status=\"↓$(display_count \"$behind\")\"; \
                    if [ -n \"$status\" ]; then status=\"$status $behind_status\"; else status=\"$behind_status\"; fi; \
                fi; \
            fi; \
            if [ -n \"$status\" ]; then printf \"%s • %s\\n\" \"$branch\" \"$status\"; else printf \"%s\\n\" \"$branch\"; fi; \
        else \
            printf \"%s\\n\" \"$branch\"; \
        fi'";
    let pwsh_command = safe_git_powershell(
        "$branch = git symbolic-ref --short HEAD 2>$null; \
        if ($LASTEXITCODE -ne 0 -or -not $branch) { \
            $branch = git rev-parse --short HEAD 2>$null; \
        } \
        if ($LASTEXITCODE -ne 0 -or -not $branch) { throw } \
        function Format-GitCount($count) { if ($count -gt 999) { '999+' } else { [string]$count } } \
        $counts = git rev-list --left-right --cherry-mark --count 'HEAD...@{u}' 2>$null; \
        if ($LASTEXITCODE -eq 0 -and $counts) { \
            $parts = $counts -split '\\s+'; \
            if ($parts.Length -ge 2) { \
                $ahead = [int]$parts[0]; \
                $behind = [int]$parts[1]; \
                $equivalent = if ($parts.Length -ge 3) { [int]$parts[2] } else { 0 }; \
                $status = @(); \
                if ($ahead -eq 0 -and $behind -eq 0 -and $equivalent -gt 0) { \
                    $status += '⇅'; \
                } else { \
                    if ($ahead -gt 0) { $status += \"↑$(Format-GitCount $ahead)\" } \
                    if ($behind -gt 0) { $status += \"↓$(Format-GitCount $behind)\" } \
                } \
                if ($status.Count -gt 0) { \"$branch • $($status -join ' ')\" } else { $branch } \
            } else { \
                $branch; \
            } \
        } else { \
            $branch; \
            $global:LASTEXITCODE = 0; \
        }",
    );

    let command = ShellCommand::shell_specific([
        (ShellType::PowerShell, pwsh_command),
        (ShellType::Bash, SH_COMMAND.to_string()),
        (ShellType::Zsh, SH_COMMAND.to_string()),
        (ShellType::Fish, SH_COMMAND.to_string()),
    ]);

    ShellCommandGenerator::new(command, Some(vec!["git".to_owned()]))
}

/// Generator function to get summary of git diff (num files changed and num lines changed).
///
/// Used as a remote-session fallback when GitRepoStatusModel is unavailable.
pub fn shell_git_line_changes() -> ShellCommandGenerator {
    const GIT_COMMAND: &str =
        "GIT_OPTIONAL_LOCKS=0 git -c diff.autoRefreshIndex=false diff --shortstat HEAD";

    let command = ShellCommand::shell_specific([
        (ShellType::Bash, GIT_COMMAND.to_string()),
        (ShellType::Zsh, GIT_COMMAND.to_string()),
        (ShellType::Fish, GIT_COMMAND.to_string()),
        (
            ShellType::PowerShell,
            safe_git_powershell("git -c diff.autoRefreshIndex=false diff --shortstat HEAD"),
        ),
    ]);

    ShellCommandGenerator::new(command, Some(vec!["git".to_owned()]))
}

pub fn github_pull_request_url() -> ShellCommandGenerator {
    // `gh pr view` exits non-zero both when there is no PR for the current
    // branch and when the command actually fails. The wrapper treats "no PR"
    // as an empty success while preserving auth/config/network failures.
    const SH_COMMAND: &str = r#"git rev-parse --is-inside-work-tree >/dev/null 2>&1 || exit 0
git symbolic-ref --quiet --short HEAD >/dev/null 2>&1 || exit 0

remote_url=$(git remote get-url origin 2>/dev/null) || exit 0
case "$remote_url" in
    git@github.com:*|https://github.com/*|http://github.com/*|ssh://git@github.com/*)
        ;;
    *)
        exit 0
        ;;
esac

output=$(gh pr view --json url --jq .url 2>&1)
exit_code=$?

if [ $exit_code -eq 0 ]; then
    printf '%s\n' "$output"
else
    case "$output" in
        *'no pull requests found for branch '*|*'no open pull requests found for branch '*)
            exit 0
            ;;
    esac
    printf '%s\n' "$output" >&2
    exit $exit_code
fi"#;
    const FISH_COMMAND: &str = r#"git rev-parse --is-inside-work-tree >/dev/null 2>/dev/null; or exit 0
git symbolic-ref --quiet --short HEAD >/dev/null 2>/dev/null; or exit 0

set remote_url (git remote get-url origin 2>/dev/null); or exit 0
string match -rq '^(git@github\.com:|https?://github\.com/|ssh://git@github\.com/)' -- $remote_url; or exit 0

set output (gh pr view --json url --jq .url 2>&1)
set exit_code $status

if test $exit_code -eq 0
    printf '%s\n' "$output"
else
    set joined_output (string join '\n' $output)
    string match -rq 'no (open )?pull requests found for branch ' -- $joined_output; and exit 0
    printf '%s\n' "$joined_output" >&2
    exit $exit_code
end"#;
    const PWSH_COMMAND: &str = r#"git rev-parse --is-inside-work-tree 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) { exit 0 }

$branch = git symbolic-ref --quiet --short HEAD 2>$null
if ($LASTEXITCODE -ne 0 -or [string]::IsNullOrWhiteSpace($branch)) { exit 0 }

$remoteUrl = git remote get-url origin 2>$null
if ($LASTEXITCODE -ne 0) { exit 0 }
if ($remoteUrl -notmatch '^(git@github\.com:|https?://github\.com/|ssh://git@github\.com/)') { exit 0 }

$output = gh pr view --json url --jq .url 2>&1 | Out-String
$exitCode = $LASTEXITCODE
$output = $output.TrimEnd()

if ($exitCode -eq 0) {
    if (-not [string]::IsNullOrWhiteSpace($output)) { $output }
    exit 0
}

if ($output -match 'no (open )?pull requests found for branch ') { exit 0 }
if (-not [string]::IsNullOrWhiteSpace($output)) { [Console]::Error.WriteLine($output) }
exit $exitCode"#;

    let command = ShellCommand::shell_specific([
        (ShellType::PowerShell, PWSH_COMMAND.to_string()),
        (ShellType::Bash, SH_COMMAND.to_string()),
        (ShellType::Zsh, SH_COMMAND.to_string()),
        (ShellType::Fish, FISH_COMMAND.to_string()),
    ]);

    ShellCommandGenerator::new(command, Some(vec!["gh".to_owned(), "git".to_owned()]))
}

pub fn kubernetes_current_context() -> ShellCommandGenerator {
    ShellCommandGenerator::new(
        ShellCommand::portable("kubectl config current-context"),
        Some(vec!["kubectl".to_owned()]),
    )
}

/// Generator function that shows the current svn "branch".
/// Since svn uses directories for different branches and tags,
/// we take the latest directory of the working copy as the branch/tag name.
pub fn svn_branch_context() -> ShellCommandGenerator {
    const SH_COMMAND: &str = "basename $(svn info --show-item wc-root)";
    const PWSH_COMMAND: &str = "svn info --show-item wc-root | Split-Path -Leaf";
    let command = ShellCommand::shell_specific([
        (ShellType::PowerShell, PWSH_COMMAND.to_string()),
        (ShellType::Bash, SH_COMMAND.to_string()),
        (ShellType::Zsh, SH_COMMAND.to_string()),
        (ShellType::Fish, SH_COMMAND.to_string()),
    ]);

    ShellCommandGenerator::new(command, Some(vec!["svn".to_owned()]))
}

/// Generator function that shows the number of uncommitted svn files/directories.
pub fn svn_dirty_items() -> ShellCommandGenerator {
    const SH_COMMAND: &str = "count=$(svn status | wc -l) \
        && (( $count > 0 )) && echo $(( $count ))";
    const FISH_COMMAND: &str = "set count (svn status | wc -l) \
        && test $count -gt 0 && string trim $count";
    const PWSH_COMMAND: &str = "svn status | Measure-Object -line | \
        where {$_.Lines -gt 0 } | foreach { $_.Lines }";
    let command = ShellCommand::shell_specific([
        (ShellType::Bash, SH_COMMAND.to_string()),
        (ShellType::Zsh, SH_COMMAND.to_string()),
        (ShellType::Fish, FISH_COMMAND.to_string()),
        (ShellType::PowerShell, PWSH_COMMAND.to_string()),
    ]);
    ShellCommandGenerator::new(command, Some(vec!["svn".to_owned()]))
}

fn safe_git_powershell(cmd: &str) -> String {
    format!(
        "\
        $gitOptionalLocks = $env:GIT_OPTIONAL_LOCKS; \
        $env:GIT_OPTIONAL_LOCKS = 0; \
        try {{ \
            {cmd} \
        }} finally {{ \
            $success = $?; \
            $exitCode = $LASTEXITCODE; \
            $env:GIT_OPTIONAL_LOCKS = $gitOptionalLocks; \
            if ($exitCode -ne 0 -or -not $success) {{ \
                throw \
            }} \
        }}"
    )
}
