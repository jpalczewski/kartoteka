use fluent_bundle::concurrent::FluentBundle;
use fluent_bundle::{FluentArgs, FluentResource, FluentValue};
use std::collections::HashMap;
use std::path::Path;
use unic_langid::LanguageIdentifier;

pub struct McpI18n {
    bundles: HashMap<String, FluentBundle<FluentResource>>,
}

impl McpI18n {
    /// Load FTL files from `<locales_dir>/<lang>/mcp.ftl`.
    pub fn load_from(locales_dir: &Path) -> Self {
        let mut bundles = HashMap::new();
        for lang in ["en", "pl"] {
            let path = locales_dir.join(lang).join("mcp.ftl");
            let src = std::fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            let resource = FluentResource::try_new(src)
                .unwrap_or_else(|e| panic!("parse {}: {e:?}", path.display()));
            let id: LanguageIdentifier = lang.parse().expect("parse langid");
            let mut bundle = FluentBundle::new_concurrent(vec![id]);
            bundle.add_resource(resource).expect("add resource");
            bundle.set_use_isolating(false);
            bundles.insert(lang.to_string(), bundle);
        }
        Self { bundles }
    }

    /// Load FTL files from `locales/` relative to the current working directory.
    pub fn load() -> Self {
        Self::load_from(Path::new("locales"))
    }

    pub fn translate(&self, locale: &str, key: &str) -> String {
        self.translate_args(locale, key, &[])
    }

    pub fn translate_args(&self, locale: &str, key: &str, args: &[(&str, &str)]) -> String {
        let bundle = self
            .bundles
            .get(locale)
            .or_else(|| self.bundles.get("en"))
            .expect("en fallback bundle missing");
        let Some(msg) = bundle.get_message(key) else {
            return key.to_string();
        };
        let Some(pattern) = msg.value() else {
            return key.to_string();
        };
        let mut fluent_args = FluentArgs::new();
        for (k, v) in args {
            fluent_args.set(*k, FluentValue::from(v.to_string()));
        }
        let mut errors = vec![];
        bundle
            .format_pattern(pattern, Some(&fluent_args), &mut errors)
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_i18n() -> McpI18n {
        // CARGO_MANIFEST_DIR is crates/mcp; locales are two levels up
        let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let locales = manifest.parent().unwrap().parent().unwrap().join("locales");
        McpI18n::load_from(&locales)
    }

    #[test]
    fn loads_en_and_pl() {
        let i = test_i18n();
        assert!(
            i.translate("en", "mcp-tool-create_item-desc")
                .contains("Create")
        );
        assert!(
            i.translate("pl", "mcp-tool-create_item-desc")
                .contains("Utwórz")
        );
    }

    #[test]
    fn formats_args() {
        let i = test_i18n();
        let s = i.translate_args("en", "mcp-err-not-found", &[("entity", "item")]);
        assert!(s.contains("item"), "got: {s}");
    }

    #[test]
    fn unknown_key_returns_key() {
        let i = test_i18n();
        assert_eq!(i.translate("en", "nonexistent-key"), "nonexistent-key");
    }
}
