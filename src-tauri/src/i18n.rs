//! Translations for texts drawn by Rust (the tray menu), from the same files as the UI.

use std::collections::HashMap;

include!(concat!(env!("OUT_DIR"), "/locales.rs"));

pub const FALLBACK_LOCALE: &str = "en";

/// Same rules as `detectLocale` in `src/lib/i18n.ts`.
pub fn detect_locale<'a>(
    preferred: impl IntoIterator<Item = &'a str>,
    available: &[&str],
) -> String {
    preferred
        .into_iter()
        .filter_map(|tag| tag.split(['-', '_']).next())
        .map(str::to_lowercase)
        .find(|primary| available.contains(&primary.as_str()))
        .unwrap_or_else(|| FALLBACK_LOCALE.to_owned())
}

pub struct Translator {
    messages: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

impl Translator {
    pub fn new(locale: &str) -> Self {
        Translator {
            messages: catalog(locale),
            fallback: catalog(FALLBACK_LOCALE),
        }
    }

    /// Message for `key`, falling back to English, then to the key itself.
    /// `{name}` placeholders are replaced by the matching parameter.
    pub fn t(&self, key: &str, params: &[(&str, &str)]) -> String {
        let message = self
            .messages
            .get(key)
            .or_else(|| self.fallback.get(key))
            .map_or(key, String::as_str);
        params
            .iter()
            .fold(message.to_owned(), |text, (name, value)| {
                text.replace(&format!("{{{name}}}"), value)
            })
    }
}

fn catalog(locale: &str) -> HashMap<String, String> {
    LOCALES
        .iter()
        .find(|(lang, _)| *lang == locale)
        .and_then(|(_, json)| serde_json::from_str(json).ok())
        .unwrap_or_default()
}

pub fn available_locales() -> Vec<&'static str> {
    LOCALES.iter().map(|(lang, _)| *lang).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embeds_english_and_french() {
        let locales = available_locales();
        assert!(
            locales.contains(&"en") && locales.contains(&"fr"),
            "{locales:?}"
        );
    }

    #[test]
    fn detects_the_first_available_preferred_language() {
        assert_eq!(detect_locale(["de-DE", "fr-FR"], &["en", "fr"]), "fr");
        assert_eq!(detect_locale(["FR_ca"], &["en", "fr"]), "fr");
        assert_eq!(detect_locale(["ja"], &["en", "fr"]), "en");
        assert_eq!(detect_locale([], &["en", "fr"]), "en");
    }

    #[test]
    fn translates_with_fallback_and_parameters() {
        let fr = Translator::new("fr");
        assert_eq!(fr.t("tray.quit", &[]), "Quitter TunnelBar");
        assert_eq!(Translator::new("xx").t("tray.quit", &[]), "Quit TunnelBar");
        assert_eq!(fr.t("no.such.key", &[]), "no.such.key");
        assert_eq!(
            Translator::new("en").t("tray.retrying", &[("name", "Prod"), ("seconds", "4")]),
            "Prod — retrying in 4 s"
        );
    }
}
