use chrono::NaiveDate;
use leptos::prelude::*;
use leptos_router::{components::A, hooks::use_params_map};

use crate::app::{ToastContext, ToastKind};
use crate::components::common::loading::LoadingSpinner;
use crate::server_fns::items::{get_items_by_date, toggle_item};
use kartoteka_shared::types::DateItem;

/// Add `days` to a "YYYY-MM-DD" date string. Returns the input unchanged on parse error.
fn add_days(date: &str, days: i64) -> String {
    use std::str::FromStr;
    NaiveDate::from_str(date)
        .ok()
        .and_then(|d| {
            d.checked_add_signed(chrono::Duration::days(days))
                .map(|nd| nd.format("%Y-%m-%d").to_string())
        })
        .unwrap_or_else(|| date.to_string())
}

/// Groups DateItems by list, preserving encounter order.
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
pub fn CalendarDayPage() -> impl IntoView {
    let params = use_params_map();
    let date = move || params.read().get("date").unwrap_or_default();

    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let (refresh, set_refresh) = signal(0u32);

    let items_res = Resource::new(
        move || (date(), refresh.get()),
        |(d, _)| get_items_by_date(d),
    );

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
            // Day header with prev/next navigation
            {move || {
                let d = date();
                let prev = add_days(&d, -1);
                let next = add_days(&d, 1);
                view! {
                    <div class="flex items-center justify-between mb-4">
                        <A href=format!("/calendar/{prev}") attr:class="btn btn-sm btn-ghost">"‹"</A>
                        <div class="text-center">
                            <h1 class="text-xl font-bold">{d}</h1>
                            <A href="/calendar" attr:class="text-xs text-base-content/50 hover:text-primary">
                                "← Calendar"
                            </A>
                        </div>
                        <A href=format!("/calendar/{next}") attr:class="btn btn-sm btn-ghost">"›"</A>
                    </div>
                }
            }}

            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || items_res.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="text-error">"Error: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(items) if items.is_empty() => view! {
                        <p class="text-center text-base-content/50 py-12">
                            "No items scheduled for this date."
                        </p>
                    }.into_any(),
                    Ok(items) => {
                        let groups = group_by_list(items);
                        view! {
                            <div class="flex flex-col gap-4">
                                {groups.into_iter().map(|(list_id, list_name, group_items)| {
                                    view! {
                                        <div>
                                            <h4 class="text-sm font-semibold uppercase tracking-wide mb-1 text-base-content/70">
                                                <A href=format!("/lists/{list_id}") attr:class="link link-hover">
                                                    {list_name}
                                                </A>
                                            </h4>
                                            <div class="flex flex-col gap-1">
                                                {group_items.into_iter().map(|date_item| {
                                                    let item_id = date_item.item.id.clone();
                                                    let href = format!(
                                                        "/lists/{}/items/{}",
                                                        date_item.item.list_id,
                                                        date_item.item.id
                                                    );
                                                    let completed = date_item.item.completed;
                                                    let title = date_item.item.title.clone();

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
                                                        </div>
                                                    }
                                                }).collect_view()}
                                            </div>
                                        </div>
                                    }
                                }).collect_view()}
                            </div>
                        }.into_any()
                    }
                })}
            </Suspense>
        </div>
    }
}
