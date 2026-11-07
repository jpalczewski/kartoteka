//! The list's own items: counts, the hide-completed toggle and the rows, either flat (with
//! drag and drop) or, with the deadlines feature, in overdue / upcoming / done sections.
//!
//! What is visible and in which section is derived from the item store with memos, so the
//! `<For>`s only see id lists change and every row component survives refetches.

use leptos::prelude::*;

use kartoteka_shared::FEATURE_DEADLINES;
use kartoteka_shared::types::Item;

use super::state::ListPageState;
use crate::components::items::item_list::{ItemDnd, ItemRowList};
use crate::components::items::item_row::RowTags;
use crate::components::items::item_view::ItemCaps;
use crate::state::item_store::tracked_items;

/// Ids of the items that pass the page's filters, plus their deadline sections.
#[derive(Clone, Debug, Default, PartialEq)]
struct VisibleIds {
    all: Vec<String>,
    overdue: Vec<String>,
    upcoming: Vec<String>,
    done: Vec<String>,
}

#[component]
pub fn ListItems(
    state: ListPageState,
    show_completed: RwSignal<bool>,
    active_tag: RwSignal<Option<String>>,
    item_dnd: ItemDnd,
) -> impl IntoView {
    let store = state.items;
    let list_id = state.field(|m| m.id.clone());
    let features = state.field(|m| m.features.clone());
    let has_deadlines =
        Memo::new(move |_| features.with(|f| f.iter().any(|f| f == FEATURE_DEADLINES)));

    // Flat rows can be dragged; the date sections keep the server's grouping.
    let caps = Memo::new(move |_| {
        features.with(|f| ItemCaps {
            tags: true,
            delete: true,
            move_to: true,
            ..ItemCaps::from_features(f)
        })
    });
    let flat_caps = Memo::new(move |_| ItemCaps {
        drag: true,
        ..caps.get()
    });
    let move_targets = Memo::new(move |_| {
        let current = list_id.get();
        state.all_lists.with(|lists| {
            lists
                .iter()
                .filter(|(id, _)| *id != current)
                .cloned()
                .collect::<Vec<_>>()
        })
    });
    let tags = RowTags {
        all_tags: state.all_tags.into(),
        links: state.item_tag_links.into(),
    };

    // (completed, total)
    let counts = Memo::new(move |_| {
        let items = tracked_items(store);
        (items.iter().filter(|i| i.completed).count(), items.len())
    });
    let visible = Memo::new(move |_| {
        let tag = active_tag.get();
        let show_completed = show_completed.get();
        let links = if tag.is_some() {
            state.item_tag_links.get()
        } else {
            Vec::new()
        };
        let items: Vec<Item> = tracked_items(store)
            .into_iter()
            .filter(|i| show_completed || !i.completed)
            .filter(|i| {
                tag.as_ref().is_none_or(|tag| {
                    links
                        .iter()
                        .any(|l| l.item_id == i.id && l.tag_id.as_str() == tag.as_str())
                })
            })
            .collect();
        let mut visible = VisibleIds {
            all: items.iter().map(|i| i.id.clone()).collect(),
            ..VisibleIds::default()
        };
        if has_deadlines.get() {
            (visible.overdue, visible.upcoming, visible.done) =
                partition_by_deadline(&items, &state.today_date.get());
        }
        visible
    });
    let all_ids = Memo::new(move |_| visible.with(|v| v.all.clone()));
    let overdue_ids = Memo::new(move |_| visible.with(|v| v.overdue.clone()));
    let upcoming_ids = Memo::new(move |_| visible.with(|v| v.upcoming.clone()));
    let done_ids = Memo::new(move |_| visible.with(|v| v.done.clone()));
    let is_empty = Memo::new(move |_| all_ids.with(Vec::is_empty));

    let section = move |label: &'static str, label_class: &'static str, ids: Memo<Vec<String>>| {
        view! {
            <Show when=move || ids.with(|ids| !ids.is_empty())>
                <div class="mb-4">
                    <p class=format!("text-xs font-semibold uppercase tracking-wide mb-2 {label_class}")>{label}</p>
                    <div class="flex flex-col gap-2">
                        <ItemRowList
                            store=store
                            ids=ids
                            list_id=list_id
                            caps=caps
                            move_targets=move_targets
                            tags=Some(tags)
                        />
                    </div>
                </div>
            </Show>
        }
    };

    view! {
        <Show when=move || is_empty.get()>
            <div class="text-center text-base-content/50 py-8">
                {move || if active_tag.with(Option::is_some) {
                    "Brak elementów z tym tagiem."
                } else if !show_completed.get() && counts.get().0 > 0 {
                    "Wszystkie elementy ukończone — odznacz filtr aby je zobaczyć."
                } else {
                    "Brak elementów. Dodaj pierwszy powyżej."
                }}
            </div>
        </Show>
        <Show when=move || !is_empty.get()>
            <div>
                <div class="flex items-center justify-between mb-2">
                    <span class="text-sm text-base-content/60" data-testid="completion-count">
                        {move || counts.get().0} "/" {move || counts.get().1} " ukończone"
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
                <Show
                    when=move || has_deadlines.get()
                    fallback=move || view! {
                        <div class="flex flex-col">
                            <ItemRowList
                                store=store
                                ids=all_ids
                                list_id=list_id
                                caps=flat_caps
                                move_targets=move_targets
                                tags=Some(tags)
                                dnd=item_dnd
                            />
                        </div>
                    }
                >
                    <div class="flex flex-col">
                        {section("Zaległe", "text-error", overdue_ids)}
                        {section("Nadchodzące", "text-base-content/60", upcoming_ids)}
                        {section("Ukończone", "text-base-content/40", done_ids)}
                    </div>
                </Show>
            </div>
        </Show>
    }
}

/// Split items into (overdue, upcoming, done) id buckets for the deadline date-view.
/// - done: `item.completed`
/// - overdue: not completed and deadline < today
/// - upcoming: everything else (no deadline, deadline >= today)
fn partition_by_deadline(items: &[Item], today: &str) -> (Vec<String>, Vec<String>, Vec<String>) {
    let mut overdue = Vec::new();
    let mut upcoming = Vec::new();
    let mut done = Vec::new();

    for item in items {
        if item.completed {
            done.push(item.id.clone());
        } else if item
            .deadline
            .as_ref()
            .is_some_and(|d| d.start().format("%Y-%m-%d").to_string().as_str() < today)
        {
            overdue.push(item.id.clone());
        } else {
            upcoming.push(item.id.clone());
        }
    }

    (overdue, upcoming, done)
}
