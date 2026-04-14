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
                        let has_overdue = !data.overdue.is_empty();
                        let has_today = !data.today.is_empty();
                        let overdue_groups = group_by_list(data.overdue);
                        let today_groups = group_by_list(data.today);

                        view! {
                            <div>
                                // Page header
                                <div class="flex items-center justify-between mb-6">
                                    <h1 class="text-2xl font-bold">"Today"</h1>
                                    <span class="text-base-content/50 text-sm">{data.today_date}</span>
                                </div>

                                // Overdue section
                                {if has_overdue {
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
                                {if has_today {
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
                                } else if !has_overdue {
                                    view! {
                                        <p class="text-center text-base-content/50 py-12">
                                            "Nothing scheduled for today."
                                        </p>
                                    }.into_any()
                                } else { ().into_any() }}
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
