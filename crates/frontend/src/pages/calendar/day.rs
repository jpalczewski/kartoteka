use leptos::prelude::*;
use leptos_router::{components::A, hooks::use_params_map};
use reactive_stores::Store;

use kartoteka_shared::ChangeTopic;
use kartoteka_shared::types::DateItem;

use super::add_days;
use crate::components::calendar::item_row::CalendarItemRow;
use crate::components::common::loading::LoadingSpinner;
use crate::server_fns::items::get_items_by_date;
use crate::state::invalidation::use_invalidation;
use crate::state::item_store::{ItemRows, patch_rows, tracked_items};

/// The items of one list in the day view, in display order.
#[derive(Clone, Debug, PartialEq)]
struct ListGroup {
    list_id: String,
    list_name: String,
    item_ids: Vec<String>,
}

/// Group entries by list, keeping first-seen list order and item order.
fn group_ids(items: &[DateItem]) -> Vec<ListGroup> {
    let mut groups: Vec<ListGroup> = Vec::new();
    for di in items {
        match groups.iter_mut().find(|g| g.list_id == di.item.list_id) {
            Some(group) => group.item_ids.push(di.item.id.clone()),
            None => groups.push(ListGroup {
                list_id: di.item.list_id.clone(),
                list_name: di.list_name.clone(),
                item_ids: vec![di.item.id.clone()],
            }),
        }
    }
    groups
}

#[component]
pub fn CalendarDayPage() -> impl IntoView {
    let params = use_params_map();
    let date = move || params.read().get("date").unwrap_or_default();

    let invalidation = use_invalidation();
    let show_completed = RwSignal::new(true);

    let items_res = Resource::new(
        move || {
            (
                date(),
                invalidation.track(&[ChangeTopic::Items, ChangeTopic::Lists]),
            )
        },
        |(d, _)| get_items_by_date(d),
    );

    // Row data lives in a keyed store, so a refetch updates the changed rows in place.
    let day_rows = Store::new(ItemRows::<DateItem>::default());

    let total = move || tracked_items(day_rows).len();
    let completed_count = move || {
        tracked_items(day_rows)
            .iter()
            .filter(|di| di.item.completed)
            .count()
    };
    let has_any = move || total() > 0;
    let groups = Memo::new(move |_| {
        let show = show_completed.get();
        let visible = tracked_items(day_rows)
            .into_iter()
            .filter(|di| show || !di.item.completed)
            .collect::<Vec<_>>();
        group_ids(&visible)
    });

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

            <Transition fallback=|| view! { <LoadingSpinner/> }>
                // Patch the store first: this closure runs before the list below in the
                // same render pass, on the server and in the browser.
                {move || match items_res.get() {
                    Some(Ok(items)) => {
                        patch_rows(day_rows, items);
                        ().into_any()
                    }
                    Some(Err(e)) => view! {
                        <p class="text-error">"Błąd: " {e.to_string()}</p>
                    }.into_any(),
                    None => ().into_any(),
                }}

                <Show when=move || items_res.with(|r| matches!(r, Some(Ok(_))))>
                    <Show
                        when=has_any
                        fallback=|| view! {
                            <p class="text-center text-base-content/50 py-12">
                                "Brak zadań na ten dzień."
                            </p>
                        }
                    >
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
                                        on:change=move |ev| show_completed.set(!event_target_checked(&ev))
                                    />
                                </label>
                            </div>

                            <DayGroups store=day_rows groups=groups/>
                        </div>
                    </Show>
                </Show>
            </Transition>
        </div>
    }
}

/// List groups of the day, keyed by list id, each with its rows keyed by item id.
#[component]
fn DayGroups(store: Store<ItemRows<DateItem>>, groups: Memo<Vec<ListGroup>>) -> impl IntoView {
    view! {
        <div class="flex flex-col gap-4">
            <For
                each=move || groups.get()
                key=|group| group.list_id.clone()
                let:group
            >
                {
                    let list_id = group.list_id.clone();
                    let list_id_name = list_id.clone();
                    let list_name = move || {
                        groups.with(|gs| {
                            gs.iter()
                                .find(|g| g.list_id == list_id_name)
                                .map(|g| g.list_name.clone())
                                .unwrap_or_default()
                        })
                    };
                    let item_ids = move || {
                        groups.with(|gs| {
                            gs.iter()
                                .find(|g| g.list_id == list_id)
                                .map(|g| g.item_ids.clone())
                                .unwrap_or_default()
                        })
                    };
                    view! {
                        <div>
                            <h4 class="text-sm font-semibold uppercase tracking-wide mb-1 text-base-content/70">
                                <A href=format!("/lists/{}", group.list_id) attr:class="link link-hover">
                                    {list_name}
                                </A>
                            </h4>
                            <div class="flex flex-col gap-1">
                                <For each=item_ids key=|id| id.clone() let:id>
                                    <CalendarItemRow store=store id=id/>
                                </For>
                            </div>
                        </div>
                    }
                }
            </For>
        </div>
    }
}
