use kartoteka_shared::types::Tag;
use leptos::prelude::*;
use leptos_router::components::A;
use std::collections::{HashMap, HashSet};

use crate::components::tags::tag_badge::TagBadge;

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

    roots.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    for children in children_map.values_mut() {
        children.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
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

/// Recursive row that renders a tag and its children, indented by `depth`.
/// Callbacks are delegated to the hosting page so tree rows stay stateless about persistence.
/// `show_add_child` / `show_delete` flags control which action buttons appear — hiding either
/// button suppresses the corresponding callback, so callers can pass noop callbacks when unused.
#[component]
pub fn TagTreeRow(
    node: TagNode,
    depth: usize,
    #[prop(default = Callback::new(|_| {}))] on_create_child: Callback<(String, String)>,
    #[prop(default = Callback::new(|_| {}))] on_delete: Callback<String>,
    #[prop(default = true)] show_add_child: bool,
    #[prop(default = true)] show_delete: bool,
    #[prop(default = true)] linkable: bool,
) -> impl IntoView {
    let tag = node.tag;
    let children = node.children;
    let tag_id_for_link = tag.id.clone();
    let tag_id_for_add = tag.id.clone();
    let tag_id_for_delete = tag.id.clone();
    let padding = format!("padding-left: {}rem;", depth as f64 * 1.0);
    let adding_child = RwSignal::new(false);
    let new_child_name = RwSignal::new(String::new());

    let parent_id_for_submit = StoredValue::new(tag_id_for_add.clone());
    let on_submit_child: Callback<()> = Callback::new(move |_| {
        let name = new_child_name.get_untracked();
        if name.trim().is_empty() {
            return;
        }
        on_create_child.run((parent_id_for_submit.get_value(), name));
        new_child_name.set(String::new());
        adding_child.set(false);
    });

    let badge = if linkable {
        view! {
            <A href=format!("/tags/{tag_id_for_link}") attr:class="no-underline">
                <TagBadge tag=tag.clone() />
            </A>
        }
        .into_any()
    } else {
        view! { <TagBadge tag=tag.clone() /> }.into_any()
    };

    view! {
        <div>
            <div class="flex items-center gap-1 py-1" style=padding.clone()>
                {badge}
                {show_add_child.then(|| view! {
                    <button
                        type="button"
                        class="btn btn-ghost btn-xs btn-square"
                        title="Dodaj podtag"
                        data-testid="tag-add-child-btn"
                        on:click=move |_| adding_child.update(|v| *v = !*v)
                    >"+"</button>
                })}
                {show_delete.then(|| {
                    let tid = tag_id_for_delete.clone();
                    view! {
                        <button
                            type="button"
                            class="btn btn-ghost btn-xs btn-square text-error"
                            title="Usuń"
                            data-testid="tag-delete-btn"
                            on:click=move |_| on_delete.run(tid.clone())
                        >"✕"</button>
                    }
                })}
            </div>

            {move || {
                if adding_child.get() {
                    let child_padding = format!("padding-left: {}rem;", (depth + 1) as f64 * 1.0);
                    view! {
                        <div class="flex gap-2 items-center py-1" style=child_padding>
                            <input
                                type="text"
                                class="input input-bordered input-xs flex-1"
                                placeholder="Nazwa podtagu"
                                data-testid="tag-child-input"
                                prop:value=move || new_child_name.get()
                                on:input=move |ev| new_child_name.set(event_target_value(&ev))
                                on:keydown=move |ev| {
                                    match ev.key().as_str() {
                                        "Enter" => on_submit_child.run(()),
                                        "Escape" => {
                                            new_child_name.set(String::new());
                                            adding_child.set(false);
                                        }
                                        _ => {}
                                    }
                                }
                                autofocus=true
                            />
                            <button
                                type="button"
                                class="btn btn-primary btn-xs"
                                on:click=move |_| on_submit_child.run(())
                            >"Dodaj"</button>
                            <button
                                type="button"
                                class="btn btn-ghost btn-xs"
                                on:click=move |_| {
                                    new_child_name.set(String::new());
                                    adding_child.set(false);
                                }
                            >"✕"</button>
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}

            {children.into_iter().map(|child| {
                view! {
                    <TagTreeRow
                        node=child
                        depth=depth + 1
                        on_create_child=on_create_child
                        on_delete=on_delete
                        show_add_child=show_add_child
                        show_delete=show_delete
                        linkable=linkable
                    />
                }
            }).collect_view()}
        </div>
    }
    .into_any()
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
}
