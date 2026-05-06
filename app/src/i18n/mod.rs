use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::env;
use std::sync::{Mutex, OnceLock};

/// Enable basic UI string translation (Chinese) for WarpUI text elements.
///
/// This is intentionally lightweight: we translate only *exact* string matches
/// from a mapping table, and the core `warpui_core::elements::Text` only applies
/// translation to borrowed/static strings by default (see `warpui_core::i18n`).
pub fn init() {
    if !is_chinese_locale() {
        return;
    }

    let map = zh_cn_map();
    let collect_missing = collect_missing_translations_enabled();
    if collect_missing {
        // Use WARN so this shows up even with default log filters (which are
        // commonly set to `warn` in release builds). This feature is opt-in.
        log::warn!(
            target: "warp_i18n",
            "Enabled missing translation collection (env: WARP_I18N_COLLECT_MISSES=1)."
        );
        if collect_missing_translations_allow_slash() {
            log::warn!(
                target: "warp_i18n",
                "Missing translation collection: allowing limited slash UI strings (env: WARP_I18N_COLLECT_MISSES_ALLOW_SLASH=1)."
            );
        }
    }
    let _ = warpui::i18n::set_translator(Box::new(move |text| {
        // Fast path: exact match.
        if let Some(t) = map.get(text) {
            return Some(Cow::Borrowed(*t));
        }

        // Robustness: many UI call sites include incidental leading/trailing whitespace
        // (for example when composing a sentence with a link). We allow translations to
        // be keyed by the trimmed string while preserving the original whitespace.
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }
        if trimmed != text {
            if let Some(t) = map.get(trimmed) {
                let prefix_len = text.len() - text.trim_start().len();
                let suffix_len = text.len() - text.trim_end().len();
                let prefix = &text[..prefix_len];
                let suffix = &text[text.len() - suffix_len..];

                let mut out = String::with_capacity(prefix.len() + t.len() + suffix.len());
                out.push_str(prefix);
                out.push_str(t);
                out.push_str(suffix);
                return Some(Cow::Owned(out));
            }
        }

        // Extra robustness: some strings are written with newlines or multiple spaces for layout.
        // If the input contains "weird" whitespace, try a whitespace-normalized lookup.
        //
        // We only do this when whitespace looks non-trivial to avoid unnecessary allocations for
        // common short labels.
        if text.contains('\n') || text.contains('\t') || text.contains("  ") {
            let normalized = normalize_whitespace(trimmed);
            if normalized != trimmed {
                if let Some(t) = map.get(normalized.as_str()) {
                    return Some(Cow::Borrowed(*t));
                }
            }
        }

        // Best-effort dynamic/pattern translations for common UI strings.
        //
        // These cover places where the UI builds labels via `format!(...)` so exact-match
        // lookup isn't possible (e.g. "Create foo…" or "Failed to export bar").
        //
        // Keep this intentionally small and conservative to avoid translating user content.
        if let Some(rest) = trimmed.strip_prefix("Create ") {
            if let Some(name) = rest.strip_suffix('…') {
                return Some(Cow::Owned(format!("创建 {name}…")));
            }
        }
        if let Some(rest) = trimmed.strip_prefix("Create file: ") {
            return Some(Cow::Owned(format!("创建文件：{rest}")));
        }
        if let Some(rest) = trimmed.strip_prefix("Press Enter to create ") {
            if let Some(name) = rest.strip_suffix(" in the current directory") {
                return Some(Cow::Owned(format!("按 Enter 在当前目录创建 {name}")));
            }
        }
        if let Some(rest) = trimmed.strip_prefix("Directory: ") {
            return Some(Cow::Owned(format!("目录：{rest}")));
        }
        if let Some(rest) = trimmed.strip_prefix("File: ") {
            return Some(Cow::Owned(format!("文件：{rest}")));
        }
        if let Some(rest) = trimmed.strip_prefix("Notebook: ") {
            return Some(Cow::Owned(format!("笔记本：{rest}")));
        }
        if let Some(rest) = trimmed.strip_prefix("Workflow: ") {
            return Some(Cow::Owned(format!("工作流：{rest}")));
        }
        if let Some(rest) = trimmed.strip_prefix("Rule: ") {
            return Some(Cow::Owned(format!("规则：{rest}")));
        }
        if let Some(rest) = trimmed.strip_prefix("Move to ") {
            return Some(Cow::Owned(format!("移动到 {rest}")));
        }
        if let Some(rest) = trimmed.strip_prefix("Copied link to ") {
            if let Some(name) = rest.strip_suffix('.') {
                return Some(Cow::Owned(format!("已复制链接到 {name}。")));
            }
        }
        if let Some(rest) = trimmed.strip_prefix("Exported ") {
            return Some(Cow::Owned(format!("已导出 {rest}")));
        }
        if let Some(rest) = trimmed.strip_prefix("Failed to export ") {
            return Some(Cow::Owned(format!("导出失败：{rest}")));
        }

        if collect_missing {
            record_missing_translation(text, trimmed);
        }
        None
    }));
}

/// Returns whether the current UI locale should be treated as Chinese.
///
/// This is used both for installing the UI translator and for locale-aware
/// formatting of dynamic strings (time-ago labels, toast messages, etc).
pub fn is_chinese_locale() -> bool {
    match preferred_locale() {
        Some(locale) => locale_is_chinese(&locale),
        None => false,
    }
}

/// Translate a UI-authored string literal into the current locale (if a translator is installed).
///
/// This is primarily for call sites that build UI copy as owned `String`s (e.g. via
/// `FormattedTextFragment`), which do not automatically go through the `Text` /
/// `FormattedTextElement::from_str` translation hook.
///
/// When missing translation collection is enabled (`WARP_I18N_COLLECT_MISSES=1`), this also lets
/// the collector observe and record misses for these strings.
pub(crate) fn ui_str(text: &'static str) -> String {
    warpui::i18n::translate(text)
        .map(|t| t.into_owned())
        .unwrap_or_else(|| text.to_string())
}

/// Translate a UI-authored string into the current locale (if a translator is installed).
///
/// Unlike [`ui_str`], this accepts any `&str`, which is useful for call sites that work with
/// borrowed runtime strings that still come from UI-authored static content (for example, values
/// stored in instruction tables or tooltip metadata).
pub(crate) fn ui_text(text: &str) -> String {
    warpui::i18n::translate(text)
        .map(|t| t.into_owned())
        .unwrap_or_else(|| text.to_string())
}

fn normalize_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_ws = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !prev_ws {
                out.push(' ');
                prev_ws = true;
            }
        } else {
            out.push(ch);
            prev_ws = false;
        }
    }
    out.trim().to_string()
}

fn preferred_locale() -> Option<String> {
    env::var("WARP_UI_LOCALE")
        .ok()
        .or_else(|| env::var("WARP_LOCALE").ok())
        .or_else(|| env::var("LC_ALL").ok())
        .or_else(|| env::var("LC_MESSAGES").ok())
        .or_else(|| env::var("LANG").ok())
        .map(|raw| normalize_locale(&raw))
        .filter(|s| !s.is_empty())
}

fn normalize_locale(raw: &str) -> String {
    // Examples:
    // - "zh_CN.UTF-8" -> "zh-CN"
    // - "en_US" -> "en-US"
    // - "C" -> "C"
    let raw = raw.trim();
    let raw = raw.split('.').next().unwrap_or(raw);
    let raw = raw.replace('_', "-");

    let mut parts = raw.splitn(3, '-');
    let lang = parts.next().unwrap_or("").to_ascii_lowercase();
    let region = parts.next().map(|p| p.to_ascii_uppercase());
    let rest = parts.next().map(|p| p.to_string());

    let mut out = lang;
    if let Some(region) = region.filter(|s| !s.is_empty()) {
        out.push('-');
        out.push_str(&region);
    }
    if let Some(rest) = rest.filter(|s| !s.is_empty()) {
        out.push('-');
        out.push_str(&rest);
    }
    out
}

fn locale_is_chinese(locale: &str) -> bool {
    // Keep this intentionally permissive; we can tighten later.
    // "zh", "zh-CN", "zh-SG", "zh-Hans"...
    locale.to_ascii_lowercase().starts_with("zh")
}

fn zh_cn_map() -> &'static HashMap<&'static str, &'static str> {
    static MAP: OnceLock<HashMap<&'static str, &'static str>> = OnceLock::new();
    MAP.get_or_init(|| load_leaked_string_map(include_str!("zh_cn.json")))
}

fn collect_missing_translations_enabled() -> bool {
    env_flag("WARP_I18N_COLLECT_MISSES")
}

fn collect_missing_translations_allow_slash() -> bool {
    env_flag("WARP_I18N_COLLECT_MISSES_ALLOW_SLASH")
}

fn env_flag(name: &str) -> bool {
    env::var(name)
        .ok()
        .map(|v| v.trim().to_ascii_lowercase())
        .is_some_and(|v| matches!(v.as_str(), "1" | "true" | "yes" | "on"))
}

fn record_missing_translation(original: &str, trimmed: &str) {
    let s = trimmed;
    if s.is_empty() {
        return;
    }

    // Avoid logging potentially sensitive user content. This is a best-effort filter.
    if !looks_like_ui_string(s) {
        return;
    }

    static SEEN: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let seen = SEEN.get_or_init(|| Mutex::new(HashSet::new()));
    let mut guard = match seen.lock() {
        Ok(g) => g,
        Err(poisoned) => poisoned.into_inner(),
    };

    // Prefer trimmed strings as stable keys, but keep the original around for debugging.
    if guard.insert(s.to_string()) {
        // Use WARN so misses are visible without requiring users to tweak
        // RUST_LOG. This is still opt-in via WARP_I18N_COLLECT_MISSES=1 and
        // filtered through `looks_like_ui_string`.
        log::warn!(
            target: "warp_i18n",
            "Missing zh-CN translation: {:?} (orig={:?})",
            s,
            original
        );
    }
}

fn looks_like_ui_string(s: &str) -> bool {
    // Only consider ASCII-ish short/medium strings to reduce false positives from terminal output.
    if !s.is_ascii() {
        return false;
    }
    let len = s.len();
    if len < 2 || len > 240 {
        return false;
    }
    if s.contains('\n') || s.contains('\r') || s.contains('\t') {
        return false;
    }
    // Common indicators of non-UI/user content.
    //
    // NOTE: We intentionally treat path separators as non-UI by default to avoid
    // persisting file paths, URLs, and other user-controlled strings in logs.
    //
    // However, some *UI* strings legitimately contain a forward slash, notably:
    // - "Ctrl /" (keyboard shortcut hint)
    // - "Slash command: /agent" (keyboard shortcuts list)
    //
    // To make localization easier without widening the default safety boundary,
    // we gate this behind an extra opt-in env var and only allow a *narrow* set
    // of forward-slash patterns.
    if s.contains('\\') {
        return false;
    }
    if s.contains('/') {
        if !collect_missing_translations_allow_slash() {
            return false;
        }
        if !looks_like_safe_slash_ui_string(s) {
            return false;
        }
    }
    if s.contains("://") {
        return false;
    }

    // Avoid logging secrets / tokens / keys. Even though the collector is opt-in, the log file is
    // persistent and we should not record anything that looks like a credential.
    let lower = s.to_ascii_lowercase();
    if lower.starts_with("sk-")
        || lower.starts_with("sk_")
        || lower.starts_with("ak_")
        || lower.starts_with("ghp_")
        || lower.starts_with("github_pat_")
        || lower.starts_with("xox")
        || lower.starts_with("aizasy")
    {
        return false;
    }
    // Exclude obvious AWS key shapes and other long opaque tokens.
    if s.starts_with("AKIA") || s.starts_with("ASIA") {
        return false;
    }
    if s.len() >= 24
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'=' | b'-' | b'_'))
        && s.bytes().any(|b| b.is_ascii_digit())
    {
        return false;
    }

    // Require at least one ASCII letter.
    if !s
        .bytes()
        .any(|b| (b'A'..=b'Z').contains(&b) || (b'a'..=b'z').contains(&b))
    {
        return false;
    }

    // Heuristic: exclude strings that look like shell prompts/commands.
    if lower.starts_with("$ ")
        || lower.starts_with("> ")
        || lower.starts_with("sudo ")
        || lower.starts_with("cd ")
        || lower.starts_with("git ")
        || lower.starts_with("ssh ")
        || lower.starts_with("curl ")
        || lower.starts_with("aws ")
        || lower.starts_with("oz ")
        || lower.starts_with("npm ")
        || lower.starts_with("pip ")
        || lower.starts_with("python ")
        || lower.starts_with("node ")
        || lower.starts_with("cargo ")
        || lower.starts_with("http ")
        || lower.starts_with("https ")
    {
        return false;
    }

    // Exclude common "user@host" / email-like tokens that are often terminal prompts or user data.
    if let Some((left, right)) = s.split_once('@') {
        let left_tok = left.split_whitespace().last().unwrap_or("");
        let right_tok = right.split_whitespace().next().unwrap_or("");
        if !left_tok.is_empty()
            && !right_tok.is_empty()
            && left_tok
                .bytes()
                .all(|b: u8| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-')
            && right_tok.bytes().all(|b: u8| {
                b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_' | b':' | b'~')
            })
            && (right_tok.contains('.') || right_tok.contains(':') || right_tok.contains('~'))
        {
            return false;
        }
    }

    true
}

fn looks_like_safe_slash_ui_string(s: &str) -> bool {
    // We only allow *one* forward slash. This filters out typical absolute/relative paths
    // like "/home/user/..." and "src/foo/bar", while still allowing UI tokens like "Ctrl /"
    // and "/agent".
    let slash_count = s.as_bytes().iter().filter(|&&b| b == b'/').count();
    if slash_count != 1 {
        return false;
    }

    let lower = s.to_ascii_lowercase();

    // Common path-like prefixes (Linux/macOS) that we should never log.
    // Note: we keep this list short and conservative.
    if lower.starts_with("~/")
        || lower.starts_with("./")
        || lower.starts_with("../")
        || lower.starts_with("/home")
        || lower.starts_with("/users")
        || lower.starts_with("/usr")
        || lower.starts_with("/etc")
        || lower.starts_with("/var")
        || lower.starts_with("/tmp")
        || lower.starts_with("/opt")
        || lower.starts_with("/proc")
        || lower.starts_with("/sys")
        || lower.starts_with("/dev")
        || lower.starts_with("/run")
        || lower.starts_with("/mnt")
        || lower.starts_with("/media")
        || lower.starts_with("/volumes")
        || lower.starts_with("/applications")
    {
        return false;
    }

    // Windows drive-like paths: "C:/..."
    if lower.len() >= 3 {
        let b = lower.as_bytes();
        if b[1] == b':' && b[2] == b'/' && b[0].is_ascii_alphabetic() {
            return false;
        }
    }

    // Case 1: A standalone slash separator in UI copy (e.g. "slash command / fork menus").
    // This is very unlikely to be a filesystem path, and it helps catch untranslated
    // instructional text.
    if s.contains(" / ") {
        return true;
    }

    // Case 2: Keybinding token like "Ctrl /".
    if s.ends_with('/') {
        if lower.contains("ctrl") || lower.contains("cmd") || lower.contains("meta") {
            return true;
        }
    }

    // Case 3: Slash command token like "/agent" or "/fork-and-compact" possibly embedded
    // in a longer sentence (e.g. "Slash command: /agent").
    if let Some(idx) = s.find('/') {
        let after = &s[idx + 1..];
        let mut chars = after.chars();
        let first = match chars.next() {
            Some(c) => c,
            None => return false,
        };
        if !first.is_ascii_alphabetic() {
            return false;
        }

        // Consume the command word.
        let mut consumed = 1usize;
        for c in chars {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                consumed += c.len_utf8();
                continue;
            }
            break;
        }

        let cmd = &after[..consumed];
        if cmd.len() > 64 {
            return false;
        }

        // Block any "command" that looks like a common directory even if it starts with "/".
        // This prevents accidental logging of "/home" or "/usr" when shown standalone.
        match cmd.to_ascii_lowercase().as_str() {
            "home" | "users" | "usr" | "etc" | "var" | "tmp" | "opt" | "proc" | "sys" | "dev"
            | "run" | "mnt" | "media" | "volumes" | "applications" => return false,
            _ => {}
        }

        return true;
    }

    false
}

fn load_leaked_string_map(json: &str) -> HashMap<&'static str, &'static str> {
    let raw: HashMap<String, String> =
        serde_json::from_str(json).expect("zh_cn.json must be valid JSON");
    raw.into_iter()
        .map(|(k, v)| (leak_str(k), leak_str(v)))
        .collect()
}

fn leak_str(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}
