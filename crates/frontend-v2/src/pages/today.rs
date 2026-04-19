use leptos::prelude::*;
use leptos_router::components::A;

use crate::app::{ToastContext, ToastKind};
use crate::components::common::loading::LoadingSpinner;
use crate::server_fns::items::{get_today_data, toggle_item};
use kartoteka_shared::types::DateItem;

/// Groups a flat list of DateItems by list, preserving encounter order.
fn group_by_list(items: Vec<DateItem>) -> Vec<(String, String, Vec<DateItem>)> {
    let mut groups: Vec<(String, String, Vec<DateItem>)> = Vec::new();
    for item in items {
        let lid = item.item.list_id.clone();
        if let Some(g) = groups.iter_mut().find(|(id, _, _)| id == &lid) {
            g.2.push(item);
        } else {
            groups.push((lid, item.list_name.clone(), vec![item]));
        }
    }
    groups
}

#[component]
pub fn TodayPage() -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let (refresh, set_refresh) = signal(0u32);
    let (show_completed, set_show_completed) = signal(true);

    let today_res = Resource::new(move || refresh.get(), |_| get_today_data());

    let on_toggle = move |item_id: String| {
        leptos::task::spawn_local(async move {
            match toggle_item(item_id).await {
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
                        <p class="text-error">"Error: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(data) => {
                        let all_items: Vec<&DateItem> = data.overdue.iter().chain(data.today.iter()).collect();
                        let total = all_items.len();
                        let completed_count = all_items.iter().filter(|di| di.item.completed).count();

                        let has_overdue = !data.overdue.is_empty();
                        let has_today = !data.today.is_empty();

                        let overdue_items = data.overdue;
                        let today_items = data.today;

                        view! {
                            <div>
                                // Page header
                                <div class="flex items-center justify-between mb-6">
                                    <h1 class="text-2xl font-bold">"Today"</h1>
                                    <span class="text-base-content/50 text-sm">{data.today_date}</span>
                                </div>

                                {move || {
                                    let visible_overdue: Vec<DateItem> = overdue_items
                                        .iter()
                                        .filter(|di| show_completed.get() || !di.item.completed)
                                        .cloned()
                                        .collect();
                                    let visible_today: Vec<DateItem> = today_items
                                        .iter()
                                        .filter(|di| show_completed.get() || !di.item.completed)
                                        .cloned()
                                        .collect();

                                    if !has_overdue && !has_today {
                                        // No items at all
                                        view! {
                                            <p class="text-center text-base-content/50 py-12">
                                                "Nothing scheduled for today."
                                            </p>
                                        }.into_any()
                                    } else {
                                        // Has items — always show header, then either list or filtered-empty message
                                        let all_hidden = visible_overdue.is_empty() && visible_today.is_empty();
                                        let overdue_groups = group_by_list(visible_overdue);
                                        let today_groups = group_by_list(visible_today);

                                        view! {
                                            <div>
                                                // Completion count + toggle header — always visible when items exist
                                                <div class="flex items-center justify-between mb-4">
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

                                                {if all_hidden {
                                                    view! {
                                                        <div class="text-center text-base-content/50 py-4">
                                                            "Wszystkie elementy ukończone — odznacz filtr aby je zobaczyć."
                                                        </div>
                                                    }.into_any()
                                                } else {
                                                    view! {
                                                        <div>
                                                            // Overdue section
                                                            {if !overdue_groups.is_empty() {
                                                                view! {
                                                                    <div class="mb-6">
                                                                        <h3 class="text-xs text-error uppercase tracking-wider font-semibold mb-2">
                                                                            "Overdue"
                                                                        </h3>
                                                                        {render_groups(overdue_groups, on_toggle)}
                                                                    </div>
                                                                }.into_any()
                                                            } else { ().into_any() }}

                                                            // Today section
                                                            {if !today_groups.is_empty() {
                                                                view! {
                                                                    <div class="mb-6">
                                                                        {if has_overdue {
                                                                            view! {
                                                                                <h3 class="text-xs text-base-content/50 uppercase tracking-wider font-semibold mb-2">
                                                                                    "Today"
                                                                                </h3>
                                                                            }.into_any()
                                                                        } else { ().into_any() }}
                                                                        {render_groups(today_groups, on_toggle)}
                                                                    </div>
                                                                }.into_any()
                                                            } else { ().into_any() }}
                                                        </div>
                                                    }.into_any()
                                                }}
                                            </div>
                                        }.into_any()
                                    }
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
                            let href = format!("/lists/{}/items/{}", date_item.item.list_id, date_item.item.id);
                            let completed = date_item.item.completed;
                            let title = date_item.item.title.clone();
                            let deadline = date_item.item.deadline
                                .as_ref()
                                .map(|d| d.to_string());

                            view! {
                                <div class="flex items-center gap-3 p-2 bg-base-200 rounded-lg">
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
                                </div>
                            }
                        }).collect_view()}
                    </div>
                </div>
            }
        })
        .collect_view()
}
