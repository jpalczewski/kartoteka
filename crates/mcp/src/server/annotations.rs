//! Static MCP tool annotations grouped by behavior.
//!
//! Splitting this from `server.rs` keeps the tool-handler module focused on
//! request plumbing and lets us unit-test classification without spinning up a
//! server.

use rmcp::model::ToolAnnotations;

const READ_ONLY: &[&str] = &[
    "list_lists",
    "get_list",
    "list_items",
    "list_container_items",
    "list_containers",
    "get_container",
    "list_tags",
    "get_today",
    "get_time_summary",
    "get_relations",
    "get_tag_entities",
    "search_items",
    "get_item",
    "list_templates",
    "list_overdue",
    "get_active_timer",
    "list_marked_for_delete",
    "list_comments",
    "list_time_inbox",
];

const ADDITIVE_WRITE: &[&str] = &[
    "create_list",
    "create_item",
    "create_container",
    "create_items",
    "create_lists",
    "create_containers",
    "add_comment",
    "add_relation",
    "log_time",
    "start_timer",
    "create_list_from_template",
    "save_as_template",
    "create_tag",
    "create_tags",
    "assign_tag",
    "toggle_mark_for_delete",
];

const DESTRUCTIVE: &[&str] = &[
    "update_item",
    "toggle_item",
    "set_item_status",
    "set_container_status",
    "remove_relation",
    "stop_timer",
    "move_list_to_container",
    "move_container_to_parent",
    "unassign_tag",
    "assign_time_entry",
    "reorder_items",
    "move_item",
    "reorder_lists",
];

pub fn for_tool(name: &str) -> ToolAnnotations {
    let mut ann = ToolAnnotations::default();
    if READ_ONLY.contains(&name) {
        ann.read_only_hint = Some(true);
        ann.destructive_hint = Some(false);
        ann.idempotent_hint = Some(true);
        ann.open_world_hint = Some(false);
    } else if ADDITIVE_WRITE.contains(&name) {
        ann.read_only_hint = Some(false);
        ann.destructive_hint = Some(false);
        ann.idempotent_hint = Some(false);
        ann.open_world_hint = Some(false);
    } else if DESTRUCTIVE.contains(&name) {
        ann.read_only_hint = Some(false);
        ann.destructive_hint = Some(true);
        ann.idempotent_hint = Some(false);
        ann.open_world_hint = Some(false);
    }
    ann
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_only_tool_is_classified_as_read_only() {
        let a = for_tool("list_lists");
        assert_eq!(a.read_only_hint, Some(true));
        assert_eq!(a.destructive_hint, Some(false));
        assert_eq!(a.idempotent_hint, Some(true));
    }

    #[test]
    fn additive_write_tool_is_non_destructive_non_read_only() {
        let a = for_tool("create_item");
        assert_eq!(a.read_only_hint, Some(false));
        assert_eq!(a.destructive_hint, Some(false));
        assert_eq!(a.idempotent_hint, Some(false));
    }

    #[test]
    fn destructive_tool_is_flagged_destructive() {
        let a = for_tool("update_item");
        assert_eq!(a.read_only_hint, Some(false));
        assert_eq!(a.destructive_hint, Some(true));
    }

    #[test]
    fn toggle_item_tool_is_flagged_destructive() {
        let a = for_tool("toggle_item");
        assert_eq!(a.read_only_hint, Some(false));
        assert_eq!(a.destructive_hint, Some(true));
        assert_eq!(a.idempotent_hint, Some(false));
    }

    #[test]
    fn unknown_tool_has_no_hints() {
        let a = for_tool("does_not_exist");
        assert!(a.read_only_hint.is_none());
        assert!(a.destructive_hint.is_none());
        assert!(a.idempotent_hint.is_none());
        assert!(a.open_world_hint.is_none());
    }

    #[test]
    fn no_tool_appears_in_multiple_buckets() {
        for &name in READ_ONLY {
            assert!(
                !ADDITIVE_WRITE.contains(&name),
                "{name} is read-only AND additive"
            );
            assert!(
                !DESTRUCTIVE.contains(&name),
                "{name} is read-only AND destructive"
            );
        }
        for &name in ADDITIVE_WRITE {
            assert!(
                !DESTRUCTIVE.contains(&name),
                "{name} is additive AND destructive"
            );
        }
    }
}
