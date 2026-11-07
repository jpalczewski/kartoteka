use std::collections::{HashMap, HashSet};

use kartoteka_shared::{FilterMode, types::Tag};
use leptos::prelude::*;
use leptos_fluent::{I18n, move_tr};

use crate::components::tags::tag_badge::TagBadge;

#[component]
pub fn HomeTagFilterBar(
    all_tags: Signal<Vec<Tag>>,
    ancestor_map: Memo<HashMap<String, String>>,
    active_tags: RwSignal<HashSet<String>>,
    filter_mode: RwSignal<FilterMode>,
    related_tag_ids: Signal<HashSet<String>>,
    is_loading: Signal<bool>,
) -> impl IntoView {
    let i18n = expect_context::<I18n>();

    view! {
        <div class="mb-3 flex flex-col gap-2">

            // Mode switch
            <div class="flex items-center gap-2">
                <div class="join">
                    {[
                        (FilterMode::Listwise, "tags-filter-mode-lists"),
                        (FilterMode::Itemwise, "tags-filter-mode-items"),
                        (FilterMode::Joined, "tags-filter-mode-both"),
                    ].map(|(mode, label_key)| {
                        let mode_for_cmp = mode.clone();
                        let mode_for_set = mode.clone();
                        view! {
                            <button
                                type="button"
                                class=move || if filter_mode.get() == mode_for_cmp {
                                    "join-item btn btn-xs btn-primary"
                                } else {
                                    "join-item btn btn-xs"
                                }
                                on:click=move |_| filter_mode.set(mode_for_set.clone())
                            >
                                {move || i18n.tr(label_key)}
                            </button>
                        }
                    })}
                </div>
                {move || is_loading.get().then(|| view! {
                    <span class="loading loading-spinner loading-xs" />
                })}
            </div>

            // Selected tags (shown with × to deselect)
            {move || {
                let active = active_tags.get();
                if active.is_empty() { return view! {}.into_any(); }
                let tags = all_tags.get();
                let paths = ancestor_map.get();
                let selected: Vec<Tag> = tags.into_iter()
                    .filter(|t| active.contains(&t.id))
                    .collect();
                view! {
                    <div class="flex flex-wrap gap-1">
                        {selected.into_iter().map(|tag| {
                            let tid = tag.id.clone();
                            let label = format!(
                                "{} ×",
                                paths.get(&tid).cloned().unwrap_or_else(|| tag.name.clone())
                            );
                            view! {
                                <TagBadge
                                    tag=tag
                                    active=true
                                    label=label
                                    on_click=Callback::new(move |id: String| {
                                        active_tags.update(|s| { s.remove(&id); });
                                    })
                                />
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            }}

            // Available tags (all when no filter, narrowed when filter active)
            {move || {
                let active = active_tags.get();
                let all = all_tags.get();
                let related = related_tag_ids.get();
                let paths = ancestor_map.get();

                let available: Vec<Tag> = if active.is_empty() {
                    all
                } else {
                    all.into_iter()
                        .filter(|t| related.contains(&t.id) && !active.contains(&t.id))
                        .collect()
                };

                if available.is_empty() && !active.is_empty() {
                    return view! {
                        <p class="text-sm text-base-content/50">
                            {move_tr!("tags-filter-no-matches")}
                        </p>
                    }.into_any();
                }
                if available.is_empty() {
                    return view! {}.into_any();
                }

                view! {
                    <div class="flex flex-wrap gap-1">
                        {available.into_iter().map(|tag| {
                            let tid = tag.id.clone();
                            let label = paths.get(&tid)
                                .cloned()
                                .unwrap_or_else(|| tag.name.clone());
                            view! {
                                <TagBadge
                                    tag=tag
                                    active=false
                                    label=label
                                    on_click=Callback::new(move |id: String| {
                                        active_tags.update(|s| { s.insert(id); });
                                    })
                                />
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            }}

        </div>
    }
}
