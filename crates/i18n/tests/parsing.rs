//! Asserts that all .ftl files parse without errors.
use std::fs;
use std::path::Path;

#[test]
fn all_ftl_files_parse_without_errors() {
    let locales = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../locales");

    for locale in &["en", "pl"] {
        let dir = locales.join(locale);
        for entry in fs::read_dir(&dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("ftl") {
                continue;
            }

            let content = fs::read_to_string(&path).unwrap();
            let result = fluent_syntax::parser::parse(content.as_str());

            let errors = match result {
                Ok(_) => vec![],
                Err((_, errors)) => errors,
            };

            assert!(
                errors.is_empty(),
                "Parse errors in {}: {:?}",
                path.display(),
                errors
            );
        }
    }
}
