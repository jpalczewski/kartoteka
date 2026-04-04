use kartoteka_shared::{Item, Tag};
use leptos::prelude::*;

use crate::components::common::date_utils::{
    format_date_short, get_today_string, is_overdue, item_date_badges, relative_date,
};
use crate::components::common::inline_confirm_button::InlineConfirmButton;
use crate::components::items::date_badge_chips::DateBadgeChips;
use crate::components::items::inline_date_editor_section::InlineDateEditorSection;
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
    let item_for_editor = item.clone();
    let item_href = format!("/lists/{}/items/{}", item.list_id, item.id);
    let item_title = item.title.clone();
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
                <a
                    href=item_href
                    class=format!("{title_class} hover:text-primary transition-colors no-underline")
                >
                    {item_title}
                </a>

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
                        <DateBadgeChips badges=secondary_badges editing_date=editing_date />
                    </div>
                }.into_any()
            } else {
                view! {}.into_any()
            }}

            // Inline date editor
            {move || {
                let dt = editing_date.get();
                if let (Some(dt), Some(on_save)) = (dt, on_date_save) {
                    view! {
                        <InlineDateEditorSection
                            date_type=dt
                            item=item_for_editor.clone()
                            item_id=id_for_editor.clone()
                            on_save=on_save
                            on_close=Callback::new(move |()| editing_date.set(None))
                            wrapper_class="pl-10 pb-2".to_string()
                        />
                    }.into_any()
                } else {
                    view! {}.into_any()
                }
            }}
        </div>
    }
}
