use kartoteka_shared::TagUsage;
use kartoteka_shared::types::Tag;
use leptos::prelude::*;
use leptos_fluent::{I18n, move_tr};
use leptos_router::components::A;
use std::collections::{HashMap, HashSet};

use crate::components::tags::tag_badge::TagBadge;
use crate::components::ui::{ActionMenu, ActionMenuItem, Chip, Icon, IconButton};

/// A tag plus its children — the unit of recursive tree rendering.
#[derive(Clone, Debug)]
pub struct TagNode {
    pub tag: Tag,
    pub children: Vec<TagNode>,
}

/// Build a forest of `TagNode`s from a flat tag list. Returns root nodes (tags without a parent
/// in `tags`). Children and siblings are sorted alphabetically, case-insensitively.
pub fn build_tag_tree(tags: &[Tag]) -> Vec<TagNode> {
    let tag_ids: HashSet<&str> = tags.iter().map(|t| t.id.as_str()).collect();

    let mut children_map: HashMap<&str, Vec<&Tag>> = HashMap::new();
    let mut roots: Vec<&Tag> = Vec::new();

    for tag in tags {
        match &tag.parent_tag_id {
            Some(pid) if tag_ids.contains(pid.as_str()) => {
                children_map.entry(pid.as_str()).or_default().push(tag);
            }
            _ => roots.push(tag),
        }
    }

    roots.sort_by_key(|a| a.name.to_lowercase());
    for children in children_map.values_mut() {
        children.sort_by_key(|a| a.name.to_lowercase());
    }

    fn build_subtree_inner<'a>(
        tag: &'a Tag,
        children_map: &HashMap<&str, Vec<&'a Tag>>,
    ) -> TagNode {
        let children = children_map
            .get(tag.id.as_str())
            .map(|kids| {
                kids.iter()
                    .map(|t| build_subtree_inner(t, children_map))
                    .collect()
            })
            .unwrap_or_default();
        TagNode {
            tag: tag.clone(),
            children,
        }
    }

    roots
        .iter()
        .map(|t| build_subtree_inner(t, &children_map))
        .collect()
}

/// Keeps the tags matching `query` (case-insensitive substring of the name) and
/// all of their ancestors, so a filtered tree still shows where a match lives.
///
/// Returns the tags in their original order; an empty/blank `query` returns
/// every tag unchanged.
pub fn filter_tags(tags: &[Tag], query: &str) -> Vec<Tag> {
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return tags.to_vec();
    }

    let by_id: HashMap<&str, &Tag> = tags.iter().map(|t| (t.id.as_str(), t)).collect();
    let mut keep: HashSet<&str> = HashSet::new();
    for tag in tags {
        if !tag.name.to_lowercase().contains(&needle) {
            continue;
        }
        let mut current = Some(tag);
        // Bounded walk: a corrupted parent chain must not hang the page.
        for _ in 0..64 {
            let Some(node) = current else { break };
            keep.insert(node.id.as_str());
            current = node
                .parent_tag_id
                .as_deref()
                .and_then(|parent| by_id.get(parent).copied());
        }
    }

    tags.iter()
        .filter(|tag| keep.contains(tag.id.as_str()))
        .cloned()
        .collect()
}

/// Walk ancestors of `tag_id` up to its root. Returns the path from root to `tag_id`.
/// Detects and breaks out of cycles defensively.
pub fn build_breadcrumb(tags: &[Tag], tag_id: &str) -> Vec<Tag> {
    let tag_map: HashMap<&str, &Tag> = tags.iter().map(|t| (t.id.as_str(), t)).collect();
    let mut path = Vec::new();
    let mut current_id = Some(tag_id);
    let mut visited: HashSet<String> = HashSet::new();

    while let Some(id) = current_id {
        if !visited.insert(id.to_string()) {
            break;
        }
        if let Some(tag) = tag_map.get(id) {
            path.push((*tag).clone());
            current_id = tag.parent_tag_id.as_deref();
        } else {
            break;
        }
    }

    path.reverse();
    path
}

/// All descendant ids of `tag_id` (excluding `tag_id` itself).
pub fn get_descendant_ids(tags: &[Tag], tag_id: &str) -> Vec<String> {
    let mut descendants = Vec::new();
    let mut stack = vec![tag_id.to_string()];
    while let Some(id) = stack.pop() {
        for tag in tags {
            if tag.parent_tag_id.as_deref() == Some(&id) {
                descendants.push(tag.id.clone());
                stack.push(tag.id.clone());
            }
        }
    }
    descendants
}

/// Forest of nodes rooted at the direct children of `root_id`.
pub fn build_subtree(tags: &[Tag], root_id: &str) -> Vec<TagNode> {
    let descendant_ids = get_descendant_ids(tags, root_id);
    let subtags: Vec<Tag> = tags
        .iter()
        .filter(|t| descendant_ids.contains(&t.id))
        .cloned()
        .collect();
    build_tag_tree(&subtags)
}

/// The i18n key for a tag type label (`priority`, `country`, `city`, `address`, …).
///
/// Tag types drive behaviour (a `priority` tag is exclusive per item, location
/// tags feed the location pickers), so they are shown as a translated chip
/// instead of the bare stored string.
pub fn tag_type_key(tag_type: &str) -> &'static str {
    match tag_type {
        "priority" => "tags-type-priority",
        "country" => "tags-type-country",
        "city" => "tags-type-city",
        "address" => "tags-type-address",
        _ => "tags-type-tag",
    }
}

/// One tree row plus the child rows beneath it.
///
/// Collapse state lives in `collapsed` (owned by the page, so it survives
/// refetches for the lifetime of the page). Rows are indented by nesting their
/// container, which also draws the thin guide line down the left of a subtree.
/// Actions live behind an [`ActionMenu`]; `show_*` flags let the detail page
/// render the same rows read-only.
#[component]
pub fn TagTreeRow(
    node: TagNode,
    /// Ids of collapsed tags. The page owns this so collapsing survives refetches.
    collapsed: RwSignal<HashSet<String>>,
    /// Item/list usage counts per tag id, as loaded with the tag list.
    #[prop(default = HashMap::new())]
    usage: HashMap<String, TagUsage>,
    #[prop(default = Callback::new(|_| {}))] on_create_child: Callback<(String, String)>,
    #[prop(default = Callback::new(|_| {}))] on_rename: Callback<(String, String)>,
    #[prop(default = Callback::new(|_| {}))] on_delete: Callback<String>,
    #[prop(default = true)] show_add_child: bool,
    #[prop(default = true)] show_rename: bool,
    #[prop(default = true)] show_delete: bool,
    #[prop(default = true)] linkable: bool,
) -> impl IntoView {
    let i18n = expect_context::<I18n>();
    let tag = StoredValue::new(node.tag);
    let children = node.children;
    let has_children = !children.is_empty();

    let tag_id = tag.with_value(|t| t.id.clone());
    let id_for_toggle = tag_id.clone();
    let id_for_child = tag_id.clone();
    let id_for_rename = tag_id.clone();
    let id_for_delete = tag_id.clone();
    let id_for_usage = tag_id.clone();
    let id_for_href = tag_id;

    let adding_child = RwSignal::new(false);
    let new_child_name = RwSignal::new(String::new());
    let renaming = RwSignal::new(false);
    let rename_value = RwSignal::new(String::new());
    let menu_open = RwSignal::new(false);

    let collapsed_id = id_for_toggle.clone();
    let is_collapsed = Signal::derive(move || collapsed.get().contains(&collapsed_id));
    let toggle_label = Signal::derive(move || {
        if is_collapsed.get() {
            i18n.tr("common-expand")
        } else {
            i18n.tr("common-collapse")
        }
    });
    let chevron = Signal::derive(move || {
        if is_collapsed.get() {
            "chevron-right"
        } else {
            "chevron-down"
        }
        .to_string()
    });
    let on_toggle = move |_| {
        let id = id_for_toggle.clone();
        collapsed.update(|ids| {
            if !ids.remove(&id) {
                ids.insert(id);
            }
        });
    };

    let tag_type = tag.with_value(|t| t.tag_type.clone());
    let type_label = Signal::derive(move || i18n.tr(tag_type_key(&tag_type)));
    let show_type_chip = tag.with_value(|t| t.tag_type != "tag");

    let usage_count = usage.get(&id_for_usage).map_or(0, |usage| usage.total());

    let submit_rename = Callback::new(move |_: ()| {
        let name = rename_value.get_untracked().trim().to_string();
        renaming.set(false);
        if name.is_empty() {
            return;
        }
        on_rename.run((id_for_rename.clone(), name));
    });
    let submit_child = Callback::new(move |_: ()| {
        let name = new_child_name.get_untracked().trim().to_string();
        if name.is_empty() {
            return;
        }
        on_create_child.run((id_for_child.clone(), name));
        new_child_name.set(String::new());
        adding_child.set(false);
    });

    let menu_items = StoredValue::new({
        let mut items: Vec<ActionMenuItem> = Vec::new();
        if show_add_child {
            items.push(
                ActionMenuItem::new(
                    "plus",
                    move_tr!("tags-add-subtag"),
                    Callback::new(move |_: ()| adding_child.set(true)),
                )
                .testid("tag-add-child-btn"),
            );
        }
        if show_rename {
            items.push(
                ActionMenuItem::new(
                    "pencil",
                    move_tr!("tags-rename"),
                    Callback::new(move |_: ()| {
                        rename_value.set(tag.with_value(|t| t.name.clone()));
                        renaming.set(true);
                    }),
                )
                .testid("tag-rename-btn"),
            );
        }
        if show_delete {
            items.push(
                ActionMenuItem::new(
                    "trash",
                    move_tr!("tags-delete-button"),
                    Callback::new(move |_: ()| on_delete.run(id_for_delete.clone())),
                )
                .danger()
                .testid("tag-delete-btn"),
            );
        }
        items
    });
    let show_menu = show_add_child || show_rename || show_delete;
    let children_for_view = children;
    let usage_for_children = usage.clone();

    view! {
        <div data-testid="tag-item-group">
            <div
                data-testid="tag-item"
                class="group flex items-center gap-1.5 rounded-md py-1 pr-1 hover:bg-base-200/50"
            >
                {if has_children {
                    view! {
                        <button
                            type="button"
                            class="btn btn-ghost btn-xs btn-square shrink-0"
                            data-testid="tag-toggle-btn"
                            aria-label=move || toggle_label.get()
                            on:click=on_toggle
                        >
                            <Icon name=chevron size=14 />
                        </button>
                    }
                        .into_any()
                } else {
                    view! { <span class="w-6 shrink-0"></span> }.into_any()
                }}

                {move || {
                    if renaming.get() {
                        return view! {
                            <input
                                type="text"
                                class="input input-xs input-bordered flex-1 min-w-0"
                                data-testid="tag-rename-input"
                                prop:value=move || rename_value.get()
                                on:input=move |ev| rename_value.set(event_target_value(&ev))
                                on:keydown=move |ev| match ev.key().as_str() {
                                    "Enter" => submit_rename.run(()),
                                    "Escape" => renaming.set(false),
                                    _ => {}
                                }
                                autofocus=true
                            />
                        }
                            .into_any();
                    }
                    let href = format!("/tags/{id_for_href}");
                    let badge = if linkable {
                        view! {
                            <A
                                href=href
                                attr:class="no-underline min-w-0"
                                attr:data-testid="tag-link"
                            >
                                <TagBadge tag=tag.get_value() />
                            </A>
                        }
                            .into_any()
                    } else {
                        view! { <TagBadge tag=tag.get_value() /> }.into_any()
                    };
                    view! {
                        <div class="flex min-w-0 flex-1 items-center gap-1.5">
                            {badge}
                            {show_type_chip.then(|| view! {
                                <Chip class="shrink-0">{move || type_label.get()}</Chip>
                            })}
                        </div>
                    }
                        .into_any()
                }}

                <div class="ml-auto flex shrink-0 items-center gap-1">
                    {move || {
                        (usage_count > 0).then(|| view! {
                            <span
                                class="text-xs tabular-nums text-base-content/40"
                                data-testid="tag-usage-count"
                                title=move || i18n.tr("tags-usage-title")
                            >
                                {usage_count}
                            </span>
                        })
                    }}
                    {show_menu.then(|| view! {
                        // Visible on hover, or while it has focus (keyboard/touch).
                        <div class="opacity-0 transition-opacity focus-within:opacity-100 group-hover:opacity-100">
                            <ActionMenu
                                items=menu_items.get_value()
                                open=menu_open
                                label=move_tr!("common-actions")
                                testid=Some("tag-actions-btn")
                            />
                        </div>
                    })}
                </div>
            </div>

            {move || {
                adding_child.get().then(|| view! {
                    <div class="flex items-center gap-2 py-1 pl-6">
                        <input
                            type="text"
                            class="input input-bordered input-xs flex-1"
                            placeholder=move || i18n.tr("tags-subtag-placeholder")
                            data-testid="tag-child-input"
                            prop:value=move || new_child_name.get()
                            on:input=move |ev| new_child_name.set(event_target_value(&ev))
                            on:keydown=move |ev| match ev.key().as_str() {
                                "Enter" => submit_child.run(()),
                                "Escape" => {
                                    new_child_name.set(String::new());
                                    adding_child.set(false);
                                }
                                _ => {}
                            }
                            autofocus=true
                        />
                        <button
                            type="button"
                            class="btn btn-primary btn-xs"
                            on:click=move |_| submit_child.run(())
                        >
                            {move_tr!("common-add")}
                        </button>
                        <IconButton
                            icon="x"
                            label=move_tr!("common-cancel")
                            on_click=Callback::new(move |_| {
                                new_child_name.set(String::new());
                                adding_child.set(false);
                            })
                        />
                    </div>
                })
            }}

            {move || {
                if children_for_view.is_empty() || is_collapsed.get() {
                    return view! {}.into_any();
                }
                view! {
                    <div class="ml-2 border-l border-base-300/60 pl-2">
                        {children_for_view
                            .clone()
                            .into_iter()
                            .map(|child| {
                                view! {
                                    <TagTreeRow
                                        node=child
                                        collapsed=collapsed
                                        usage=usage_for_children.clone()
                                        on_create_child=on_create_child
                                        on_rename=on_rename
                                        on_delete=on_delete
                                        show_add_child=show_add_child
                                        show_rename=show_rename
                                        show_delete=show_delete
                                        linkable=linkable
                                    />
                                }
                            })
                            .collect_view()}
                    </div>
                }
                    .into_any()
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tag(id: &str, name: &str, parent: Option<&str>) -> Tag {
        Tag {
            id: id.to_string(),
            user_id: "u1".to_string(),
            name: name.to_string(),
            icon: None,
            color: None,
            parent_tag_id: parent.map(String::from),
            tag_type: "tag".to_string(),
            metadata: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn build_tag_tree_nests_children_under_parents() {
        let tags = vec![
            tag("root", "Root", None),
            tag("child1", "Child 1", Some("root")),
            tag("child2", "Child 2", Some("root")),
            tag("grandchild", "Grand", Some("child1")),
            tag("other", "Other", None),
        ];
        let tree = build_tag_tree(&tags);
        assert_eq!(tree.len(), 2, "two roots expected");
        let root_node = tree.iter().find(|n| n.tag.id == "root").unwrap();
        assert_eq!(root_node.children.len(), 2);
        let c1 = root_node
            .children
            .iter()
            .find(|n| n.tag.id == "child1")
            .unwrap();
        assert_eq!(c1.children.len(), 1);
        assert_eq!(c1.children[0].tag.id, "grandchild");
    }

    #[test]
    fn build_tag_tree_promotes_orphans_to_root() {
        let tags = vec![tag("a", "A", Some("missing_parent")), tag("b", "B", None)];
        let tree = build_tag_tree(&tags);
        assert_eq!(tree.len(), 2);
    }

    #[test]
    fn build_tag_tree_sorts_siblings_alphabetically() {
        let tags = vec![
            tag("c", "Charlie", None),
            tag("a", "Alpha", None),
            tag("b", "Bravo", None),
        ];
        let tree = build_tag_tree(&tags);
        let names: Vec<_> = tree.iter().map(|n| n.tag.name.as_str()).collect();
        assert_eq!(names, vec!["Alpha", "Bravo", "Charlie"]);
    }

    #[test]
    fn build_breadcrumb_returns_path_root_first() {
        let tags = vec![
            tag("root", "Root", None),
            tag("mid", "Mid", Some("root")),
            tag("leaf", "Leaf", Some("mid")),
        ];
        let path = build_breadcrumb(&tags, "leaf");
        let ids: Vec<_> = path.iter().map(|t| t.id.as_str()).collect();
        assert_eq!(ids, vec!["root", "mid", "leaf"]);
    }

    #[test]
    fn build_breadcrumb_handles_cycle_without_panic() {
        let tags = vec![tag("a", "A", Some("b")), tag("b", "B", Some("a"))];
        let path = build_breadcrumb(&tags, "a");
        assert!(path.len() <= 2);
    }

    #[test]
    fn get_descendant_ids_returns_all_descendants_not_self() {
        let tags = vec![
            tag("root", "Root", None),
            tag("c1", "C1", Some("root")),
            tag("c2", "C2", Some("root")),
            tag("gc", "GC", Some("c1")),
        ];
        let mut descendants = get_descendant_ids(&tags, "root");
        descendants.sort();
        assert_eq!(descendants, vec!["c1", "c2", "gc"]);
        assert!(!descendants.contains(&"root".to_string()));
    }

    #[test]
    fn build_subtree_returns_direct_children_as_roots() {
        let tags = vec![
            tag("root", "Root", None),
            tag("c1", "C1", Some("root")),
            tag("gc", "GC", Some("c1")),
            tag("unrelated", "U", None),
        ];
        let subtree = build_subtree(&tags, "root");
        assert_eq!(subtree.len(), 1);
        assert_eq!(subtree[0].tag.id, "c1");
        assert_eq!(subtree[0].children.len(), 1);
        assert_eq!(subtree[0].children[0].tag.id, "gc");
    }

    fn ids(tags: &[Tag]) -> Vec<&str> {
        tags.iter().map(|t| t.id.as_str()).collect()
    }

    #[test]
    fn filter_tags_keeps_matches_and_their_ancestors() {
        let tags = vec![
            tag("hobby", "Hobby", None),
            tag("garden", "Garden", Some("hobby")),
            tag("work", "Work", None),
            tag("trips", "Trips", None),
        ];
        let filtered = filter_tags(&tags, "gard");
        assert_eq!(ids(&filtered), vec!["hobby", "garden"]);
    }

    #[test]
    fn filter_tags_keeps_deep_ancestor_chain() {
        let tags = vec![
            tag("root", "Root", None),
            tag("mid", "Mid", Some("root")),
            tag("leaf", "Leaf things", Some("mid")),
        ];
        let filtered = filter_tags(&tags, "LEAF");
        assert_eq!(ids(&filtered), vec!["root", "mid", "leaf"]);
    }

    #[test]
    fn filter_tags_matches_the_tag_itself_plus_descendant_rows_only() {
        // Matching a parent must not drag its children in: the tree is rebuilt
        // from the filtered list and children are absent unless they match too.
        let tags = vec![
            tag("hobby", "Hobby", None),
            tag("garden", "Garden", Some("hobby")),
        ];
        let filtered = filter_tags(&tags, "hobby");
        assert_eq!(ids(&filtered), vec!["hobby"]);
        let tree = build_tag_tree(&filtered);
        assert_eq!(tree.len(), 1);
        assert!(tree[0].children.is_empty());
    }

    #[test]
    fn filter_tags_blank_query_returns_everything() {
        let tags = vec![tag("a", "A", None), tag("b", "B", None)];
        assert_eq!(filter_tags(&tags, "   ").len(), 2);
    }

    #[test]
    fn filter_tags_without_match_returns_empty() {
        let tags = vec![tag("a", "A", None)];
        assert!(filter_tags(&tags, "zzz").is_empty());
    }

    #[test]
    fn tag_type_keys_cover_known_types_and_default() {
        assert_eq!(tag_type_key("priority"), "tags-type-priority");
        assert_eq!(tag_type_key("country"), "tags-type-country");
        assert_eq!(tag_type_key("city"), "tags-type-city");
        assert_eq!(tag_type_key("address"), "tags-type-address");
        assert_eq!(tag_type_key("tag"), "tags-type-tag");
        assert_eq!(tag_type_key("something-new"), "tags-type-tag");
    }
}
