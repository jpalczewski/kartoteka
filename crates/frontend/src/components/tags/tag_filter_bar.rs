use kartoteka_shared::types::{ItemTagLink, Tag};
use leptos::prelude::*;
use std::collections::HashSet;

/// Filter bar shown above the item list. Displays only tags that are actually used in the list.
/// Clicking a tag filters to items with that tag; clicking "Wszystkie" clears the filter.
#[component]
pub fn TagFilterBar(
    all_tags: Vec<Tag>,
    item_tag_links: Vec<ItemTagLink>,
    active_tag: RwSignal<Option<String>>,
) -> impl IntoView {
    let used_tag_ids: HashSet<String> = item_tag_links.iter().map(|l| l.tag_id.clone()).collect();
    let used_tags: Vec<Tag> = all_tags
        .into_iter()
        .filter(|t| used_tag_ids.contains(&t.id))
        .collect();

    if used_tags.is_empty() {
        return view! {}.into_any();
    }

    view! {
        <div class="flex flex-wrap items-center gap-1 mb-3">
            <button
                type="button"
                class=move || {
                    if active_tag.get().is_none() { "btn btn-xs btn-primary" }
                    else { "btn btn-xs btn-ghost" }
                }
                on:click=move |_| active_tag.set(None)
            >
                "Wszystkie"
            </button>
            {used_tags.into_iter().map(|tag| {
                let tid_class = tag.id.clone();
                let tid_click = tag.id.clone();
                let color = tag.color.clone().unwrap_or_else(|| "#6b7280".to_string());
                let name = tag.name.clone();
                view! {
                    <button
                        type="button"
                        class=move || {
                            if active_tag.get().as_deref() == Some(tid_class.as_str()) {
                                "btn btn-xs ring-2 ring-offset-1 text-white"
                            } else {
                                "btn btn-xs text-white opacity-80 hover:opacity-100"
                            }
                        }
                        style=format!("background:{color};border-color:{color};")
                        on:click=move |_| {
                            let id = tid_click.clone();
                            active_tag.update(|cur| {
                                if cur.as_deref() == Some(id.as_str()) { *cur = None; }
                                else { *cur = Some(id); }
                            });
                        }
                    >
                        {name}
                    </button>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
    .into_any()
}
