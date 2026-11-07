use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::components::A;
use reactive_stores::Store;

use kartoteka_shared::ChangeTopic;
use kartoteka_shared::types::DateItem;

use crate::components::common::loading::LoadingSpinner;
use crate::components::items::item_groups::{ItemGroups, group_ids};
use crate::server_fns::items::get_all_items;
use crate::state::invalidation::use_invalidation;
use crate::state::item_actions::use_item_actions;
use crate::state::item_store::{ItemRows, patch_rows, row_item, tracked_items};

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
                                        row=move |id: String| view! { <AllRow store=rows id=id/> }.into_any()
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

/// Notes and log lists have no completion concept, so their items must not render a checkbox.
fn shows_checkbox(list_type: &str) -> bool {
    !matches!(list_type, "notes" | "log")
}

/// One item row. Created once per item id and kept across refetches; every displayed
/// field reads the keyed store, so only what changed re-renders.
#[component]
fn AllRow(store: Store<ItemRows<DateItem>>, id: String) -> impl IntoView {
    let item = row_item(store, id.clone());
    let id = StoredValue::new(id);
    let actions = use_item_actions();

    let completed = move || item.with(|di| di.as_ref().is_some_and(|di| di.item.completed));
    let title = move || item.with(|di| di.as_ref().map(|di| di.item.title.clone()));
    let href = move || {
        item.with(|di| {
            di.as_ref()
                .map(|di| format!("/lists/{}/items/{}", di.item.list_id, di.item.id))
                .unwrap_or_default()
        })
    };
    let deadline = move || {
        item.with(|di| {
            di.as_ref()
                .and_then(|di| di.item.deadline.as_ref().map(ToString::to_string))
        })
    };
    let checkbox = Memo::new(move |_| {
        item.with(|di| di.as_ref().is_some_and(|di| shows_checkbox(&di.list_type)))
    });
    // Shown instead of the checkbox for notes and log items, when they have text.
    let description = Memo::new(move |_| {
        item.with(|di| {
            di.as_ref()
                .filter(|di| !shows_checkbox(&di.list_type))
                .and_then(|di| di.item.description.clone())
                .filter(|d| !d.trim().is_empty())
        })
    });
    let has_description = Memo::new(move |_| description.with(Option::is_some));

    view! {
        {move || if has_description.get() {
            view! {
                <div class="flex items-start gap-3 p-2 bg-base-200 rounded-lg">
                    <div class="flex-1 min-w-0">
                        <A
                            href=href
                            attr:class=move || if completed() {
                                "text-sm text-base-content/50 line-through"
                            } else {
                                "text-sm text-base-content hover:text-primary"
                            }
                        >
                            {title}
                        </A>
                        <p
                            class="text-xs text-base-content/60 line-clamp-2"
                            data-testid="all-item-description"
                        >
                            {move || description.get().unwrap_or_default()}
                        </p>
                    </div>
                    {move || deadline().map(|d| view! {
                        <span class="text-xs text-error/70">{d}</span>
                    })}
                </div>
            }.into_any()
        } else {
            view! {
                <div class="flex items-center gap-3 p-2 bg-base-200 rounded-lg">
                    {move || checkbox.get().then(|| view! {
                        <input
                            type="checkbox"
                            class="checkbox checkbox-sm checkbox-primary"
                            data-testid="all-item-toggle"
                            prop:checked=completed
                            on:change=move |_| actions.toggle.run(id.get_value())
                        />
                    })}
                    <A
                        href=href
                        attr:class=move || if completed() {
                            "flex-1 text-sm text-base-content/50 line-through"
                        } else {
                            "flex-1 text-sm text-base-content hover:text-primary"
                        }
                    >
                        {title}
                    </A>
                    {move || deadline().map(|d| view! {
                        <span class="text-xs text-error/70">{d}</span>
                    })}
                </div>
            }.into_any()
        }}
    }
}

#[cfg(test)]
mod tests {
    use super::shows_checkbox;

    #[test]
    fn checkbox_only_for_completable_list_types() {
        assert!(shows_checkbox("checklist"));
        assert!(shows_checkbox("shopping"));
        assert!(shows_checkbox("schedule"));
        assert!(!shows_checkbox("log"));
        assert!(!shows_checkbox("notes"));
    }
}
