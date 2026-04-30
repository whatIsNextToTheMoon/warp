use std::borrow::Cow;
use std::sync::OnceLock;

/// A translation result returned by a translator.
pub type Translation = Cow<'static, str>;

/// A localizable UI string identified by a stable key with an English fallback.
///
/// This is intended for incremental i18n adoption: call sites can provide a stable key
/// (e.g. `"common.ok"`) while preserving the original English text as a fallback.
#[derive(Clone, Copy, Debug)]
pub struct LocalizedText {
    key: &'static str,
    fallback: &'static str,
}

impl LocalizedText {
    pub const fn new(key: &'static str, fallback: &'static str) -> Self {
        Self { key, fallback }
    }
}

impl From<LocalizedText> for Cow<'static, str> {
    fn from(value: LocalizedText) -> Self {
        translate(value.key).unwrap_or(Cow::Borrowed(value.fallback))
    }
}

/// A global translator hook for UI strings.
///
/// Notes:
/// - This is intentionally optional and defaults to `None`.
/// - It is expected that applications (e.g. Warp) install a translator once during startup.
/// - Translators should be fast and ideally avoid allocation, as UI trees may be rebuilt often.
pub type TranslateFn = dyn Fn(&str) -> Option<Translation> + Send + Sync + 'static;

static TRANSLATOR: OnceLock<Box<TranslateFn>> = OnceLock::new();

/// Installs the global translator hook.
///
/// If a translator is already installed, this is a no-op and returns `false`.
pub fn set_translator(translator: Box<TranslateFn>) -> bool {
    TRANSLATOR.set(translator).is_ok()
}

/// Translates a string using the installed translator hook (if any).
pub fn translate(text: &str) -> Option<Translation> {
    TRANSLATOR.get().and_then(|t| t(text))
}

/// Convenience helper to construct a [`LocalizedText`].
#[inline]
pub const fn localized(key: &'static str, fallback: &'static str) -> LocalizedText {
    LocalizedText::new(key, fallback)
}
