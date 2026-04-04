use std::collections::HashSet;

use leptos::prelude::*;

use crate::components::tags::tag_tree::TagFilterOption;

#[component]
pub fn FilterChips(
    unique_lists: Vec<(String, String)>,
    relevant_tags: Vec<TagFilterOption>,
    hidden_lists: HashSet<String>,
    hidden_tags: HashSet<String>,
    show_completed: bool,
    on_toggle_list: Callback<String>,
    on_toggle_tag: Callback<String>,
    on_toggle_show_completed: Callback<()>,
) -> impl IntoView {
    view! {
        <div>
            // Filter chips — lists
            <div class="flex flex-wrap gap-2 mb-2">
                {unique_lists.into_iter().map(|(list_id, list_name)| {
                    let lid = list_id.clone();
                    let is_hidden = {
                        let lid = lid.clone();
                        let hidden_lists = hidden_lists.clone();
                        move || hidden_lists.contains(&lid)
                    };
                    view! {
                        <button
                            class=move || if is_hidden() {
                                "btn btn-xs btn-ghost opacity-40 line-through"
                            } else {
                                "btn btn-xs btn-outline btn-primary"
                            }
                            on:click=move |_| on_toggle_list.run(list_id.clone())
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
                            let tag_name = tag.label.clone();
                            let tag_color = tag.color.clone();
                            let is_hidden = {
                                let tid = tid.clone();
                                let hidden_tags = hidden_tags.clone();
                                move || hidden_tags.contains(&tid)
                            };
                            view! {
                                <button
                                    class=move || if is_hidden() {
                                        "badge badge-sm h-auto whitespace-normal py-1 text-left opacity-40 line-through cursor-pointer"
                                    } else {
                                        "badge badge-sm h-auto whitespace-normal py-1 text-left cursor-pointer"
                                    }
                                    style=format!("background-color: {}; color: white;", tag_color)
                                    on:click=move |_| on_toggle_tag.run(tid.clone())
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
                    prop:checked=show_completed
                    on:change=move |_| on_toggle_show_completed.run(())
                />
                <span class="text-sm text-base-content/60">"Ukończone"</span>
            </label>
        </div>
    }
}
