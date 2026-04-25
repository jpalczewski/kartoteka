use kartoteka_shared::types::Tag;
use leptos::prelude::*;
use std::collections::HashSet;

/// Chips to toggle visibility of lists, tags, and completed items. Degrades gracefully:
/// - only `lists` + `hidden_lists` → just list chips (original behavior)
/// - add `tags` + `hidden_tags` → tag chips appear too
/// - add `show_completed` → "pokaż ukończone" checkbox appears
///
/// Renders nothing when all three sections would be empty.
#[component]
pub fn ListFilterChips(
    lists: Vec<(String, String)>,
    hidden_lists: RwSignal<HashSet<String>>,
    #[prop(optional)] tags: Option<Vec<Tag>>,
    #[prop(optional)] hidden_tags: Option<RwSignal<HashSet<String>>>,
    #[prop(optional)] show_completed: Option<RwSignal<bool>>,
) -> impl IntoView {
    let show_list_chips = lists.len() > 1;
    let tag_chips = tags.unwrap_or_default();
    let show_tag_chips = !tag_chips.is_empty() && hidden_tags.is_some();
    let show_completed_toggle = show_completed.is_some();

    if !show_list_chips && !show_tag_chips && !show_completed_toggle {
        return view! {}.into_any();
    }

    view! {
        <div class="flex flex-col gap-2 mb-4">
            {show_list_chips.then(|| view! {
                <div class="flex flex-wrap gap-1" data-testid="filter-list-chips">
                    {lists.into_iter().map(|(lid, lname)| {
                        let lid_class = lid.clone();
                        let lid_click = lid.clone();
                        view! {
                            <button
                                type="button"
                                class=move || {
                                    if hidden_lists.get().contains(&lid_class) {
                                        "badge badge-ghost opacity-40 cursor-pointer"
                                    } else {
                                        "badge badge-primary cursor-pointer"
                                    }
                                }
                                on:click=move |_| {
                                    hidden_lists.update(|hl| {
                                        if hl.contains(&lid_click) {
                                            hl.remove(&lid_click);
                                        } else {
                                            hl.insert(lid_click.clone());
                                        }
                                    });
                                }
                            >{lname}</button>
                        }
                    }).collect_view()}
                </div>
            })}

            {(show_tag_chips).then(|| {
                let hidden = hidden_tags.expect("hidden_tags required when showing tag chips");
                view! {
                    <div class="flex flex-wrap gap-1" data-testid="filter-tag-chips">
                        {tag_chips.into_iter().map(|tag| {
                            let tid_class = tag.id.clone();
                            let tid_click = tag.id.clone();
                            let color = tag.color.unwrap_or_else(|| "#6b7280".to_string());
                            let name = tag.name;
                            view! {
                                <button
                                    type="button"
                                    class=move || {
                                        if hidden.get().contains(&tid_class) {
                                            "badge opacity-30 cursor-pointer"
                                        } else {
                                            "badge cursor-pointer"
                                        }
                                    }
                                    style=format!("background: {color}; color: white;")
                                    on:click=move |_| {
                                        hidden.update(|ht| {
                                            if ht.contains(&tid_click) {
                                                ht.remove(&tid_click);
                                            } else {
                                                ht.insert(tid_click.clone());
                                            }
                                        });
                                    }
                                >{name}</button>
                            }
                        }).collect_view()}
                    </div>
                }
            })}

            {show_completed.map(|sc| view! {
                <label class="flex items-center gap-2 text-sm cursor-pointer">
                    <input
                        type="checkbox"
                        class="checkbox checkbox-sm"
                        data-testid="filter-show-completed"
                        prop:checked=move || sc.get()
                        on:change=move |ev| sc.set(event_target_checked(&ev))
                    />
                    <span>"Pokaż ukończone"</span>
                </label>
            })}
        </div>
    }
    .into_any()
}
