use kartoteka_shared::types::Tag;
use leptos::prelude::*;

use super::tag_badge::TagBadge;

/// Renders a row of tag badges for a list.
/// If `on_toggle` is provided, badges are clickable (removes tag) and a "+" dropdown
/// lets the user assign any unassigned tag.
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

    let unassigned_tags: Vec<Tag> = all_tags
        .into_iter()
        .filter(|t| !selected_tag_ids.contains(&t.id))
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
            {on_toggle.map(|cb| {
                if unassigned_tags.is_empty() {
                    return view! {}.into_any();
                }
                view! {
                    <div class="dropdown" on:click=move |ev| ev.stop_propagation()>
                        <div
                            tabindex="0"
                            role="button"
                            class="btn btn-ghost btn-xs"
                            data-testid="tag-add-btn"
                        >
                            "+"
                        </div>
                        <ul
                            tabindex="0"
                            class="dropdown-content menu bg-base-100 rounded-box z-50 w-40 p-1 shadow-lg border border-base-300"
                        >
                            {unassigned_tags.into_iter().map(|tag| {
                                let tid = tag.id.clone();
                                let color = tag.color.clone().unwrap_or_else(|| "#6366f1".to_string());
                                view! {
                                    <li>
                                        <button
                                            type="button"
                                            class="flex items-center gap-2 text-sm"
                                            data-testid="tag-dropdown-option"
                                            on:click=move |_| cb.run(tid.clone())
                                        >
                                            <span
                                                class="w-3 h-3 rounded-full inline-block shrink-0"
                                                style=format!("background:{color}")
                                            ></span>
                                            {tag.name.clone()}
                                        </button>
                                    </li>
                                }
                            }).collect::<Vec<_>>()}
                        </ul>
                    </div>
                }.into_any()
            })}
        </div>
    }
    .into_any()
}
