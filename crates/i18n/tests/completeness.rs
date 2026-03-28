//! Asserts that every .ftl message key in en/ exists in pl/ and vice versa.
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn extract_keys(ftl_content: &str) -> BTreeSet<String> {
    let resource = match fluent_syntax::parser::parse(ftl_content) {
        Ok(r) => r,
        Err((r, _)) => r,
    };
    resource
        .body
        .iter()
        .filter_map(|entry| {
            if let fluent_syntax::ast::Entry::Message(msg) = entry {
                Some(msg.id.name.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn ftl_files_in(dir: &Path) -> Vec<String> {
    let mut files: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .filter_map(|e| {
            let e = e.unwrap();
            let name = e.file_name().to_string_lossy().to_string();
            if name.ends_with(".ftl") {
                Some(name)
            } else {
                None
            }
        })
        .collect();
    files.sort();
    files
}

#[test]
fn en_and_pl_have_same_ftl_files() {
    let locales = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../locales");
    let en_files = ftl_files_in(&locales.join("en"));
    let pl_files = ftl_files_in(&locales.join("pl"));
    assert_eq!(
        en_files, pl_files,
        "en/ and pl/ must have the same .ftl files"
    );
}

#[test]
fn en_and_pl_have_same_keys() {
    let locales = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../locales");
    let en_dir = locales.join("en");
    let pl_dir = locales.join("pl");

    for filename in ftl_files_in(&en_dir) {
        let en_content = fs::read_to_string(en_dir.join(&filename)).unwrap();
        let pl_content = fs::read_to_string(pl_dir.join(&filename)).unwrap();

        let en_keys = extract_keys(&en_content);
        let pl_keys = extract_keys(&pl_content);

        let missing_in_pl: Vec<_> = en_keys.difference(&pl_keys).collect();
        let missing_in_en: Vec<_> = pl_keys.difference(&en_keys).collect();

        assert!(
            missing_in_pl.is_empty(),
            "{filename}: keys in en/ missing from pl/: {missing_in_pl:?}"
        );
        assert!(
            missing_in_en.is_empty(),
            "{filename}: keys in pl/ missing from en/: {missing_in_en:?}"
        );
    }
}
