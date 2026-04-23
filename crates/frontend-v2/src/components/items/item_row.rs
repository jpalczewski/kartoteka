use kartoteka_shared::types::{Item, Tag};
use leptos::prelude::*;
use leptos_router::components::A;
use web_sys::DragEvent;

use super::date_field::DateFieldInput;
use super::quantity_stepper::QuantityStepper;
use crate::components::common::dnd::ItemDragHandleButton;
use crate::components::tags::tag_badge::TagBadge;
use crate::state::dnd::{DraggedItem, ItemDndState, begin_item_drag, clear_item_dnd_state};

pub type DateSavePayload = (String, Option<String>, Option<String>, Option<String>);

fn flex_date_display(item: &Item) -> Option<(String, &'static str, bool)> {
    // Show the most important date: hard_deadline > deadline > start_date
    if let Some(ref d) = item.hard_deadline {
        return Some((d.to_string(), "🚨", false));
    }
    if let Some(ref d) = item.deadline {
        return Some((d.to_string(), "⏰", false));
    }
    if let Some(ref d) = item.start_date {
        return Some((d.to_string(), "📅", true));
    }
    None
}

#[component]
pub fn ItemRow(
    item: Item,
    on_toggle: Callback<String>,
    on_delete: Callback<String>,
    #[prop(default = false)] has_quantity: bool,
    #[prop(default = vec![])] item_tags: Vec<Tag>,
    #[prop(default = vec![])] move_targets: Vec<(String, String)>,
    #[prop(optional)] on_move: Option<Callback<(String, String)>>,
    #[prop(optional)] on_quantity_change: Option<Callback<(String, i32)>>,
    #[prop(optional)] on_description_save: Option<Callback<(String, String)>>,
    #[prop(optional)] on_date_save: Option<Callback<DateSavePayload>>,
    /// When provided, the row gains a drag handle and participates in DnD.
    #[prop(optional)]
    dnd_state: Option<RwSignal<ItemDndState>>,
) -> impl IntoView {
    let item_id_toggle = item.id.clone();
    let item_id_delete = item.id.clone();
    let item_id_qty = item.id.clone();
    let item_id_desc = item.id.clone();
    let item_id_dates = item.id.clone();
    let item_id_move = item.id.clone();
    let completed = item.completed;
    let item_href = format!("/lists/{}/items/{}", item.list_id, item.id);
    let link_class = if completed {
        "flex-1 text-base-content/50 line-through hover:text-base-content/80"
    } else {
        "flex-1 text-base-content hover:text-primary"
    };

    let expanded = RwSignal::new(false);
    let desc_text = RwSignal::new(item.description.clone().unwrap_or_default());

    let start_input = RwSignal::new(
        item.start_date
            .as_ref()
            .map(|d| d.to_string())
            .unwrap_or_default(),
    );
    let deadline_input = RwSignal::new(
        item.deadline
            .as_ref()
            .map(|d| d.to_string())
            .unwrap_or_default(),
    );
    let hard_deadline_input = RwSignal::new(
        item.hard_deadline
            .as_ref()
            .map(|d| d.to_string())
            .unwrap_or_default(),
    );

    let date_info = flex_date_display(&item);

    let show_stepper = has_quantity && item.quantity.is_some();
    let stepper = show_stepper.then(|| {
        let qty_cb = Callback::new(move |new_actual: i32| {
            if let Some(cb) = on_quantity_change {
                cb.run((item_id_qty.clone(), new_actual));
            }
        });
        view! {
            <QuantityStepper
                actual=item.actual_quantity.unwrap_or(0)
                target=item.quantity
                unit=item.unit.clone()
                on_change=qty_cb
            />
        }
    });

    let dragged_item = dnd_state.map(|_| DraggedItem {
        item_id: item.id.clone(),
        source_list_id: item.list_id.clone(),
    });
    let dragged_item_for_start = dragged_item.clone();

    view! {
        <div
            class="p-3 bg-base-200 rounded-lg"
            draggable=if dnd_state.is_some() { "true" } else { "false" }
            on:dragstart=move |ev: DragEvent| {
                if let Some((state, di)) = dnd_state.zip(dragged_item_for_start.clone()) {
                    if let Some(dt) = ev.data_transfer() {
                        let _ = dt.set_data("text/plain", &di.item_id);
                        dt.set_effect_allowed("move");
                    }
                    state.update(|s| begin_item_drag(s, di));
                }
            }
            on:dragend=move |_| {
                if let Some(state) = dnd_state {
                    state.update(clear_item_dnd_state);
                }
            }
        >
                <div class="flex items-center gap-3">
                {dnd_state.zip(dragged_item.clone()).map(|(state, di)| view! {
                    <ItemDragHandleButton dnd_state=state dragged_item=di aria_label="Przeciągnij element" />
                })}
                <input
                    type="checkbox"
                    class="checkbox checkbox-primary"
                    data-testid="item-toggle"
                    checked=completed
                    on:change=move |_| on_toggle.run(item_id_toggle.clone())
                />
                <button
                    type="button"
                    class="btn btn-ghost btn-xs btn-square"
                    title="Rozwiń"
                    on:click=move |_| expanded.update(|e| *e = !*e)
                >
                    {move || if expanded.get() { "▲" } else { "▼" }}
                </button>
                <A href=item_href attr:class=link_class>
                    {item.title.clone()}
                </A>
                {(!item_tags.is_empty()).then(|| view! {
                    <div class="flex gap-1 shrink-0">
                        {item_tags.into_iter().map(|tag| view! {
                            <TagBadge tag=tag />
                        }).collect::<Vec<_>>()}
                    </div>
                })}
                {date_info.map(|(date_str, icon, _is_start)| {
                    let date_color = if completed {
                        "text-xs text-base-content/40 shrink-0"
                    } else {
                        "text-xs text-base-content/60 shrink-0"
                    };
                    view! {
                        <span
                            class=format!("{date_color} cursor-pointer hover:opacity-80")
                            title="Kliknij ▼ aby edytować daty"
                        >
                            {icon} {date_str}
                        </span>
                    }
                })}
                {stepper}
                {on_move
                    .filter(|_| !move_targets.is_empty())
                    .map(|cb| {
                        let open = RwSignal::new(false);
                        let targets = StoredValue::new(move_targets.clone());
                        let iid = item_id_move.clone();
                        view! {
                            <div class="relative">
                                <button
                                    type="button"
                                    class="btn btn-ghost btn-xs btn-square"
                                    title="Przenieś do…"
                                    data-testid="item-move-btn"
                                    on:click=move |_| open.update(|v| *v = !*v)
                                >{"↗"}</button>
                                <div
                                    class="absolute right-0 top-full mt-1 bg-base-200 border border-base-300 rounded-box min-w-44 max-h-60 overflow-y-auto z-50 p-2 shadow-lg"
                                    style:display=move || if open.get() { "block" } else { "none" }
                                    data-testid="item-move-menu"
                                >
                                    {move || targets.get_value().into_iter().map(|(tid, tname)| {
                                        let iid = iid.clone();
                                        view! {
                                            <button type="button"
                                                class="flex items-center gap-2 px-2 py-1.5 text-sm rounded cursor-pointer hover:bg-base-300 w-full text-left"
                                                on:click=move |_| {
                                                    open.set(false);
                                                    cb.run((iid.clone(), tid.clone()));
                                                }
                                            >{tname.clone()}</button>
                                        }
                                    }).collect::<Vec<_>>()}
                                </div>
                            </div>
                        }
                    })
                }
                <button
                    type="button"
                    class="btn btn-ghost btn-xs btn-circle text-error"
                    on:click=move |_| on_delete.run(item_id_delete.clone())
                >
                    {"✕"}
                </button>
            </div>

            {move || {
                if expanded.get() {
                    let id_desc = item_id_desc.clone();
                    let id_dates = item_id_dates.clone();
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
                                    if let Some(cb) = on_description_save {
                                        cb.run((id_desc.clone(), desc_text.get_untracked()));
                                    }
                                }
                            >
                                "Zapisz opis"
                            </button>

                            {on_date_save.map(|save_cb| {
                                let id = id_dates.clone();
                                view! {
                                    <div class="flex flex-col gap-1 text-sm">
                                        <DateFieldInput label="📅 Rozpoczęcie" value=start_input show_clear=true/>
                                        <DateFieldInput label="⏰ Termin" value=deadline_input show_clear=true/>
                                        <DateFieldInput label="🚨 Ostateczny" value=hard_deadline_input show_clear=true/>
                                        <button
                                            type="button"
                                            class="btn btn-xs btn-primary self-start mt-1"
                                            on:click=move |_| {
                                                save_cb.run((
                                                    id.clone(),
                                                    Some(start_input.get_untracked()),
                                                    Some(deadline_input.get_untracked()),
                                                    Some(hard_deadline_input.get_untracked()),
                                                ));
                                            }
                                        >
                                            "Zapisz daty"
                                        </button>
                                    </div>
                                }
                            })}
                        </div>
                    }.into_any()
                } else {
                    let desc = desc_text.get();
                    if desc.is_empty() {
                        view! {}.into_any()
                    } else {
                        view! {
                            <p class="pl-16 pt-1 text-sm text-base-content/60">{desc}</p>
                        }.into_any()
                    }
                }
            }}
        </div>
    }
}
