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
    let _ = warpui::i18n::set_translator(Box::new(move |text| {
        let collect_missing = collect_missing_translations_enabled();

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
        log::info!(
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
    if s.contains('/') || s.contains('\\') {
        return false;
    }
    if s.contains("://") {
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
    let lower = s.to_ascii_lowercase();
    if lower.starts_with("$ ")
        || lower.starts_with("> ")
        || lower.starts_with("sudo ")
        || lower.starts_with("cd ")
        || lower.starts_with("git ")
        || lower.starts_with("ssh ")
        || lower.starts_with("curl ")
        || lower.starts_with("http ")
        || lower.starts_with("https ")
    {
        return false;
    }

    true
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
