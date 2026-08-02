//! App language / locale support.
//!
//! Wraps [`rust_i18n`] behind a small serializable [`Language`] enum that is
//! persisted in the user config (`settings.language`), applied globally at
//! boot via [`Language::apply`], and switchable at runtime from the Options
//! dialog. Translation files live in `locales/` (YAML, `en` / `zh-CN`); the
//! English source text doubles as the lookup key.

use serde::{Deserialize, Serialize};

/// UI languages shipped with the app.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Language {
    #[serde(rename = "en")]
    En,
    #[serde(rename = "zh-CN")]
    ZhCn,
}

impl Language {
    /// All available languages, in picker order.
    pub fn all() -> &'static [Language] {
        &[Language::En, Language::ZhCn]
    }

    /// The rust-i18n locale id.
    pub fn locale(self) -> &'static str {
        match self {
            Language::En => "en",
            Language::ZhCn => "zh-CN",
        }
    }

    /// Display label shown in the options picker (self-localized).
    pub fn label(self) -> &'static str {
        match self {
            Language::En => "English",
            Language::ZhCn => "简体中文",
        }
    }

    /// Reverse lookup of [`Language::label`], used by the options picker.
    pub fn from_label(label: &str) -> Option<Language> {
        Language::all()
            .iter()
            .copied()
            .find(|l| l.label() == label)
    }

    /// Switch the whole process to this language (native + web).
    pub fn apply(self) {
        rust_i18n::set_locale(self.locale());
    }

    /// Best-effort detection of the user's system language; falls back to
    /// English when the platform locale cannot be read.
    pub fn detect() -> Language {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let lang = std::env::var("LC_ALL")
                .or_else(|_| std::env::var("LC_MESSAGES"))
                .or_else(|_| std::env::var("LANG"))
                .ok();
            if let Some(lang) = lang {
                return if lang.to_ascii_lowercase().starts_with("zh") {
                    Language::ZhCn
                } else {
                    Language::En
                };
            }
        }
        #[cfg(target_arch = "wasm32")]
        {
            if let Some(lang) = web_sys::window()
                .and_then(|win| win.navigator().language())
            {
                return if lang.to_ascii_lowercase().starts_with("zh") {
                    Language::ZhCn
                } else {
                    Language::En
                };
            }
        }
        Language::En
    }
}

impl Default for Language {
    fn default() -> Self {
        Language::detect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_i18n::t;

    #[test]
    fn locale_roundtrip() {
        let zh = Language::ZhCn;
        assert_eq!(zh.locale(), "zh-CN");
        assert_eq!(Language::from_label("简体中文"), Some(zh));
        assert_eq!(Language::from_label("English"), Some(Language::En));
        assert_eq!(Language::from_label("Deutsch"), None);
    }

    #[test]
    fn translations_switch_with_locale() {
        rust_i18n::set_locale("en");
        assert_eq!(t!("Close"), "Close");
        rust_i18n::set_locale("zh-CN");
        assert_eq!(t!("Close"), "关闭");
        assert_eq!(t!("Default save format:"), "默认保存格式：");
        // Untranslated keys fall back to the source text itself.
        assert_eq!(t!("Definitely Not A Key"), "Definitely Not A Key");
        rust_i18n::set_locale("en");
    }
}
