use chrono::Datelike;
use leptos::prelude::*;
use leptos_fluent::move_tr;

use super::ViewMode;
use crate::components::common::date_utils::{
    format_date_short, parse_date, polish_month_name, week_range,
};

#[component]
pub fn CalendarNav(
    anchor_date: RwSignal<String>,
    view_mode: RwSignal<ViewMode>,
    on_prev: Callback<()>,
    on_next: Callback<()>,
    on_view_mode_change: Callback<ViewMode>,
    on_today: Callback<()>,
) -> impl IntoView {
    let title = move || {
        let date = anchor_date.get();
        match view_mode.get() {
            ViewMode::Month => {
                if let Some(d) = parse_date(&date) {
                    format!("{} {}", polish_month_name(d.month()), d.year())
                } else {
                    date
                }
            }
            ViewMode::Week => {
                let (mon, sun) = week_range(&date);
                format!("{} – {}", format_date_short(&mon), format_date_short(&sun))
            }
        }
    };

    view! {
        <div class="flex items-center justify-between mb-4">
            <div class="flex items-center gap-2">
                <button class="btn btn-sm btn-ghost" on:click=move |_| on_prev.run(())>
                    "‹"
                </button>
                <h2 class="text-lg font-bold min-w-48 text-center">{title}</h2>
                <button class="btn btn-sm btn-ghost" on:click=move |_| on_next.run(())>
                    "›"
                </button>
            </div>

            <div class="flex items-center gap-2">
                <button
                    class="btn btn-sm btn-outline"
                    on:click=move |_| on_today.run(())
                >
                    {move_tr!("nav-today")}
                </button>
                <div class="join">
                    <input
                        class="join-item btn btn-sm"
                        type="radio"
                        name="calendar-view-mode"
                        aria-label="Miesiąc"
                        prop:checked=move || view_mode.get() == ViewMode::Month
                        on:change=move |_| on_view_mode_change.run(ViewMode::Month)
                    />
                    <input
                        class="join-item btn btn-sm"
                        type="radio"
                        name="calendar-view-mode"
                        aria-label="Tydzień"
                        prop:checked=move || view_mode.get() == ViewMode::Week
                        on:change=move |_| on_view_mode_change.run(ViewMode::Week)
                    />
                </div>
            </div>
        </div>
    }
}
