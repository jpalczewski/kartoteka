use leptos::prelude::*;

use crate::components::tags::tag_list::TagList;
use kartoteka_shared::Tag;

/// Displays assigned tags as badges with remove, plus a tag selector to add more.
#[component]
pub fn ListTagBar(
    all_tags: Vec<Tag>,
    assigned_tag_ids: Vec<String>,
    on_toggle: Callback<String>,
) -> impl IntoView {
    view! {
        <div class="mb-3">
            <TagList
                all_tags=all_tags
                selected_tag_ids=assigned_tag_ids
                on_toggle=Some(on_toggle)
            />
        </div>
    }
}
