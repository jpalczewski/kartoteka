use kartoteka_shared::{Item, Tag};
use leptos::prelude::*;

use super::tag_badge::TagBadge;
use super::tag_selector::TagSelector;

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
) -> impl IntoView {
    let id = item.id.clone();
    let id_toggle = id.clone();
    let id_delete = id.clone();
    let id_move = id.clone();
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

    view! {
        <div class="border-b border-base-300 py-1">
            // Row 1: checkbox, expand, title, quantity, actions
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
                    {move || if expanded.get() { "▲" } else { "▼" }}
                </button>
                <span class=title_class>{item.title}</span>

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
                                >"−"</button>
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
                {if !move_targets.is_empty() && on_move.is_some() {
                    let move_open = RwSignal::new(false);
                    let on_move_cb = on_move.unwrap();
                    view! {
                        <div class="relative">
                            <button type="button" class="btn btn-ghost btn-xs btn-square" title="Przenieś do..."
                                on:click=move |_| move_open.update(|v| *v = !*v)
                            >"↗"</button>
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
                >"✕"</button>
            </div>

            // Row 2: Tags (below title, indented)
            {if has_tags {
                let item_tags: Vec<Tag> = all_tags.iter()
                    .filter(|t| item_tag_ids.contains(&t.id))
                    .cloned()
                    .collect();
                view! {
                    <div class="flex flex-wrap items-center gap-1 pl-14 pb-1">
                        {item_tags.into_iter().map(|t| {
                            match on_tag_toggle {
                                Some(cb) => view! { <TagBadge tag=t on_remove=cb/> }.into_any(),
                                None => view! { <TagBadge tag=t/> }.into_any(),
                            }
                        }).collect::<Vec<_>>()}
                        {if let Some(toggle_cb) = on_tag_toggle {
                            view! {
                                <TagSelector
                                    all_tags=all_tags.clone()
                                    selected_tag_ids=item_tag_ids.clone()
                                    on_toggle=toggle_cb
                                />
                            }.into_any()
                        } else {
                            view! {}.into_any()
                        }}
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }}

            // Description (expandable)
            {move || {
                if expanded.get() {
                    let id_blur = id.clone();
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
