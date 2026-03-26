use std::collections::{BTreeMap, HashSet};

use leptos::prelude::*;
use leptos_router::components::A;

use crate::api;
use crate::app::{ToastContext, ToastKind};
use crate::components::date_item_row::{DateItemRow, get_today_string};
use kartoteka_shared::*;

#[component]
pub fn TodayPage() -> impl IntoView {
    if !api::is_logged_in() {
        return view! { <p><a href="/login">"Zaloguj się"</a></p> }.into_any();
    }

    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let today = get_today_string();
    let items = RwSignal::new(Vec::<DateItem>::new());
    let all_tags = RwSignal::new(Vec::<Tag>::new());
    let item_tag_links = RwSignal::new(Vec::<ItemTagLink>::new());
    let (loading, set_loading) = signal(true);
    let hidden_lists = RwSignal::new(HashSet::<String>::new());
    let hidden_tags = RwSignal::new(HashSet::<String>::new());
    let show_completed = RwSignal::new(true);

    let today_for_fetch = today.clone();
    let _resource = LocalResource::new(move || {
        let date = today_for_fetch.clone();
        async move {
            match api::fetch_items_by_date(&date, true).await {
                Ok(fetched) => items.set(fetched),
                Err(e) => toast.push(format!("Błąd ładowania zadań: {e}"), ToastKind::Error),
            }
            if let Ok(tags) = api::fetch_tags().await {
                all_tags.set(tags);
            }
            if let Ok(links) = api::fetch_item_tag_links().await {
                item_tag_links.set(links);
            }
            set_loading.set(false);
        }
    });

    let today_display = today.clone();
    let today_for_overdue = today.clone();

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <div class="flex items-center justify-between mb-4">
                <h1 class="text-2xl font-bold">"Dziś"</h1>
                <span class="text-base-content/50">{format_polish_date(&today_display)}</span>
            </div>

            {move || {
                if loading.get() {
                    return view! { <p>"Wczytywanie..."</p> }.into_any();
                }

                let all_items = items.get();
                let tags = all_tags.get();
                let links = item_tag_links.get();
                let today_str = today_for_overdue.clone();

                if all_items.is_empty() {
                    return view! {
                        <p class="text-center text-base-content/50 py-12">
                            "Brak zadań na dziś"
                        </p>
                    }.into_any();
                }

                // Collect unique lists from items
                let mut unique_lists: Vec<(String, String)> = Vec::new();
                let mut seen_lists = HashSet::new();
                for item in &all_items {
                    if seen_lists.insert(item.list_id.clone()) {
                        unique_lists.push((item.list_id.clone(), item.list_name.clone()));
                    }
                }

                // Collect unique tags that appear on these items
                let item_ids: HashSet<String> = all_items.iter().map(|i| i.id.clone()).collect();
                let relevant_tag_ids: HashSet<String> = links.iter()
                    .filter(|l| item_ids.contains(&l.item_id))
                    .map(|l| l.tag_id.clone())
                    .collect();
                let relevant_tags: Vec<Tag> = tags.iter()
                    .filter(|t| relevant_tag_ids.contains(&t.id))
                    .cloned()
                    .collect();

                // Filter items
                let hl = hidden_lists.get();
                let ht = hidden_tags.get();
                let sc = show_completed.get();

                let filtered: Vec<DateItem> = all_items.into_iter()
                    .filter(|item| {
                        if hl.contains(&item.list_id) { return false; }
                        if !sc && item.completed { return false; }
                        if !ht.is_empty() {
                            let item_tags: HashSet<String> = links.iter()
                                .filter(|l| l.item_id == item.id)
                                .map(|l| l.tag_id.clone())
                                .collect();
                            if ht.iter().any(|t| item_tags.contains(t)) {
                                return false;
                            }
                        }
                        true
                    })
                    .collect();

                // Split into overdue and today
                let mut overdue_groups: BTreeMap<(String, String), Vec<DateItem>> = BTreeMap::new();
                let mut today_groups: BTreeMap<(String, String), Vec<DateItem>> = BTreeMap::new();

                for item in filtered {
                    let is_overdue = item.due_date.as_ref()
                        .map(|d| d < &today_str && !item.completed)
                        .unwrap_or(false);
                    let key = (item.list_id.clone(), item.list_name.clone());
                    if is_overdue {
                        overdue_groups.entry(key).or_default().push(item);
                    } else {
                        today_groups.entry(key).or_default().push(item);
                    }
                }

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

                        // Overdue section
                        {
                            let has_overdue = !overdue_groups.is_empty();
                            let has_today = !today_groups.is_empty();

                            let overdue_view = if has_overdue {
                                view! {
                                    <div class="mb-6">
                                        <h3 class="text-xs text-error uppercase tracking-wider font-semibold mb-2">
                                            "Zaległe"
                                        </h3>
                                        {render_groups(overdue_groups, tags.clone(), links.clone(), items)}
                                    </div>
                                }.into_any()
                            } else {
                                ().into_any()
                            };

                            let today_view = if has_today {
                                view! {
                                    <div class="mb-6">
                                        {if has_overdue {
                                            view! {
                                                <h3 class="text-xs text-base-content/50 uppercase tracking-wider font-semibold mb-2">
                                                    "Dziś"
                                                </h3>
                                            }.into_any()
                                        } else {
                                            ().into_any()
                                        }}
                                        {render_groups(today_groups, tags.clone(), links.clone(), items)}
                                    </div>
                                }.into_any()
                            } else {
                                ().into_any()
                            };

                            view! {
                                {overdue_view}
                                {today_view}
                            }
                        }
                    </div>
                }.into_any()
            }}
        </div>
    }
    .into_any()
}

fn render_groups(
    groups: BTreeMap<(String, String), Vec<DateItem>>,
    all_tags: Vec<Tag>,
    all_links: Vec<ItemTagLink>,
    items_signal: RwSignal<Vec<DateItem>>,
) -> impl IntoView {
    groups
        .into_iter()
        .map(|((list_id, list_name), group_items)| {
            let tags = all_tags.clone();
            let links = all_links.clone();
            view! {
                <div class="mb-4">
                    <h4 class="text-sm font-semibold uppercase tracking-wide mb-1 text-base-content/70">
                        <A href=format!("/lists/{list_id}") attr:class="link link-hover">
                            {list_name}
                        </A>
                    </h4>
                    {group_items.into_iter().map(|date_item| {
                        let item_id = date_item.id.clone();
                        let item_list_id = date_item.list_id.clone();
                        let item: Item = date_item.into();

                        let item_tag_ids: Vec<String> = links.iter()
                            .filter(|l| l.item_id == item_id)
                            .map(|l| l.tag_id.clone())
                            .collect();

                        let toggle_list_id = item_list_id.clone();
                        let toggle_item_id = item_id.clone();
                        let on_toggle = Callback::new(move |_id: String| {
                            let lid = toggle_list_id.clone();
                            let iid = toggle_item_id.clone();
                            // Optimistic update
                            items_signal.update(|items| {
                                if let Some(item) = items.iter_mut().find(|i| i.id == iid) {
                                    item.completed = !item.completed;
                                }
                            });
                            leptos::task::spawn_local(async move {
                                let current = items_signal.get_untracked()
                                    .iter()
                                    .find(|i| i.id == iid)
                                    .map(|i| i.completed)
                                    .unwrap_or(false);
                                let req = UpdateItemRequest {
                                    title: None,
                                    description: None,
                                    completed: Some(current),
                                    position: None,
                                    quantity: None,
                                    actual_quantity: None,
                                    unit: None,
                                    due_date: None,
                                    due_time: None,
                                };
                                let _ = api::update_item(&lid, &iid, &req).await;
                            });
                        });

                        let delete_list_id = item_list_id.clone();
                        let delete_item_id = item_id.clone();
                        let on_delete = Callback::new(move |_id: String| {
                            let lid = delete_list_id.clone();
                            let iid = delete_item_id.clone();
                            items_signal.update(|items| {
                                items.retain(|i| i.id != iid);
                            });
                            leptos::task::spawn_local(async move {
                                let _ = api::delete_item(&lid, &iid).await;
                            });
                        });

                        view! {
                            <DateItemRow
                                item=item
                                on_toggle=on_toggle
                                on_delete=on_delete
                                all_tags=tags.clone()
                                item_tag_ids=item_tag_ids
                            />
                        }
                    }).collect_view()}
                </div>
            }
        })
        .collect_view()
}

fn format_polish_date(date_str: &str) -> String {
    let parts: Vec<&str> = date_str.split('-').collect();
    if parts.len() != 3 {
        return date_str.to_string();
    }
    let day = parts[2].trim_start_matches('0');
    let month = match parts[1] {
        "01" => "stycznia",
        "02" => "lutego",
        "03" => "marca",
        "04" => "kwietnia",
        "05" => "maja",
        "06" => "czerwca",
        "07" => "lipca",
        "08" => "sierpnia",
        "09" => "września",
        "10" => "października",
        "11" => "listopada",
        "12" => "grudnia",
        _ => parts[1],
    };
    format!("{day} {month} {}", parts[0])
}
