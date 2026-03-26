use kartoteka_shared::{Item, Tag};
use leptos::prelude::*;

use crate::components::common::date_utils::{
    format_date_short, get_today_string, is_overdue, relative_date,
};
use crate::components::tags::tag_list::TagList;

#[component]
pub fn DateItemRow(
    item: Item,
    on_toggle: Callback<String>,
    on_delete: Callback<String>,
    #[prop(default = vec![])] all_tags: Vec<Tag>,
    #[prop(default = vec![])] item_tag_ids: Vec<String>,
    #[prop(optional)] on_tag_toggle: Option<Callback<String>>,
) -> impl IntoView {
    let id_toggle = item.id.clone();
    let id_delete = item.id.clone();
    let completed = item.completed;
    let today = get_today_string();

    let overdue = is_overdue(&item, &today);

    let row_class = if completed {
        "flex items-center gap-3 py-3 opacity-50"
    } else if overdue {
        "flex items-center gap-3 py-3 text-error"
    } else {
        "flex items-center gap-3 py-3"
    };

    let title_class = if completed {
        "flex-1 line-through text-base-content/50"
    } else {
        "flex-1"
    };

    let date_display = item.due_date.as_ref().map(|d| format_date_short(d));
    let relative = item.due_date.as_ref().map(|d| relative_date(d, &today));
    let time_display = item.due_time.clone();

    let date_color = if completed {
        "text-right text-sm text-base-content/40"
    } else if overdue {
        "text-right text-sm text-error"
    } else {
        "text-right text-sm text-base-content/60"
    };

    view! {
        <div class="border-b border-base-300">
            <div class=row_class>
                <input
                    type="checkbox"
                    class="checkbox checkbox-secondary"
                    checked=completed
                    on:change=move |_| on_toggle.run(id_toggle.clone())
                />
                <span class=title_class>{item.title}</span>

                // Tags
                <TagList
                    all_tags=all_tags.clone()
                    selected_tag_ids=item_tag_ids.clone()
                    on_toggle=on_tag_toggle
                />

                <div class=date_color>
                    {date_display.map(|d| view! { <div class="font-medium">{d}</div> })}
                    {relative.map(|r| view! { <div class="text-xs">{r}</div> })}
                    {time_display.map(|t| view! { <div class="text-xs">{t}</div> })}
                </div>
                {
                    let confirming = RwSignal::new(false);
                    view! {
                        <button
                            type="button"
                            class=move || if confirming.get() { "btn btn-error btn-sm" } else { "btn btn-ghost btn-sm btn-square opacity-60 hover:opacity-100" }
                            on:click=move |_| {
                                if confirming.get() {
                                    on_delete.run(id_delete.clone());
                                    confirming.set(false);
                                } else {
                                    confirming.set(true);
                                    set_timeout(move || confirming.set(false), std::time::Duration::from_millis(2500));
                                }
                            }
                        >
                            {move || if confirming.get() { "Na pewno?" } else { "✕" }}
                        </button>
                    }
                }
            </div>
        </div>
    }
}
