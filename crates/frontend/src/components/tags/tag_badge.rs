use crate::components::tag_tree::build_breadcrumb;
use kartoteka_shared::Tag;
use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::components::A;
use std::time::Duration;

#[component]
pub fn TagBadge(
    tag: Tag,
    #[prop(optional)] on_remove: Option<Callback<String>>,
    #[prop(default = vec![])] all_tags: Vec<Tag>,
) -> impl IntoView {
    let tag_id = tag.id.clone();
    let confirming = RwSignal::new(false);

    let handle_remove_click = move |ev: web_sys::MouseEvent| {
        ev.stop_propagation();
        ev.prevent_default();
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

    let color = tag.color.clone();
    let tag_id_link = tag.id.clone();

    // Build breadcrumb path if all_tags is provided
    let breadcrumb = if !all_tags.is_empty() {
        build_breadcrumb(&all_tags, &tag.id)
    } else {
        vec![tag.clone()]
    };

    let has_path = breadcrumb.len() > 1 || !all_tags.is_empty();

    view! {
        <span
            class=move || if confirming.get() { "tag-badge confirming" } else { "tag-badge" }
            style=format!("background: {}; color: white;", color)
        >
            {if has_path {
                // Clickable path: each segment links to its tag detail page
                breadcrumb.iter().enumerate().map(|(i, bt)| {
                    let bt_id = bt.id.clone();
                    let bt_name = bt.name.clone();
                    let is_last = i == breadcrumb.len() - 1;
                    view! {
                        <A href=format!("/tags/{bt_id}") attr:class="tag-badge-link" attr:style="color: inherit; text-decoration: none;">
                            {bt_name}
                        </A>
                        {if !is_last {
                            view! { <span class="tag-badge-sep">" › "</span> }.into_any()
                        } else {
                            view! {}.into_any()
                        }}
                    }
                }).collect_view().into_any()
            } else {
                // Simple: just tag name, link to detail
                view! {
                    <A href=format!("/tags/{tag_id_link}") attr:class="tag-badge-link" attr:style="color: inherit; text-decoration: none;">
                        {tag.name.clone()}
                    </A>
                }.into_any()
            }}
            {if on_remove.is_some() {
                view! {
                    <button
                        type="button"
                        class="tag-remove"
                        aria-label=move_tr!("tags-remove-aria")
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
