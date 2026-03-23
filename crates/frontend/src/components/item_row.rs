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
    let class = if item.completed {
        "item-row completed"
    } else {
        "item-row"
    };

    view! {
        <div class=class>
            <input
                type="checkbox"
                checked=item.completed
                on:change=move |_| on_toggle.run(id_toggle.clone())
            />
            <span class="item-title" style="flex: 1;">{item.title}</span>
            // Tag badges for this item
            {if !item_tag_ids.is_empty() {
                let item_tags: Vec<Tag> = all_tags.iter()
                    .filter(|t| item_tag_ids.contains(&t.id))
                    .cloned()
                    .collect();
                view! {
                    <div class="tag-list">
                        {item_tags.into_iter().map(|t| view! { <TagBadge tag=t/> }).collect::<Vec<_>>()}
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
                class="btn"
                style="padding: 0.25rem 0.5rem; font-size: 0.8rem;"
                on:click=move |_| on_delete.run(id_delete.clone())
            >
                "X"
            </button>
        </div>
    }
}
