use crate::components::tag_tree::{build_tag_tree, TagNode};
use kartoteka_shared::Tag;
use leptos::prelude::*;
use std::collections::HashMap;

#[component]
pub fn TagSelector(
    all_tags: Vec<Tag>,
    selected_tag_ids: Vec<String>,
    on_toggle: Callback<String>,
) -> impl IntoView {
    let (open, set_open) = signal(false);
    let expanded = RwSignal::new(HashMap::<String, bool>::new());
    let tree = build_tag_tree(&all_tags);

    view! {
        <div class="relative">
            <button type="button" class="btn btn-ghost btn-xs btn-square" on:click=move |_| set_open.update(|v| *v = !*v)>
                "＋"
            </button>
            <div
                class="absolute right-0 top-full mt-1 bg-base-200 border border-base-300 rounded-box min-w-44 max-h-60 overflow-y-auto z-50 p-2 shadow-lg"
                style:display=move || if open.get() { "block" } else { "none" }
            >
                {tree.into_iter().map(|node| {
                    view! {
                        <TagSelectorNode
                            node=node
                            depth=0
                            selected_tag_ids=selected_tag_ids.clone()
                            on_toggle=on_toggle
                            expanded=expanded
                        />
                    }
                }).collect_view()}
            </div>
        </div>
    }
}

#[component]
fn TagSelectorNode(
    node: TagNode,
    depth: usize,
    selected_tag_ids: Vec<String>,
    on_toggle: Callback<String>,
    expanded: RwSignal<HashMap<String, bool>>,
) -> impl IntoView {
    let tag = node.tag;
    let children = node.children;
    let has_children = !children.is_empty();
    let is_selected = selected_tag_ids.contains(&tag.id);
    let color = tag.color.clone();
    let name = tag.name.clone();
    let tid = tag.id.clone();
    let tid_toggle = tag.id.clone();
    let tid_expand = tag.id.clone();
    let margin = format!("margin-left: {}rem;", depth as f64 * 0.75);

    view! {
        <div style=margin>
            <label
                class="flex items-center gap-1.5 px-2 py-1.5 text-sm rounded cursor-pointer hover:bg-base-300"
                style=format!("border-left: 3px solid {color};")
            >
                {has_children.then(|| {
                    let tid_e = tid_expand.clone();
                    view! {
                        <button
                            class="btn btn-ghost btn-xs btn-square p-0 min-h-0 h-4 w-4"
                            on:click=move |ev| {
                                ev.prevent_default();
                                ev.stop_propagation();
                                expanded.update(|m| {
                                    let v = m.entry(tid_e.clone()).or_insert(false);
                                    *v = !*v;
                                });
                            }
                        >
                            {move || {
                                let is_expanded = expanded.get().get(&tid_expand).copied().unwrap_or(false);
                                if is_expanded { "▼" } else { "▶" }
                            }}
                        </button>
                    }
                })}
                <input
                    type="checkbox"
                    class="checkbox checkbox-secondary checkbox-xs"
                    checked=is_selected
                    on:change=move |_| on_toggle.run(tid_toggle.clone())
                />
                {name}
            </label>
            {move || {
                let is_expanded = expanded.get().get(&tid).copied().unwrap_or(false);
                if is_expanded && has_children {
                    children.clone().into_iter().map(|child| {
                        view! {
                            <TagSelectorNode
                                node=child
                                depth=depth + 1
                                selected_tag_ids=selected_tag_ids.clone()
                                on_toggle=on_toggle
                                expanded=expanded
                            />
                        }
                    }).collect_view().into_any()
                } else {
                    view! {}.into_any()
                }
            }}
        </div>
    }
}
