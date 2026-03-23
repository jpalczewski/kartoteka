use kartoteka_shared::Tag;
use leptos::prelude::*;
use std::time::Duration;

#[component]
pub fn TagBadge(tag: Tag, #[prop(optional)] on_remove: Option<Callback<String>>) -> impl IntoView {
    let tag_id = tag.id.clone();
    let confirming = RwSignal::new(false);

    let handle_remove_click = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        if confirming.get() {
            if let Some(cb) = on_remove {
                cb.run(tag_id.clone());
            }
            confirming.set(false);
        } else {
            confirming.set(true);
            set_timeout(move || confirming.set(false), Duration::from_millis(2500));
        }
    };

    view! {
        <span
            class=move || if confirming.get() { "tag-badge confirming" } else { "tag-badge" }
            style=format!("background: {}; color: white;", tag.color)
        >
            {tag.name.clone()}
            {if on_remove.is_some() {
                view! {
                    <button
                        type="button"
                        class="tag-remove"
                        aria-label="Usuń tag"
                        on:click=handle_remove_click
                    >
                        "×"
                    </button>
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </span>
    }
}
