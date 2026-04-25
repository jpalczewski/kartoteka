use leptos::prelude::*;
use leptos_router::{components::A, hooks::use_params_map};

use super::add_days;
use crate::app::{ToastContext, ToastKind};
use crate::components::calendar::item_row::CalendarItemRow;
use crate::components::common::loading::LoadingSpinner;
use crate::server_fns::items::{delete_item, get_items_by_date, toggle_item};
use crate::utils::group_by_list;

#[component]
pub fn CalendarDayPage() -> impl IntoView {
    let params = use_params_map();
    let date = move || params.read().get("date").unwrap_or_default();

    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let (refresh, set_refresh) = signal(0u32);
    let (show_completed, set_show_completed) = signal(true);

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
                                "← Terminarz"
                            </A>
                        </div>
                        <A href=format!("/calendar/{next}") attr:class="btn btn-sm btn-ghost">"›"</A>
                    </div>
                }
            }}

            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || items_res.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="text-error">"Błąd: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(all_items) => {
                        let total = all_items.len();
                        let completed_count = all_items.iter().filter(|di| di.item.completed).count();

                        if all_items.is_empty() {
                            return view! {
                                <p class="text-center text-base-content/50 py-12">
                                    "Brak zadań na ten dzień."
                                </p>
                            }.into_any();
                        }

                        let visible = all_items
                            .into_iter()
                            .filter(|di| show_completed.get() || !di.item.completed)
                            .collect::<Vec<_>>();
                        let groups = group_by_list(visible);

                        view! {
                            <div>
                                <div class="flex items-center justify-between mb-4">
                                    <span class="text-sm text-base-content/60">
                                        {completed_count} "/" {total} " ukończone"
                                    </span>
                                    <label class="flex items-center gap-2 cursor-pointer select-none">
                                        <span class="text-xs text-base-content/50">"Ukryj ukończone"</span>
                                        <input
                                            type="checkbox"
                                            class="toggle toggle-xs"
                                            prop:checked=move || !show_completed.get()
                                            on:change=move |ev| set_show_completed.set(!event_target_checked(&ev))
                                        />
                                    </label>
                                </div>

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
                                                        let item_id_del = item_id.clone();
                                                        view! {
                                                            <CalendarItemRow
                                                                item_id=item_id.clone()
                                                                list_id=date_item.item.list_id.clone()
                                                                title=date_item.item.title.clone()
                                                                completed=date_item.item.completed
                                                                on_toggle=Callback::new(move |()| on_toggle(item_id.clone()))
                                                                on_delete=Callback::new(move |()| on_delete(item_id_del.clone()))
                                                            />
                                                        }
                                                    }).collect_view()}
                                                </div>
                                            </div>
                                        }
                                    }).collect_view()}
                                </div>
                            </div>
                        }.into_any()
                    }
                })}
            </Suspense>
        </div>
    }
}
