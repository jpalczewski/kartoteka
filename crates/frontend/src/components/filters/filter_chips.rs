use std::collections::HashSet;

use kartoteka_shared::Tag;
use leptos::prelude::*;

#[component]
pub fn FilterChips(
    unique_lists: Vec<(String, String)>,
    relevant_tags: Vec<Tag>,
    hidden_lists: RwSignal<HashSet<String>>,
    hidden_tags: RwSignal<HashSet<String>>,
    show_completed: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <div>
            // Filter chips — lists
            <div class="flex flex-wrap gap-2 mb-2">
                {unique_lists.into_iter().map(|(list_id, list_name)| {
                    let lid = list_id.clone();
                    let is_hidden = {
                        let lid = lid.clone();
                        move || hidden_lists.get().contains(&lid)
                    };
                    view! {
                        <button
                            class=move || if is_hidden() {
                                "btn btn-xs btn-ghost opacity-40 line-through"
                            } else {
                                "btn btn-xs btn-outline btn-primary"
                            }
                            on:click=move |_| {
                                hidden_lists.update(|s| {
                                    if !s.remove(&list_id) {
                                        s.insert(list_id.clone());
                                    }
                                });
                            }
                        >
                            {list_name}
                        </button>
                    }
                }).collect_view()}
            </div>

            // Filter chips — tags
            {if !relevant_tags.is_empty() {
                view! {
                    <div class="flex flex-wrap gap-1 mb-2">
                        {relevant_tags.into_iter().map(|tag| {
                            let tid = tag.id.clone();
                            let tag_name = tag.name.clone();
                            let tag_color = tag.color.clone();
                            let is_hidden = {
                                let tid = tid.clone();
                                move || hidden_tags.get().contains(&tid)
                            };
                            view! {
                                <button
                                    class=move || if is_hidden() {
                                        "badge badge-sm opacity-40 line-through cursor-pointer"
                                    } else {
                                        "badge badge-sm cursor-pointer"
                                    }
                                    style=format!("background-color: {}; color: white;", tag_color)
                                    on:click=move |_| {
                                        hidden_tags.update(|s| {
                                            if !s.remove(&tid) {
                                                s.insert(tid.clone());
                                            }
                                        });
                                    }
                                >
                                    {tag_name}
                                </button>
                            }
                        }).collect_view()}
                    </div>
                }.into_any()
            } else {
                ().into_any()
            }}

            // Show completed toggle
            <label class="flex items-center gap-2 cursor-pointer mb-4">
                <input
                    type="checkbox"
                    class="toggle toggle-sm toggle-primary"
                    prop:checked=move || show_completed.get()
                    on:change=move |_| show_completed.update(|v| *v = !*v)
                />
                <span class="text-sm text-base-content/60">"Ukończone"</span>
            </label>
        </div>
    }
}
