use leptos::prelude::*;

use super::tag_badge::TagBadge;
use super::tag_selector::TagSelector;
use kartoteka_shared::Tag;

/// Displays assigned tags as badges with remove, plus a tag selector to add more.
#[component]
pub fn ListTagBar(
    all_tags: Vec<Tag>,
    assigned_tag_ids: Vec<String>,
    on_toggle: Callback<String>,
) -> impl IntoView {
    let assigned: Vec<Tag> = all_tags
        .iter()
        .filter(|t| assigned_tag_ids.contains(&t.id))
        .cloned()
        .collect();

    view! {
        <div class="flex flex-wrap items-center gap-1 mb-3">
            {assigned.into_iter().map(|t| {
                let tid = t.id.clone();
                let cb = on_toggle;
                view! {
                    <TagBadge
                        tag=t
                        on_remove=Callback::new(move |_: String| cb.run(tid.clone()))
                    />
                }
            }).collect::<Vec<_>>()}
            {if !all_tags.is_empty() {
                view! {
                    <TagSelector
                        all_tags=all_tags
                        selected_tag_ids=assigned_tag_ids
                        on_toggle=on_toggle
                    />
                }.into_any()
            } else {
                ().into_any()
            }}
        </div>
    }
}
