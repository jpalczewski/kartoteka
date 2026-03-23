use kartoteka_shared::Tag;
use leptos::prelude::*;

#[component]
pub fn TagBadge(tag: Tag, #[prop(optional)] on_remove: Option<Callback<String>>) -> impl IntoView {
    let tag_id = tag.id.clone();
    view! {
        <span class="tag-badge" style=format!("background: {}; color: white;", tag.color)>
            {tag.name.clone()}
            {if let Some(on_remove) = on_remove {
                let tid = tag_id.clone();
                view! {
                    <button class="tag-remove" on:click=move |_| on_remove.run(tid.clone())>"×"</button>
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </span>
    }
}
