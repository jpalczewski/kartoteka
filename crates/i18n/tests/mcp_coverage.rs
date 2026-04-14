//! Verifies that every expected MCP tool key exists in locales/en/mcp.ftl and locales/pl/mcp.ftl.
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// The canonical list of MCP tool keys that must be present in mcp.ftl.
const EXPECTED_TOOL_KEYS: &[&str] = &[
    "tool-list-lists",
    "tool-create-list",
    "tool-update-list",
    "tool-move-list",
    "tool-get-list-sublists",
    "tool-set-list-placement",
    "tool-get-items",
    "tool-add-item",
    "tool-update-item",
    "tool-toggle-item",
    "tool-move-item",
    "tool-list-containers",
    "tool-create-container",
    "tool-get-container",
    "tool-get-container-children",
    "tool-get-home",
    "tool-list-tags",
    "tool-create-tag",
    "tool-assign-tag",
    "tool-remove-tag",
    "tool-set-tag-links",
    "tool-get-tagged-items",
    "tool-get-calendar",
    "tool-get-today",
];

fn keys_in_mcp_ftl(locale: &str) -> HashSet<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../locales")
        .join(locale)
        .join("mcp.ftl");
    let content =
        fs::read_to_string(&path).unwrap_or_else(|_| panic!("cannot read {}", path.display()));
    let resource = match fluent_syntax::parser::parse(content.as_str()) {
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

#[test]
fn all_mcp_tool_keys_present_in_en() {
    let keys = keys_in_mcp_ftl("en");
    for expected in EXPECTED_TOOL_KEYS {
        assert!(
            keys.contains(*expected),
            "Missing key '{expected}' in locales/en/mcp.ftl"
        );
    }
}

#[test]
fn all_mcp_tool_keys_present_in_pl() {
    let keys = keys_in_mcp_ftl("pl");
    for expected in EXPECTED_TOOL_KEYS {
        assert!(
            keys.contains(*expected),
            "Missing key '{expected}' in locales/pl/mcp.ftl"
        );
    }
}
