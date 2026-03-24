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
    #[prop(optional)] on_description_save: Option<Callback<(String, String)>>,
) -> impl IntoView {
    let id = item.id.clone();
    let id_toggle = id.clone();
    let id_delete = id.clone();
    let completed = item.completed;

    let row_class = if completed {
        "flex items-center gap-3 py-3 opacity-50"
    } else {
        "flex items-center gap-3 py-3"
    };

    let title_class = if completed {
        "flex-1 line-through text-base-content/50"
    } else {
        "flex-1"
    };

    let expanded = RwSignal::new(false);
    let description_text = RwSignal::new(item.description.clone().unwrap_or_default());

    view! {
        <div class="border-b border-base-300">
            <div class=row_class>
                <input
                    type="checkbox"
                    class="checkbox checkbox-secondary"
                    checked=item.completed
                    on:change=move |_| on_toggle.run(id_toggle.clone())
                />
                <button
                    type="button"
                    aria-label="Rozwiń opis"
                    class="btn btn-ghost btn-xs btn-square"
                    on:click=move |_| expanded.update(|e| *e = !*e)
                >
                    {move || if expanded.get() { "▲" } else { "▼" }}
                </button>
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
            {move || {
                if expanded.get() {
                    let id_blur = id.clone();
                    view! {
                        <div class="px-10 pb-3 pt-1">
                            <textarea
                                class="textarea textarea-bordered w-full text-sm resize-none"
                                rows="3"
                                placeholder="Dodaj opis..."
                                prop:value=move || description_text.get()
                                on:input=move |ev| description_text.set(event_target_value(&ev))
                                on:blur=move |_| {
                                    if let Some(cb) = on_description_save {
                                        cb.run((id_blur.clone(), description_text.get()));
                                    }
                                }
                            />
                        </div>
                    }.into_any()
                } else {
                    let desc = description_text.get();
                    if desc.is_empty() {
                        view! {}.into_any()
                    } else {
                        view! {
                            <p class="px-10 pb-2 text-sm text-base-content/60">{desc}</p>
                        }.into_any()
                    }
                }
            }}
        </div>
    }
}
