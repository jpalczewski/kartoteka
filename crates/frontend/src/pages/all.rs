use leptos::prelude::*;
use leptos_fluent::move_tr;
use reactive_stores::Store;

use kartoteka_shared::ChangeTopic;
use kartoteka_shared::types::DateItem;

use crate::components::common::loading::LoadingSpinner;
use crate::components::items::item_compact::{CompactTestIds, ItemCompact};
use crate::components::items::item_groups::{ItemGroups, group_ids};
use crate::components::items::item_view::ItemCaps;
use crate::server_fns::items::get_all_items;
use crate::state::invalidation::use_invalidation;
use crate::state::item_store::{ItemRows, patch_rows, tracked_items};

const ROW_CAPS: ItemCaps = ItemCaps {
    checkbox: true,
    dates: true,
    quantity: true,
    ..ItemCaps::NONE
};

const ROW_TESTIDS: CompactTestIds = CompactTestIds {
    row: Some("all-row"),
    toggle: Some("all-item-toggle"),
    title: None,
    description: Some("all-item-description"),
};

#[component]
pub fn AllPage() -> impl IntoView {
    let invalidation = use_invalidation();
    let (show_completed, set_show_completed) = signal(true);

    let all_res = Resource::new(
        move || invalidation.track(&[ChangeTopic::Items, ChangeTopic::Lists]),
        |_| get_all_items(),
    );

    // Row data lives in a keyed store, so a refetch (e.g. a live update) updates the
    // changed rows in place instead of re-creating every row.
    let rows = Store::new(ItemRows::<DateItem>::default());

    // Everything structural is derived from the store, so a changed `completed` flag
    // recounts and regroups even though the set of ids did not change.
    let total = Memo::new(move |_| tracked_items(rows).len());
    let completed_count = Memo::new(move |_| {
        tracked_items(rows)
            .iter()
            .filter(|di| di.item.completed)
            .count()
    });
    let progress = Memo::new(move |_| format!("{}/{}", completed_count.get(), total.get()));
    let has_items = move || total.get() > 0;

    let groups = Memo::new(move |_| {
        let show = show_completed.get();
        let visible: Vec<DateItem> = tracked_items(rows)
            .into_iter()
            .filter(|di| show || !di.item.completed)
            .collect();
        group_ids(&visible)
    });
    let all_hidden = move || groups.with(Vec::is_empty);

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <Transition fallback=|| view! { <LoadingSpinner/> }>
                // Patch the store first: this closure runs before the list below in the
                // same render pass, on the server and in the browser.
                {move || match all_res.get() {
                    Some(Ok(items)) => {
                        patch_rows(rows, items);
                        ().into_any()
                    }
                    Some(Err(e)) => {
                        let detail = e.to_string();
                        view! {
                            <p class="text-error">
                                {move_tr!("all-loading-error", { "detail" => detail.clone() })}
                            </p>
                        }.into_any()
                    }
                    None => ().into_any(),
                }}

                <Show when=move || matches!(all_res.get(), Some(Ok(_)))>
                    <div>
                        <div class="flex items-center justify-between mb-6">
                            <h1 class="text-2xl font-bold">{move_tr!("all-title")}</h1>
                        </div>

                        <Show
                            when=has_items
                            fallback=|| view! {
                                <p class="text-center text-base-content/50 py-12">
                                    {move_tr!("all-empty")}
                                </p>
                            }
                        >
                            <div>
                                <div class="flex items-center justify-between mb-2">
                                    <span class="text-sm text-base-content/60" data-testid="all-completion-count">
                                        {move_tr!("all-completed-count", { "progress" => progress.get() })}
                                    </span>
                                    <label class="flex items-center gap-2 cursor-pointer select-none">
                                        <span class="text-xs text-base-content/50">{move_tr!("all-hide-completed")}</span>
                                        <input
                                            type="checkbox"
                                            class="toggle toggle-xs"
                                            data-testid="hide-completed-toggle"
                                            prop:checked=move || !show_completed.get()
                                            on:change=move |ev| set_show_completed.set(!event_target_checked(&ev))
                                        />
                                    </label>
                                </div>

                                <Show
                                    when=move || !all_hidden()
                                    fallback=|| view! {
                                        <div class="text-center text-base-content/50 py-4">
                                            {move_tr!("all-all-completed")}
                                        </div>
                                    }
                                >
                                    <ItemGroups
                                        groups=groups
                                        row=move |id: String| view! {
                                            <ItemCompact store=rows id=id caps=ROW_CAPS testids=ROW_TESTIDS/>
                                        }.into_any()
                                    />
                                </Show>
                            </div>
                        </Show>
                    </div>
                </Show>
            </Transition>
        </div>
    }
}
