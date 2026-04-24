use std::collections::HashSet;

use leptos::prelude::*;
use leptos_router::components::A;

use kartoteka_shared::types::DateItem;

use crate::app::{ToastContext, ToastKind};
use crate::components::common::list_filter_chips::ListFilterChips;
use crate::components::common::loading::LoadingSpinner;
use crate::server_fns::items::{delete_item, get_today_data, toggle_item};
use crate::utils::group_by_list;

#[component]
pub fn TodayPage() -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let (refresh, set_refresh) = signal(0u32);
    let (show_completed, set_show_completed) = signal(true);
    let hidden_lists: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());

    let today_res = Resource::new(move || refresh.get(), |_| get_today_data());

    let on_toggle = move |item_id: String| {
        leptos::task::spawn_local(async move {
            match toggle_item(item_id).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    let on_delete = move |item_id: String| {
        leptos::task::spawn_local(async move {
            match delete_item(item_id).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || today_res.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="text-error">"Błąd: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(data) => {
                        let all_items: Vec<&DateItem> = data.overdue.iter().chain(data.today.iter()).collect();
                        let total = all_items.len();
                        let completed_count = all_items.iter().filter(|di| di.item.completed).count();

                        let has_overdue = !data.overdue.is_empty();
                        let has_today = !data.today.is_empty();

                        let mut unique_lists: Vec<(String, String)> = Vec::new();
                        let mut seen = HashSet::new();
                        for di in &all_items {
                            if seen.insert(di.item.list_id.clone()) {
                                unique_lists.push((di.item.list_id.clone(), di.list_name.clone()));
                            }
                        }

                        let overdue_items = data.overdue;
                        let today_items = data.today;

                        view! {
                            <div>
                                <div class="flex items-center justify-between mb-4">
                                    <h1 class="text-2xl font-bold">"Dziś"</h1>
                                    <span class="text-base-content/50 text-sm">{data.today_date}</span>
                                </div>

                                {if !has_overdue && !has_today {
                                    view! {
                                        <p class="text-center text-base-content/50 py-12">
                                            "Brak zadań na dziś."
                                        </p>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div>
                                            <div class="flex items-center justify-between mb-3">
                                                <span class="text-sm text-base-content/60" data-testid="today-completion-count">
                                                    {completed_count} "/" {total} " ukończone"
                                                </span>
                                                <label class="flex items-center gap-2 cursor-pointer select-none">
                                                    <span class="text-xs text-base-content/50">"Ukryj ukończone"</span>
                                                    <input
                                                        type="checkbox"
                                                        class="toggle toggle-xs"
                                                        data-testid="hide-completed-toggle"
                                                        prop:checked=move || !show_completed.get()
                                                        on:change=move |ev| set_show_completed.set(!event_target_checked(&ev))
                                                    />
                                                </label>
                                            </div>

                                            <ListFilterChips lists=unique_lists hidden_lists=hidden_lists />

                                            {move || {
                                                let hl = hidden_lists.get();
                                                let sc = show_completed.get();

                                                let filter = |items: &Vec<DateItem>| -> Vec<DateItem> {
                                                    items.iter()
                                                        .filter(|di| !hl.contains(&di.item.list_id))
                                                        .filter(|di| sc || !di.item.completed)
                                                        .cloned()
                                                        .collect()
                                                };

                                                let visible_overdue = filter(&overdue_items);
                                                let visible_today = filter(&today_items);
                                                let all_hidden = visible_overdue.is_empty() && visible_today.is_empty();
                                                let has_visible_overdue = !visible_overdue.is_empty();

                                                if all_hidden {
                                                    view! {
                                                        <div class="text-center text-base-content/50 py-4">
                                                            "Wszystkie elementy ukończone lub ukryte."
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <div>
                                                            {if has_visible_overdue {
                                                                view! {
                                                                    <div class="mb-6">
                                                                        <h3 class="text-xs text-error uppercase tracking-wider font-semibold mb-2">
                                                                            "Zaległe"
                                                                        </h3>
                                                                        {render_groups(group_by_list(visible_overdue), on_toggle, on_delete)}
                                                                    </div>
                                                                }.into_any()
                                                            } else { ().into_any() }}

                                                            {if !visible_today.is_empty() {
                                                                view! {
                                                                    <div class="mb-6">
                                                                        {if has_overdue && has_visible_overdue {
                                                                            view! {
                                                                                <h3 class="text-xs text-base-content/50 uppercase tracking-wider font-semibold mb-2">
                                                                                    "Dziś"
                                                                                </h3>
                                                                            }.into_any()
                                                                        } else { ().into_any() }}
                                                                        {render_groups(group_by_list(visible_today), on_toggle, on_delete)}
                                                                    </div>
                                                                }.into_any()
                                                            } else { ().into_any() }}
                                                        </div>
                                                    }.into_any()
                                                }
                                            }}
                                        </div>
                                    }.into_any()
                                }}
                            </div>
                        }.into_any()
                    }
                })}
            </Suspense>
        </div>
    }
}

fn render_groups(
    groups: Vec<(String, String, Vec<DateItem>)>,
    on_toggle: impl Fn(String) + Copy + 'static,
    on_delete: impl Fn(String) + Copy + 'static,
) -> impl IntoView {
    groups
        .into_iter()
        .map(|(list_id, list_name, items)| {
            view! {
                <div class="mb-4">
                    <h4 class="text-sm font-semibold uppercase tracking-wide mb-1 text-base-content/70">
                        <A href=format!("/lists/{list_id}") attr:class="link link-hover">
                            {list_name}
                        </A>
                    </h4>
                    <div class="flex flex-col gap-1">
                        {items.into_iter().map(|date_item| {
                            let item_id = date_item.item.id.clone();
                            let item_id_del = item_id.clone();
                            let href = format!("/lists/{}/items/{}", date_item.item.list_id, date_item.item.id);
                            let completed = date_item.item.completed;
                            let title = date_item.item.title.clone();
                            let deadline = date_item.item.deadline
                                .as_ref()
                                .map(|d| d.to_string());

                            view! {
                                <div class="flex items-center gap-3 p-2 bg-base-200 rounded-lg group">
                                    <input
                                        type="checkbox"
                                        class="checkbox checkbox-sm checkbox-primary"
                                        checked=completed
                                        on:change=move |_| on_toggle(item_id.clone())
                                    />
                                    <A
                                        href=href
                                        attr:class=move || if completed {
                                            "flex-1 text-sm text-base-content/50 line-through"
                                        } else {
                                            "flex-1 text-sm text-base-content hover:text-primary"
                                        }
                                    >
                                        {title}
                                    </A>
                                    {deadline.map(|d| view! {
                                        <span class="text-xs text-error/70">{d}</span>
                                    })}
                                    <button
                                        class="btn btn-ghost btn-xs text-error opacity-0 group-hover:opacity-100"
                                        on:click=move |_| on_delete(item_id_del.clone())
                                    >
                                        "×"
                                    </button>
                                </div>
                            }
                        }).collect_view()}
                    </div>
                </div>
            }
        })
        .collect_view()
}
