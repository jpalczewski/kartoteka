use kartoteka_shared::{Item, Tag};
use leptos::prelude::*;

use crate::components::common::date_utils::{
    format_date_short, get_today_string, is_overdue, item_date_badges, relative_date,
};
use crate::components::common::inline_confirm_button::InlineConfirmButton;
use crate::components::items::date_editor::DateEditor;
use crate::components::tags::tag_list::TagList;

#[component]
pub fn DateItemRow(
    item: Item,
    on_toggle: Callback<String>,
    on_delete: Callback<String>,
    #[prop(default = vec![])] all_tags: Vec<Tag>,
    #[prop(default = vec![])] item_tag_ids: Vec<String>,
    #[prop(optional)] on_tag_toggle: Option<Callback<String>>,
    #[prop(optional)] date_type: Option<String>,
    /// (item_id, date_type, date_value, time_value)
    #[prop(optional)]
    on_date_save: Option<Callback<(String, String, String, Option<String>)>>,
) -> impl IntoView {
    let id_toggle = item.id.clone();
    let id_delete = item.id.clone();
    let id_for_editor = item.id.clone();
    let completed = item.completed;
    let today = get_today_string();
    let overdue = is_overdue(&item, &today);

    let row_class = if completed {
        "flex items-center gap-3 py-2 opacity-50"
    } else if overdue {
        "flex items-center gap-3 py-2 text-error"
    } else {
        "flex items-center gap-3 py-2"
    };

    let title_class = if completed {
        "flex-1 min-w-0 line-through text-base-content/50"
    } else {
        "flex-1 min-w-0"
    };

    let primary_dt = date_type.as_deref().unwrap_or("deadline");
    let (display_date, display_time) = match primary_dt {
        "start" => (item.start_date.clone(), item.start_time.clone()),
        "hard_deadline" => (item.hard_deadline.clone(), None),
        _ => (item.deadline.clone(), item.deadline_time.clone()),
    };

    let date_display = display_date.as_ref().map(|d| format_date_short(d));
    let relative = display_date.as_ref().map(|d| relative_date(d, &today));
    let time_display = display_time;

    let date_color = if completed {
        "text-right text-sm text-base-content/40 shrink-0"
    } else if overdue {
        "text-right text-sm text-error shrink-0"
    } else {
        "text-right text-sm text-base-content/60 shrink-0"
    };

    let primary_icon = match primary_dt {
        "start" => "\u{1F4C5}",
        "hard_deadline" => "\u{1F6A8}",
        _ => "\u{23F0}",
    };

    let secondary_badges = item_date_badges(&item, Some(primary_dt));
    let has_secondary =
        !secondary_badges.is_empty() || !item_tag_ids.is_empty() || on_tag_toggle.is_some();

    // Date editing state
    let editing_date = RwSignal::new(Option::<String>::None);

    // Store initial values for editor
    let item_start_date = item.start_date.clone();
    let item_start_time = item.start_time.clone();
    let item_deadline = item.deadline.clone();
    let item_deadline_time = item.deadline_time.clone();
    let item_hard_deadline = item.hard_deadline.clone();

    // Primary date is clickable if on_date_save is available
    let primary_dt_str = primary_dt.to_string();

    view! {
        <div class="border-b border-base-300">
            // Row 1: checkbox + title + primary date + delete
            <div class=row_class>
                <input
                    type="checkbox"
                    class="checkbox checkbox-secondary"
                    checked=completed
                    on:change=move |_| on_toggle.run(id_toggle.clone())
                />
                <span class=title_class>{item.title}</span>

                // Primary date (clickable for editing)
                {if on_date_save.is_some() {
                    let pdt = primary_dt_str.clone();
                    view! {
                        <button type="button" class=format!("{date_color} cursor-pointer hover:opacity-80")
                            on:click=move |_| {
                                let current = editing_date.get();
                                if current.as_deref() == Some(pdt.as_str()) {
                                    editing_date.set(None);
                                } else {
                                    editing_date.set(Some(pdt.clone()));
                                }
                            }
                        >
                            {date_display.as_ref().map(|d| view! {
                                <div class="flex items-center gap-1 justify-end">
                                    <span class="opacity-60">{primary_icon}</span>
                                    <span class="font-medium">{d.clone()}</span>
                                </div>
                            })}
                            {relative.as_ref().map(|r| view! { <div class="text-xs">{r.clone()}</div> })}
                            {time_display.as_ref().map(|t| view! { <div class="text-xs">{t.clone()}</div> })}
                        </button>
                    }.into_any()
                } else {
                    view! {
                        <div class=date_color>
                            {date_display.map(|d| view! {
                                <div class="flex items-center gap-1 justify-end">
                                    <span class="opacity-60">{primary_icon}</span>
                                    <span class="font-medium">{d}</span>
                                </div>
                            })}
                            {relative.map(|r| view! { <div class="text-xs">{r}</div> })}
                            {time_display.map(|t| view! { <div class="text-xs">{t}</div> })}
                        </div>
                    }.into_any()
                }}

                <InlineConfirmButton on_confirm=Callback::new(move |()| on_delete.run(id_delete.clone())) />
            </div>

            // Row 2: tags + secondary date badges (clickable)
            {if has_secondary {
                view! {
                    <div class="flex items-center gap-1 pl-10 pb-1 flex-wrap">
                        <TagList
                            all_tags=all_tags.clone()
                            selected_tag_ids=item_tag_ids.clone()
                            on_toggle=on_tag_toggle
                        />
                        {secondary_badges.into_iter().map(|b| {
                            if on_date_save.is_some() {
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
                                }.into_any()
                            } else {
                                view! { <span class=b.css>{b.label}</span> }.into_any()
                            }
                        }).collect::<Vec<_>>()}
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }}

            // Inline date editor
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
                        <div class="pl-10 pb-2">
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
        </div>
    }
}
