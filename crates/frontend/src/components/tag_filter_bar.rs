use leptos::prelude::*;

use kartoteka_shared::Tag;

/// A bar of tag filter chips. One "Wszystkie" button + one per tag.
#[component]
pub fn TagFilterBar(
    tags: Vec<Tag>,
    active_tag_id: ReadSignal<Option<String>>,
    on_select: WriteSignal<Option<String>>,
) -> impl IntoView {
    if tags.is_empty() {
        return ().into_any();
    }

    view! {
        <div class="flex flex-wrap gap-1 mb-3">
            <button
                class=move || if active_tag_id.get().is_none() { "btn btn-xs btn-primary" } else { "btn btn-xs btn-ghost" }
                on:click=move |_| on_select.set(None)
            >"Wszystkie"</button>
            {tags.into_iter().map(|t| {
                let tid_class = t.id.clone();
                let tid_style = t.id.clone();
                let tid_click = t.id.clone();
                let tname = t.name.clone();
                let tcolor_class = t.color.clone();
                let tcolor_style = t.color.clone();
                view! {
                    <button
                        class=move || if active_tag_id.get().as_deref() == Some(&tid_class) { "btn btn-xs btn-primary" } else { "btn btn-xs btn-outline" }
                        style=move || format!("border-color: {}; color: {}", tcolor_style, if active_tag_id.get().as_deref() == Some(&tid_style) { "#fff" } else { &tcolor_class })
                        on:click=move |_| on_select.set(Some(tid_click.clone()))
                    >{tname}</button>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
    .into_any()
}
