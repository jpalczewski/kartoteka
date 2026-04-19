use kartoteka_shared::types::Tag;
use leptos::prelude::*;

#[component]
pub fn TagBadge(
    tag: Tag,
    #[prop(optional)] on_click: Option<Callback<String>>,
    #[prop(default = false)] active: bool,
) -> impl IntoView {
    let color = tag.color.clone().unwrap_or_else(|| "#6b7280".to_string());
    let style = format!("background: {color}; color: white;");
    let tag_id = tag.id.clone();
    let name = tag.name.clone();

    view! {
        <span
            class=move || {
                if active { "tag-badge badge cursor-pointer ring-2 ring-offset-1 ring-white" }
                else { "tag-badge badge cursor-pointer" }
            }
            style=style
            on:click=move |ev| {
                if let Some(cb) = on_click.clone() {
                    ev.stop_propagation();
                    cb.run(tag_id.clone());
                }
            }
        >
            {name}
        </span>
    }
}
