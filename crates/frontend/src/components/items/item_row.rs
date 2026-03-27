use kartoteka_shared::Item;
use kartoteka_shared::Tag;
use leptos::prelude::*;

use crate::components::common::date_utils::item_date_badges;
use crate::components::items::date_editor::DateEditor;
use crate::components::tags::tag_list::TagList;

#[component]
pub fn ItemRow(
    item: Item,
    on_toggle: Callback<String>,
    on_delete: Callback<String>,
    #[prop(default = vec![])] all_tags: Vec<Tag>,
    #[prop(default = vec![])] item_tag_ids: Vec<String>,
    #[prop(optional)] on_tag_toggle: Option<Callback<String>>,
    #[prop(optional)] on_description_save: Option<Callback<(String, String)>>,
    #[prop(default = false)] has_quantity: bool,
    #[prop(optional)] on_quantity_change: Option<Callback<(String, i32)>>,
    #[prop(default = vec![])] move_targets: Vec<(String, String)>,
    #[prop(optional)] on_move: Option<Callback<(String, String)>>,
    /// (item_id, date_type, date_value, time_value)
    #[prop(optional)]
    on_date_save: Option<Callback<(String, String, String, Option<String>)>>,
    /// Deadlines feature config for ghost chips
    #[prop(default = serde_json::Value::Null)]
    deadlines_config: serde_json::Value,
) -> impl IntoView {
    let id = item.id.clone();
    let id_toggle = id.clone();
    let id_delete = id.clone();
    let id_move = id.clone();
    let id_for_editor = id.clone();
    let id_for_desc = id.clone();
    let completed = item.completed;

    let row_class = if completed {
        "flex items-center gap-3 py-2 opacity-50"
    } else {
        "flex items-center gap-3 py-2"
    };

    let title_class = if completed {
        "flex-1 line-through text-base-content/50"
    } else {
        "flex-1"
    };

    let expanded = RwSignal::new(false);
    let description_text = RwSignal::new(item.description.clone().unwrap_or_default());

    // Quantity stepper state
    let show_stepper = has_quantity && item.quantity.is_some();
    let target_qty = item.quantity.unwrap_or(0);
    let actual = RwSignal::new(item.actual_quantity.unwrap_or(0));
    let unit_label = item.unit.clone().unwrap_or_default();
    let id_for_stepper = id.clone();

    let has_tags = !item_tag_ids.is_empty() || on_tag_toggle.is_some();

    // Date editing state
    let editing_date = RwSignal::new(Option::<String>::None);
    let date_badges = item_date_badges(&item, None);

    // Ghost chips: date types enabled in config but not set on this item
    let cfg_start = deadlines_config
        .get("has_start_date")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let cfg_deadline = deadlines_config
        .get("has_deadline")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let cfg_hard = deadlines_config
        .get("has_hard_deadline")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let ghost_start = cfg_start && item.start_date.is_none();
    let ghost_deadline = cfg_deadline && item.deadline.is_none();
    let ghost_hard = cfg_hard && item.hard_deadline.is_none();
    let has_ghosts = ghost_start || ghost_deadline || ghost_hard;

    // Store initial values for editor
    let item_start_date = item.start_date.clone();
    let item_start_time = item.start_time.clone();
    let item_deadline = item.deadline.clone();
    let item_deadline_time = item.deadline_time.clone();
    let item_hard_deadline = item.hard_deadline.clone();

    view! {
        <div class="border-b border-base-300 py-1">
            // Row 1: checkbox, expand, title, date badges, quantity, actions
            <div class=row_class>
                <input
                    type="checkbox"
                    class="checkbox checkbox-secondary checkbox-sm"
                    checked=item.completed
                    on:change=move |_| on_toggle.run(id_toggle.clone())
                />
                <button
                    type="button"
                    aria-label="Rozwiń opis"
                    class="btn btn-ghost btn-xs btn-square"
                    on:click=move |_| expanded.update(|e| *e = !*e)
                >
                    {move || if expanded.get() { "\u{25B2}" } else { "\u{25BC}" }}
                </button>
                <span class=title_class>{item.title}</span>

                // Date badges (clickable)
                {
                    if date_badges.is_empty() && !has_ghosts {
                        view! {}.into_any()
                    } else {
                        view! {
                            <div class="flex gap-1 flex-wrap shrink-0">
                                {date_badges.into_iter().map(|b| {
                                    let dt = b.date_type.to_string();
                                    view! {
                                        <button type="button" class=format!("{} cursor-pointer", b.css)
                                            on:click=move |_| {
                                                let current = editing_date.get();
                                                if current.as_deref() == Some(dt.as_str()) {
                                                    editing_date.set(None);
                                                } else {
                                                    editing_date.set(Some(dt.clone()));
                                                }
                                            }
                                        >{b.label}</button>
                                    }
                                }).collect::<Vec<_>>()}
                                // Ghost chips for missing date types
                                {if ghost_start {
                                    view! {
                                        <button type="button" class="badge badge-ghost badge-sm opacity-40 cursor-pointer"
                                            on:click=move |_| editing_date.set(Some("start".into()))
                                        >"+\u{1F4C5}"</button>
                                    }.into_any()
                                } else { view! {}.into_any() }}
                                {if ghost_deadline {
                                    view! {
                                        <button type="button" class="badge badge-ghost badge-sm opacity-40 cursor-pointer"
                                            on:click=move |_| editing_date.set(Some("deadline".into()))
                                        >"+\u{23F0}"</button>
                                    }.into_any()
                                } else { view! {}.into_any() }}
                                {if ghost_hard {
                                    view! {
                                        <button type="button" class="badge badge-ghost badge-sm opacity-40 cursor-pointer"
                                            on:click=move |_| editing_date.set(Some("hard_deadline".into()))
                                        >"+\u{1F6A8}"</button>
                                    }.into_any()
                                } else { view! {}.into_any() }}
                            </div>
                        }.into_any()
                    }
                }

                // Quantity stepper
                {if show_stepper {
                    let id_dec = id_for_stepper.clone();
                    let id_inc = id_for_stepper.clone();
                    let cb_dec = on_quantity_change;
                    let cb_inc = on_quantity_change;
                    let unit_str = unit_label.clone();
                    view! {
                        <div class="flex flex-col items-center gap-0.5">
                            <div class="flex items-center gap-1">
                                <button type="button" class="btn btn-xs btn-circle btn-ghost"
                                    on:click=move |_| {
                                        let new_val = (actual.get() - 1).max(0);
                                        actual.set(new_val);
                                        if let Some(cb) = cb_dec { cb.run((id_dec.clone(), new_val)); }
                                    }
                                >"\u{2212}"</button>
                                <span class="text-sm font-mono">
                                    {move || actual.get()} " / " {target_qty} " " {unit_str.clone()}
                                </span>
                                <button type="button" class="btn btn-xs btn-circle btn-ghost"
                                    on:click=move |_| {
                                        let new_val = actual.get() + 1;
                                        actual.set(new_val);
                                        if let Some(cb) = cb_inc { cb.run((id_inc.clone(), new_val)); }
                                    }
                                >"+"</button>
                            </div>
                            <progress class="progress progress-primary w-20 h-1"
                                value=move || actual.get().to_string()
                                max=target_qty.to_string()
                            />
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}

                // Move to dropdown
                {if let Some(on_move_cb) = on_move.filter(|_| !move_targets.is_empty()) {
                    let move_open = RwSignal::new(false);
                    view! {
                        <div class="relative">
                            <button type="button" class="btn btn-ghost btn-xs btn-square" title="Przenie\u{015B} do..."
                                on:click=move |_| move_open.update(|v| *v = !*v)
                            >"\u{2197}"</button>
                            <div
                                class="absolute right-0 top-full mt-1 bg-base-200 border border-base-300 rounded-box min-w-44 max-h-60 overflow-y-auto z-50 p-2 shadow-lg"
                                style:display=move || if move_open.get() { "block" } else { "none" }
                            >
                                {move_targets.iter().map(|(tid, tname)| {
                                    let tid = tid.clone();
                                    let tname = tname.clone();
                                    let iid = id_move.clone();
                                    view! {
                                        <button type="button"
                                            class="flex items-center gap-2 px-2 py-1.5 text-sm rounded cursor-pointer hover:bg-base-300 w-full text-left"
                                            on:click=move |_| {
                                                move_open.set(false);
                                                on_move_cb.run((iid.clone(), tid.clone()));
                                            }
                                        >{tname.clone()}</button>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }}

                <button type="button" class="btn btn-ghost btn-xs btn-square opacity-60 hover:opacity-100"
                    on:click=move |_| on_delete.run(id_delete.clone())
                >"\u{2715}"</button>
            </div>

            // Row 2: Tags
            {if has_tags {
                view! {
                    <div class="pl-14 pb-1">
                        <TagList
                            all_tags=all_tags.clone()
                            selected_tag_ids=item_tag_ids.clone()
                            on_toggle=on_tag_toggle
                        />
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }}

            // Inline date editor (when a badge is clicked)
            {move || {
                let dt = editing_date.get();
                if let (Some(dt), Some(on_save)) = (dt, on_date_save) {
                    let id_for_save = id_for_editor.clone();
                    let dt_for_save = dt.clone();
                    let (border, init_date, init_time, has_time) = match dt.as_str() {
                        "start" => ("border-info", item_start_date.clone(), item_start_time.clone(), true),
                        "hard_deadline" => ("border-error", item_hard_deadline.clone(), None, false),
                        _ => ("border-warning", item_deadline.clone(), item_deadline_time.clone(), true),
                    };
                    view! {
                        <div class="pl-14 pb-2">
                            <DateEditor
                                border_color=border
                                initial_date=init_date
                                initial_time=init_time
                                has_time=has_time
                                on_change=Callback::new(move |(date, time): (String, Option<String>)| {
                                    on_save.run((id_for_save.clone(), dt_for_save.clone(), date, time));
                                })
                            />
                            <button type="button" class="btn btn-xs btn-ghost mt-1 opacity-50"
                                on:click=move |_| editing_date.set(None)
                            >"Zamknij"</button>
                        </div>
                    }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}

            // Description (expandable)
            {move || {
                if expanded.get() {
                    let id_blur = id_for_desc.clone();
                    view! {
                        <div class="pl-14 pb-2">
                            <textarea
                                class="textarea textarea-bordered w-full text-sm resize-none"
                                rows="3"
                                placeholder="Dodaj opis..."
                                prop:value=move || description_text.get()
                                on:input=move |ev| description_text.set(event_target_value(&ev))
                                on:blur=move |_| {
                                    if let Some(cb) = on_description_save {
                                        cb.run((id_blur.clone(), description_text.get()));
                                    }
                                }
                            />
                        </div>
                    }.into_any()
                } else {
                    let desc = description_text.get();
                    if desc.is_empty() {
                        view! {}.into_any()
                    } else {
                        view! {
                            <p class="pl-14 pb-1 text-sm text-base-content/60">{desc}</p>
                        }.into_any()
                    }
                }
            }}
        </div>
    }
}
