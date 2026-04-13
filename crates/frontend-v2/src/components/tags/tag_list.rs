use kartoteka_shared::types::Tag;
use leptos::prelude::*;

use super::tag_badge::TagBadge;

/// Renders a row of tag badges for a list.
#[component]
pub fn TagList(
    all_tags: Vec<Tag>,
    selected_tag_ids: Vec<String>,
    #[prop(optional)] on_toggle: Option<Callback<String>>,
) -> impl IntoView {
    let selected_tags: Vec<Tag> = all_tags
        .iter()
        .filter(|t| selected_tag_ids.contains(&t.id))
        .cloned()
        .collect();

    if selected_tags.is_empty() && on_toggle.is_none() {
        return view! {}.into_any();
    }

    view! {
        <div class="flex flex-wrap items-center gap-1">
            {selected_tags.into_iter().map(|tag| {
                if let Some(cb) = on_toggle {
                    view! { <TagBadge tag=tag on_click=cb /> }.into_any()
                } else {
                    view! { <TagBadge tag=tag /> }.into_any()
                }
            }).collect::<Vec<_>>()}
            {on_toggle.map(|_cb| view! {
                <button type="button" class="btn btn-ghost btn-xs ">{"+"}</button>
            })}
        </div>
    }.into_any()
}
