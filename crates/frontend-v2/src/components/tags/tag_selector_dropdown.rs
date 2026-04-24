use kartoteka_shared::types::Tag;
use leptos::prelude::*;

/// Dropdown button for assigning/removing tags from an item.
/// Calls `on_toggle(tag_id)` — parent decides assign vs remove based on current state.
#[component]
pub fn TagSelectorDropdown(
    all_tags: Vec<Tag>,
    item_tags: Vec<Tag>,
    on_toggle: Callback<String>,
) -> impl IntoView {
    if all_tags.is_empty() {
        return view! {}.into_any();
    }

    let assigned_count = item_tags.len();

    view! {
        <div class="dropdown dropdown-end">
            <div
                tabindex="0"
                role="button"
                class="btn btn-xs btn-ghost"
                title="Zarządzaj tagami"
            >
                "🏷"
                {if assigned_count > 0 {
                    view! { <span class="badge badge-xs badge-primary ml-1">{assigned_count}</span> }.into_any()
                } else {
                    view! {}.into_any()
                }}
            </div>
            <ul
                tabindex="0"
                class="dropdown-content menu bg-base-100 rounded-box z-50 w-52 p-2 shadow-lg border border-base-300 max-h-60 overflow-y-auto"
            >
                {all_tags.into_iter().map(|tag| {
                    let tag_id = tag.id.clone();
                    let tag_id_check = tag.id.clone();
                    let is_assigned = item_tags.iter().any(|t| t.id == tag_id_check);
                    let color = tag.color.clone().unwrap_or_else(|| "#6b7280".to_string());
                    let name = tag.name.clone();
                    view! {
                        <li>
                            <label class="flex items-center gap-2 cursor-pointer p-1">
                                <input
                                    type="checkbox"
                                    class="checkbox checkbox-xs"
                                    prop:checked=is_assigned
                                    on:change=move |_| on_toggle.run(tag_id.clone())
                                />
                                <span
                                    class="inline-block w-3 h-3 rounded-full flex-shrink-0"
                                    style=format!("background:{color}")
                                />
                                <span class="text-sm truncate">{name}</span>
                            </label>
                        </li>
                    }
                }).collect_view()}
            </ul>
        </div>
    }
    .into_any()
}
