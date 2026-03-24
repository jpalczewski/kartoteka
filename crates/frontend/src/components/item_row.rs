use kartoteka_shared::{Item, Tag};
use leptos::prelude::*;

use super::tag_badge::TagBadge;
use super::tag_selector::TagSelector;

#[component]
pub fn ItemRow(
    item: Item,
    on_toggle: Callback<String>,
    on_delete: Callback<String>,
    #[prop(default = vec![])] all_tags: Vec<Tag>,
    #[prop(default = vec![])] item_tag_ids: Vec<String>,
    #[prop(optional)] on_tag_toggle: Option<Callback<String>>,
) -> impl IntoView {
    let id = item.id.clone();
    let id_toggle = id.clone();
    let id_delete = id.clone();
    let completed = item.completed;

    let row_class = if completed {
        "flex items-center gap-3 py-3 border-b border-base-300 opacity-50"
    } else {
        "flex items-center gap-3 py-3 border-b border-base-300"
    };

    let title_class = if completed {
        "flex-1 line-through text-base-content/50"
    } else {
        "flex-1"
    };

    view! {
        <div class=row_class>
            <input
                type="checkbox"
                class="checkbox checkbox-secondary"
                checked=item.completed
                on:change=move |_| on_toggle.run(id_toggle.clone())
            />
            <span class=title_class>{item.title}</span>
            // Tag badges for this item
            {if !item_tag_ids.is_empty() {
                let item_tags: Vec<Tag> = all_tags.iter()
                    .filter(|t| item_tag_ids.contains(&t.id))
                    .cloned()
                    .collect();
                view! {
                    <div class="tag-list">
                        {item_tags.into_iter().map(|t| {
                            match on_tag_toggle.clone() {
                                Some(cb) => view! { <TagBadge tag=t on_remove=cb/> }.into_any(),
                                None => view! { <TagBadge tag=t/> }.into_any(),
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }}
            // Tag selector
            {if let Some(toggle_cb) = on_tag_toggle.clone() {
                view! {
                    <TagSelector
                        all_tags=all_tags.clone()
                        selected_tag_ids=item_tag_ids.clone()
                        on_toggle=toggle_cb
                    />
                }.into_any()
            } else {
                view! {}.into_any()
            }}
            <button
                type="button"
                class="btn btn-error btn-sm btn-square"
                on:click=move |_| on_delete.run(id_delete.clone())
            >
                "✕"
            </button>
        </div>
    }
}
