//! The `Row` variant of an item view: the full list row with expandable editor, quantity
//! stepper, tags, action menu, move-to and drag handle.
//!
//! A row is created once per item id (keyed `<For>` in [`super::item_list::ItemRowList`]) and
//! reads its item from the page's keyed store, so a refetch updates the changed fields in
//! place and keeps the open editor, focus and typed text. What the row shows is decided by
//! [`ItemCaps`], which the page derives from the list's features.

use kartoteka_shared::types::{FlexDate, Item, ItemTagLink, Tag};
use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::components::A;
use reactive_stores::Store;
use web_sys::DragEvent;

use super::date_field::DateFieldInput;
use super::item_view::ItemCaps;
use super::parts::{
    DateChips, EstimateChip, ItemCheckbox, ItemIcon, ItemTitle, RecurrenceChip, StatusChip,
};
use super::quantity_stepper::QuantityStepper;
use super::row_editors::{EstimateEditor, IconEditor, RecurrenceEditor, StatusEditor};
use crate::components::common::dnd::ItemDragHandleButton;
use crate::components::tags::tag_badge::TagBadge;
use crate::components::tags::tag_selector_dropdown::TagSelectorDropdown;
use crate::components::ui::{IconButton, IconButtonVariant};
use crate::state::dnd::{DraggedItem, ItemDndState, begin_item_drag, clear_item_dnd_state};
use crate::state::item_actions::{toggle_item_tag, use_item_actions};
use crate::state::item_store::{ItemRows, item_signal, row_item};

pub use crate::state::item_actions::DateSavePayload;

/// Where a row gets its tags from: the tag catalog and the item–tag links of the page.
#[derive(Clone, Copy)]
pub struct RowTags {
    pub all_tags: Signal<Vec<Tag>>,
    pub links: Signal<Vec<ItemTagLink>>,
}

fn date_input(date: Option<&FlexDate>) -> String {
    date.map(ToString::to_string).unwrap_or_default()
}

/// One list row. `caps` decides which parts and actions show; `tags` feeds the tag chips and
/// the tag selector (`caps.tags`); `move_targets` are the `(list id, name)` pairs of the move
/// menu (`caps.move_to`); `dnd_state` makes the row draggable (`caps.drag`).
#[component]
pub fn ItemRow(
    store: Store<ItemRows<Item>>,
    id: String,
    #[prop(into)] caps: Signal<ItemCaps>,
    #[prop(into)] move_targets: Signal<Vec<(String, String)>>,
    #[prop(optional_no_strip)] tags: Option<RowTags>,
    #[prop(optional_no_strip)] dnd_state: Option<RwSignal<ItemDndState>>,
) -> impl IntoView {
    let actions = use_item_actions();
    let item = row_item(store, id.clone());
    let item_sig = item_signal(item);

    let checkbox = Memo::new(move |_| caps.with(|c| c.checkbox));
    let dates = Memo::new(move |_| caps.with(|c| c.dates));
    let quantity = Memo::new(move |_| caps.with(|c| c.quantity));
    let time = Memo::new(move |_| caps.with(|c| c.time));
    let show_tags = Memo::new(move |_| caps.with(|c| c.tags));
    let delete = Memo::new(move |_| caps.with(|c| c.delete));
    let drag = Memo::new(move |_| caps.with(|c| c.drag));
    let move_to = Memo::new(move |_| caps.with(|c| c.move_to));

    let completed = Memo::new(move |_| item.with(|i| i.as_ref().is_some_and(|i| i.completed)));
    let description = Memo::new(move |_| {
        item.with(|i| {
            i.as_ref()
                .and_then(|i| i.description.clone())
                .filter(|d| !d.is_empty())
        })
    });
    let stepper_quantity = Memo::new(move |_| item.with(|i| i.as_ref().and_then(|i| i.quantity)));

    let expanded = RwSignal::new(false);
    let expand_label = move_tr!("common-expand");
    let collapse_label = move_tr!("common-collapse");

    // ── Drag and drop ──
    let dnd = move || dnd_state.filter(|_| drag.get());
    let dragged_item = move || {
        item.with_untracked(|i| {
            i.as_ref().map(|i| DraggedItem {
                item_id: i.id.clone(),
                source_list_id: i.list_id.clone(),
            })
        })
    };

    // ── Tags ──
    let item_tags = Memo::new({
        let id = id.clone();
        move |_| {
            let Some(tags) = tags.filter(|_| show_tags.get()) else {
                return Vec::new();
            };
            let links = tags.links.get();
            tags.all_tags.with(|all| {
                links
                    .iter()
                    .filter(|l| l.item_id == id)
                    .filter_map(|l| all.iter().find(|t| t.id == l.tag_id).cloned())
                    .collect::<Vec<_>>()
            })
        }
    });
    let on_tag_toggle = Callback::new({
        let id = id.clone();
        move |tag_id: String| {
            let Some(tags) = tags else { return };
            let is_assigned = tags
                .links
                .with_untracked(|ls| ls.iter().any(|l| l.item_id == id && l.tag_id == tag_id));
            toggle_item_tag(id.clone(), tag_id, is_assigned);
        }
    });

    // ── Move menu ──
    let move_open = RwSignal::new(false);
    let show_move = Memo::new(move |_| move_to.get() && move_targets.with(|t| !t.is_empty()));
    let id_stepper = id.clone();
    let id_move = id.clone();
    let id_delete = id.clone();
    let id_editor = id;

    view! {
        <div
            class="p-3 bg-base-200 rounded-lg"
            data-testid="item-row"
            draggable=move || if dnd().is_some() { "true" } else { "false" }
            on:dragstart=move |ev: DragEvent| {
                if let Some((state, di)) = dnd().zip(dragged_item()) {
                    if let Some(dt) = ev.data_transfer() {
                        let _ = dt.set_data("text/plain", &di.item_id);
                        dt.set_effect_allowed("move");
                    }
                    state.update(|s| begin_item_drag(s, di));
                }
            }
            on:dragend=move |_| {
                if let Some(state) = dnd() {
                    state.update(clear_item_dnd_state);
                }
            }
        >
            <div class="flex items-center gap-3">
                {move || dnd().zip(dragged_item()).map(|(state, di)| view! {
                    <ItemDragHandleButton dnd_state=state dragged_item=di aria_label="Przeciągnij element" />
                })}
                <Show when=move || checkbox.get()>
                    <ItemCheckbox
                        item=item_sig
                        on_toggle=actions.toggle
                        testid=Some("item-toggle")
                    />
                </Show>
                <IconButton
                    icon=Signal::derive(move || {
                        if expanded.get() { "chevron-up".to_string() } else { "chevron-down".to_string() }
                    })
                    label=Signal::derive(move || {
                        if expanded.get() { collapse_label.get() } else { expand_label.get() }
                    })
                    testid=Some("item-expand-btn")
                    on_click=Callback::new(move |_| expanded.update(|e| *e = !*e))
                />
                <ItemIcon item=item_sig />
                <ItemTitle item=item_sig testid=Some("item-title-link") flex=true />
                <StatusChip item=item_sig />
                <Show when=move || item_tags.with(|t| !t.is_empty())>
                    <div class="flex gap-1 shrink-0">
                        {move || item_tags.get().into_iter().map(|tag| {
                            let href = format!("/tags/{}", tag.id);
                            view! {
                                <A href=href attr:class="no-underline">
                                    <TagBadge tag=tag />
                                </A>
                            }
                        }).collect_view()}
                    </div>
                </Show>
                <Show when=move || dates.get()>
                    // The chips open the date editor (until the item is completed).
                    <div
                        class="flex gap-1 flex-wrap shrink-0 empty:hidden"
                        class:cursor-pointer=move || !completed.get()
                        on:click=move |_| {
                            if !completed.get_untracked() {
                                expanded.set(true);
                            }
                        }
                    >
                        <DateChips item=item_sig />
                    </div>
                </Show>
                <RecurrenceChip item=item_sig />
                <Show when=move || time.get()>
                    <EstimateChip item=item_sig />
                </Show>
                <Show when=move || quantity.get() && stepper_quantity.get().is_some()>
                    <QuantityStepper
                        actual=Signal::derive(move || {
                            item.with(|i| i.as_ref().and_then(|i| i.actual_quantity).unwrap_or(0))
                        })
                        target=stepper_quantity
                        unit=Signal::derive(move || item.with(|i| i.as_ref().and_then(|i| i.unit.clone())))
                        on_change=Callback::new({
                            let id = id_stepper.clone();
                            move |new_actual: i32| actions.set_actual_quantity.run((id.clone(), new_actual))
                        })
                    />
                </Show>
                <Show when=move || show_move.get()>
                    <div class="relative">
                        <IconButton
                            icon="arrow-up-right"
                            label=move_tr!("item-move-to")
                            testid=Some("item-move-btn")
                            on_click=Callback::new(move |_| move_open.update(|v| *v = !*v))
                        />
                        <div
                            class="absolute right-0 top-full mt-1 bg-base-200 border border-base-300 rounded-box min-w-44 max-h-60 overflow-y-auto z-50 p-2 shadow-lg"
                            style:display=move || if move_open.get() { "block" } else { "none" }
                            data-testid="item-move-menu"
                        >
                            {
                                let id = id_move.clone();
                                move || move_targets.get().into_iter().map(|(tid, tname)| {
                                    let id = id.clone();
                                    view! {
                                        <button type="button"
                                            class="flex items-center gap-2 px-2 py-1.5 text-sm rounded cursor-pointer hover:bg-base-300 w-full text-left"
                                            on:click=move |_| {
                                                move_open.set(false);
                                                actions.move_to_list.run((id.clone(), tid.clone()));
                                            }
                                        >{tname.clone()}</button>
                                    }
                                }).collect_view()
                            }
                        </div>
                    </div>
                </Show>
                {move || tags.filter(|_| show_tags.get()).map(|tags| view! {
                    <TagSelectorDropdown
                        all_tags=tags.all_tags.get()
                        item_tags=item_tags.get()
                        on_toggle=on_tag_toggle
                    />
                })}
                <Show when=move || delete.get()>
                    <IconButton
                        icon="x"
                        label=move_tr!("common-delete")
                        variant=IconButtonVariant::Danger
                        testid=Some("item-delete-btn")
                        on_click=Callback::new({
                            let id = id_delete.clone();
                            move |_| actions.delete.run(id.clone())
                        })
                    />
                </Show>
            </div>

            <Show when=move || expanded.get()>
                <ItemEditor item=item id=id_editor.clone() caps=caps />
            </Show>
            {move || (!expanded.get()).then(|| description.get()).flatten().map(|desc| view! {
                <p class="pl-16 pt-1 text-sm text-base-content/60">{desc}</p>
            })}
        </div>
    }
}

/// The expanded part of a row: description, status, and what the list's features allow:
/// date fields and recurrence (deadlines), estimate (time tracking). The drafts start from
/// the item as it is when the editor opens; later updates of the item do not overwrite what
/// is being typed.
#[component]
fn ItemEditor(item: Memo<Option<Item>>, id: String, caps: Signal<ItemCaps>) -> impl IntoView {
    let actions = use_item_actions();
    let dates = Memo::new(move |_| caps.with(|c| c.dates));
    let time = Memo::new(move |_| caps.with(|c| c.time));
    let checkbox = Memo::new(move |_| caps.with(|c| c.checkbox));
    let current = item.get_untracked();
    let current = current.as_ref();

    let desc_text = RwSignal::new(
        current
            .and_then(|i| i.description.clone())
            .unwrap_or_default(),
    );
    let start_input = RwSignal::new(date_input(current.and_then(|i| i.start_date.as_ref())));
    let start_time_input = RwSignal::new(
        current
            .and_then(|i| i.start_time.clone())
            .unwrap_or_default(),
    );
    let deadline_input = RwSignal::new(date_input(current.and_then(|i| i.deadline.as_ref())));
    let deadline_time_input = RwSignal::new(
        current
            .and_then(|i| i.deadline_time.clone())
            .unwrap_or_default(),
    );
    let hard_deadline_input =
        RwSignal::new(date_input(current.and_then(|i| i.hard_deadline.as_ref())));

    let id_desc = id.clone();
    let id_status = id.clone();
    let id_icon = id.clone();
    let id_recurrence = id.clone();
    let id_estimate = id.clone();
    view! {
        <div class="pl-16 pt-2 flex flex-col gap-2">
            <textarea
                class="textarea textarea-bordered w-full text-sm h-20"
                placeholder="Opis..."
                prop:value=move || desc_text.get()
                on:input=move |ev| desc_text.set(event_target_value(&ev))
            />
            <button
                type="button"
                class="btn btn-xs btn-primary self-start"
                on:click=move |_| {
                    actions.save_description.run((id_desc.clone(), desc_text.get_untracked()));
                }
            >
                "Zapisz opis"
            </button>

            <div class="flex flex-col gap-2 text-sm">
                <StatusEditor item=item id=id_status can_complete=checkbox />
                <IconEditor item=item id=id_icon />
                <Show when=move || time.get()>
                    <EstimateEditor item=item id=id_estimate.clone() />
                </Show>
            </div>

            <Show when=move || dates.get()>
                <div class="flex flex-col gap-2 text-sm">
                    <DateFieldInput
                        label=move_tr!("item-dates-start")
                        value=start_input
                        time_value=start_time_input
                        show_clear=true
                        show_quick=true
                    />
                    <DateFieldInput
                        label=move_tr!("item-dates-deadline")
                        value=deadline_input
                        time_value=deadline_time_input
                        show_clear=true
                        show_quick=true
                    />
                    <DateFieldInput
                        label=move_tr!("item-dates-hard-deadline")
                        value=hard_deadline_input
                        show_clear=true
                        show_quick=true
                    />
                    <button
                        type="button"
                        class="btn btn-xs btn-primary self-start mt-1"
                        on:click={
                            let id = id.clone();
                            move |_| {
                                actions.save_dates.run((
                                    id.clone(),
                                    Some(start_input.get_untracked()),
                                    Some(start_time_input.get_untracked()),
                                    Some(deadline_input.get_untracked()),
                                    Some(deadline_time_input.get_untracked()),
                                    Some(hard_deadline_input.get_untracked()),
                                ));
                            }
                        }
                    >
                        "Zapisz daty"
                    </button>
                    <RecurrenceEditor item=item id=id_recurrence.clone() />
                </div>
            </Show>
        </div>
    }
}
