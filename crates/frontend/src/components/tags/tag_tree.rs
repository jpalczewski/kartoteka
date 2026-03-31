use crate::api;
use crate::api::client::GlooClient;
use crate::components::add_input::AddInput;
use crate::components::tag_badge::TagBadge;
pub use crate::state::view_helpers::TagFilterOption;
use crate::state::view_helpers::build_tag_breadcrumb;
pub use crate::state::view_helpers::build_tag_filter_options;
use kartoteka_shared::{CreateTagRequest, Tag};
use leptos::prelude::*;
use leptos_fluent::move_tr;
use std::collections::HashMap;

/// A tag with its children, used for rendering tag trees.
#[derive(Clone, Debug)]
pub struct TagNode {
    pub tag: Tag,
    pub children: Vec<TagNode>,
}

/// Build a tree of TagNodes from a flat Vec<Tag>.
/// Returns only root nodes (tags with no parent or whose parent is not in the list).
/// Children are sorted alphabetically by name within each parent.
pub fn build_tag_tree(tags: &[Tag]) -> Vec<TagNode> {
    let tag_ids: std::collections::HashSet<&str> = tags.iter().map(|t| t.id.as_str()).collect();

    // Group children by parent_tag_id
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

    // Sort roots alphabetically
    roots.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    // Sort children within each parent
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

/// Walk ancestors of a tag (from tag up to root). Returns list from root to tag.
pub fn build_breadcrumb(tags: &[Tag], tag_id: &str) -> Vec<Tag> {
    build_tag_breadcrumb(tags, tag_id)
}

/// Get all descendant IDs of a tag (not including the tag itself).
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

/// Build a subtree rooted at a specific tag. Returns children of that tag as roots.
pub fn build_subtree(tags: &[Tag], root_id: &str) -> Vec<TagNode> {
    let descendant_ids = get_descendant_ids(tags, root_id);
    let subtags: Vec<Tag> = tags
        .iter()
        .filter(|t| descendant_ids.contains(&t.id))
        .cloned()
        .collect();
    // Build tree from descendants only — direct children of root_id become roots
    build_tag_tree(&subtags)
}

/// Shared tree row component for displaying tags in a tree.
/// Used by both the tags list page and the tag detail page subtree.
#[component]
pub fn TagTreeRow(
    node: TagNode,
    depth: usize,
    tags: RwSignal<Vec<Tag>>,
    new_color: ReadSignal<String>,
    #[prop(default = true)] show_add_child: bool,
    #[prop(default = true)] show_delete: bool,
) -> impl IntoView {
    let client = use_context::<GlooClient>().expect("GlooClient not provided");
    let tag = node.tag;
    let children = node.children;
    let tid_add = tag.id.clone();
    let tid_delete = tag.id.clone();
    let padding = format!("padding-left: {}rem;", depth as f64 * 1.0);
    let adding_child = RwSignal::new(false);

    view! {
        <div>
            <div class="flex items-center gap-1 py-1" style=padding.clone()>
                <TagBadge tag=tag.clone() />
                {show_add_child.then(|| {
                    view! {
                        <button
                            class="btn btn-ghost btn-xs btn-square"
                            title="Dodaj podtag"
                            on:click=move |_| adding_child.update(|v| *v = !*v)
                        >"+"</button>
                    }
                })}
                {show_delete.then(|| {
                    let tid = tid_delete.clone();
                    let client_del = client.clone();
                    view! {
                        <button
                            class="btn btn-error btn-xs btn-square"
                            title=move_tr!("common-delete")
                            on:click=move |_| {
                                tags.update(|t| t.retain(|tag| tag.id != tid));
                                let tid = tid.clone();
                                let client_del = client_del.clone();
                                leptos::task::spawn_local(async move {
                                    let _ = api::delete_tag(&client_del, &tid).await;
                                });
                            }
                        >"\u{2715}"</button>
                    }
                })}
            </div>

            // Inline add-child form
            {move || {
                if adding_child.get() {
                    let tid_create = tid_add.clone();
                    let client_create = client.clone();
                    let on_submit = Callback::new(move |name: String| {
                        let color = new_color.get_untracked();
                        let parent_id = tid_create.clone();
                        let client_c = client_create.clone();
                        leptos::task::spawn_local(async move {
                            let req = CreateTagRequest {
                                name,
                                color: Some(color),
                                parent_tag_id: Some(parent_id),
                            };
                            if let Ok(tag) = api::create_tag(&client_c, &req).await {
                                tags.update(|t| t.push(tag));
                            }
                            adding_child.set(false);
                        });
                    });
                    let child_padding = format!("padding-left: {}rem;", (depth + 1) as f64 * 1.0);
                    view! {
                        <div class="flex gap-2 items-center py-1" style=child_padding>
                            <AddInput placeholder=move_tr!("tags-new-placeholder") button_label=move_tr!("common-add") on_submit=on_submit />
                            <button class="btn btn-ghost btn-xs" on:click=move |_| adding_child.set(false)>"\u{2715}"</button>
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}

            // Children
            {children.into_iter().map(|child| {
                view! {
                    <TagTreeRow
                        node=child
                        depth=depth + 1
                        tags=tags
                        new_color=new_color
                        show_add_child=show_add_child
                        show_delete=show_delete
                    />
                }
            }).collect_view()}
        </div>
    }
    .into_any()
}
