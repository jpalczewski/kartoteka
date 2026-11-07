use std::collections::HashSet;

use leptos::prelude::*;
use reactive_stores::Store;

use kartoteka_shared::ChangeTopic;
use kartoteka_shared::types::{DateItem, ItemTagLink, Tag};

use crate::components::common::list_filter_chips::ListFilterChips;
use crate::components::common::loading::LoadingSpinner;
use crate::components::items::item_compact::{CompactTestIds, ItemCompact};
use crate::components::items::item_groups::{ItemGroups, group_ids};
use crate::components::items::item_view::ItemCaps;
use crate::server_fns::items::get_today_data;
use crate::server_fns::tags::{get_all_item_tag_links, get_all_tags};
use crate::state::invalidation::use_invalidation;
use crate::state::item_store::{ItemRows, patch_rows, tracked_items};

const ROW_CAPS: ItemCaps = ItemCaps {
    checkbox: true,
    dates: true,
    quantity: true,
    delete: true,
    ..ItemCaps::NONE
};

const ROW_TESTIDS: CompactTestIds = CompactTestIds {
    row: Some("today-row"),
    toggle: None,
    title: None,
    description: None,
};

#[component]
pub fn TodayPage() -> impl IntoView {
    let invalidation = use_invalidation();
    let show_completed = RwSignal::new(true);
    let hidden_lists: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());
    let hidden_tags: RwSignal<HashSet<String>> = RwSignal::new(HashSet::new());

    let today_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::Items, ChangeTopic::Lists]),
        |_| get_today_data(),
    );
    let all_tags_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::Tags]),
        |_| get_all_tags(),
    );
    let item_tag_links_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::Items, ChangeTopic::Tags]),
        |_| get_all_item_tag_links(),
    );

    // Row data lives in keyed stores, so a refetch (e.g. a live update) updates the
    // changed rows in place instead of re-creating every row.
    let overdue_rows = Store::new(ItemRows::<DateItem>::default());
    let today_rows = Store::new(ItemRows::<DateItem>::default());

    let links = move || -> Vec<ItemTagLink> {
        item_tag_links_res
            .get()
            .and_then(Result::ok)
            .unwrap_or_default()
    };

    let visible = move |items: Vec<DateItem>| -> Vec<DateItem> {
        let hl = hidden_lists.get();
        let sc = show_completed.get();
        let ht = hidden_tags.get();
        let links = if ht.is_empty() { Vec::new() } else { links() };
        items
            .into_iter()
            .filter(|di| !hl.contains(&di.item.list_id))
            .filter(|di| sc || !di.item.completed)
            .filter(|di| {
                ht.is_empty()
                    || !links
                        .iter()
                        .any(|l| l.item_id == di.item.id && ht.contains(&l.tag_id))
            })
            .collect()
    };
    let overdue_groups = Memo::new(move |_| group_ids(&visible(tracked_items(overdue_rows))));
    let today_groups = Memo::new(move |_| group_ids(&visible(tracked_items(today_rows))));

    let all_items = move || {
        let mut items = tracked_items(overdue_rows);
        items.extend(tracked_items(today_rows));
        items
    };
    let total = move || all_items().len();
    let completed_count = move || all_items().iter().filter(|di| di.item.completed).count();
    let has_any = move || total() > 0;
    let has_overdue = move || !overdue_groups.with(Vec::is_empty);
    let all_hidden = move || overdue_groups.with(Vec::is_empty) && today_groups.with(Vec::is_empty);

    let unique_lists = move || {
        let mut seen = HashSet::new();
        all_items()
            .into_iter()
            .filter(|di| seen.insert(di.item.list_id.clone()))
            .map(|di| (di.item.list_id, di.list_name))
            .collect::<Vec<_>>()
    };
    let relevant_tags = move || {
        let item_ids: HashSet<String> = all_items().into_iter().map(|di| di.item.id).collect();
        let tag_ids: HashSet<String> = links()
            .into_iter()
            .filter(|l| item_ids.contains(&l.item_id))
            .map(|l| l.tag_id)
            .collect();
        all_tags_res
            .get()
            .and_then(Result::ok)
            .unwrap_or_default()
            .into_iter()
            .filter(|t| tag_ids.contains(&t.id))
            .collect::<Vec<Tag>>()
    };

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <Transition fallback=|| view! { <LoadingSpinner/> }>
                // Patch the stores first: this closure runs before the list below in the
                // same render pass, on the server and in the browser.
                {move || match today_res.get() {
                    Some(Ok(data)) => {
                        patch_rows(overdue_rows, data.overdue);
                        patch_rows(today_rows, data.today);
                        ().into_any()
                    }
                    Some(Err(e)) => view! {
                        <p class="text-error">"Błąd: " {e.to_string()}</p>
                    }.into_any(),
                    None => ().into_any(),
                }}

                <Show when=move || matches!(today_res.get(), Some(Ok(_)))>
                    <div class="flex items-center justify-between mb-4">
                        <h1 class="text-2xl font-bold">"Dziś"</h1>
                        <span class="text-base-content/50 text-sm">
                            {move || today_res.get().and_then(Result::ok).map(|d| d.today_date)}
                        </span>
                    </div>

                    <Show
                        when=has_any
                        fallback=|| view! {
                            <p class="text-center text-base-content/50 py-12">"Brak zadań na dziś."</p>
                        }
                    >
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
                                    on:change=move |ev| show_completed.set(!event_target_checked(&ev))
                                />
                            </label>
                        </div>

                        {move || view! {
                            <ListFilterChips
                                lists=unique_lists()
                                hidden_lists=hidden_lists
                                tags=relevant_tags()
                                hidden_tags=hidden_tags
                                show_completed=show_completed
                            />
                        }}

                        <Show
                            when=move || !all_hidden()
                            fallback=|| view! {
                                <div class="text-center text-base-content/50 py-4">
                                    "Wszystkie elementy ukończone lub ukryte."
                                </div>
                            }
                        >
                            <Show when=has_overdue>
                                <div class="mb-6">
                                    <h3 class="text-xs text-error uppercase tracking-wider font-semibold mb-2">
                                        "Zaległe"
                                    </h3>
                                    <ItemGroups
                                        groups=overdue_groups
                                        row=move |id: String| view! {
                                            <ItemCompact store=overdue_rows id=id caps=ROW_CAPS testids=ROW_TESTIDS/>
                                        }.into_any()
                                    />
                                </div>
                            </Show>
                            <Show when=move || !today_groups.with(Vec::is_empty)>
                                <div class="mb-6">
                                    <Show when=has_overdue>
                                        <h3 class="text-xs text-base-content/50 uppercase tracking-wider font-semibold mb-2">
                                            "Dziś"
                                        </h3>
                                    </Show>
                                    <ItemGroups
                                        groups=today_groups
                                        row=move |id: String| view! {
                                            <ItemCompact store=today_rows id=id caps=ROW_CAPS testids=ROW_TESTIDS/>
                                        }.into_any()
                                    />
                                </div>
                            </Show>
                        </Show>
                    </Show>
                </Show>
            </Transition>
        </div>
    }
}
