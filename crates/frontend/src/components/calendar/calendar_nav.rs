use leptos::prelude::*;

use super::ViewMode;
use crate::components::common::date_utils::{
    format_date_short, get_today_string, parse_date, polish_month_name, week_range,
};

#[component]
pub fn CalendarNav(
    current_date: RwSignal<String>,
    view_mode: RwSignal<ViewMode>,
    on_prev: Callback<()>,
    on_next: Callback<()>,
) -> impl IntoView {
    let title = move || {
        let date = current_date.get();
        match view_mode.get() {
            ViewMode::Month => {
                if let Some((y, m, _)) = parse_date(&date) {
                    format!("{} {}", polish_month_name(m), y)
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
                    on:click=move |_| current_date.set(get_today_string())
                >
                    "Dziś"
                </button>
                <div class="join">
                    <button
                        class=move || if view_mode.get() == ViewMode::Month {
                            "join-item btn btn-sm btn-active"
                        } else {
                            "join-item btn btn-sm"
                        }
                        on:click=move |_| view_mode.set(ViewMode::Month)
                    >
                        "Miesiąc"
                    </button>
                    <button
                        class=move || if view_mode.get() == ViewMode::Week {
                            "join-item btn btn-sm btn-active"
                        } else {
                            "join-item btn btn-sm"
                        }
                        on:click=move |_| view_mode.set(ViewMode::Week)
                    >
                        "Tydzień"
                    </button>
                </div>
            </div>
        </div>
    }
}
