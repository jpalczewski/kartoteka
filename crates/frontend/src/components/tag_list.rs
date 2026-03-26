use kartoteka_shared::Tag;
use leptos::prelude::*;

use super::tag_badge::TagBadge;
use super::tag_selector::TagSelector;

/// Shared component for rendering a list of tag badges with optional tag selector.
/// Replaces duplicated tag rendering pattern in ItemRow, DateItemRow, and ListCard.
#[component]
pub fn TagList(
    all_tags: Vec<Tag>,
    selected_tag_ids: Vec<String>,
    on_toggle: Option<Callback<String>>,
) -> impl IntoView {
    let item_tags: Vec<Tag> = all_tags
        .iter()
        .filter(|t| selected_tag_ids.contains(&t.id))
        .cloned()
        .collect();

    let has_content = !item_tags.is_empty() || on_toggle.is_some();

    if !has_content {
        return view! {}.into_any();
    }

    view! {
        <div class="flex flex-wrap items-center gap-1">
            {item_tags.into_iter().map(|t| {
                match on_toggle {
                    Some(cb) => view! { <TagBadge tag=t on_remove=cb all_tags=all_tags.clone()/> }.into_any(),
                    None => view! { <TagBadge tag=t all_tags=all_tags.clone()/> }.into_any(),
                }
            }).collect::<Vec<_>>()}
            {if let Some(toggle_cb) = on_toggle {
                view! {
                    <TagSelector
                        all_tags=all_tags.clone()
                        selected_tag_ids=selected_tag_ids.clone()
                        on_toggle=toggle_cb
                    />
                }.into_any()
            } else {
                view! {}.into_any()
            }}
        </div>
    }.into_any()
}
