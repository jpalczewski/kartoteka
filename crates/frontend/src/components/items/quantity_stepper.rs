use leptos::prelude::*;

/// Inline stepper: [−] actual / target unit [+] with optional progress bar.
/// Fires `on_change` with the new actual value on each click.
///
/// The count is updated optimistically on click and follows `actual` when the item changes
/// (a refetch, or an edit from another tab).
#[component]
pub fn QuantityStepper(
    #[prop(into)] actual: Signal<i32>,
    #[prop(into)] target: Signal<Option<i32>>,
    #[prop(into)] unit: Signal<Option<String>>,
    on_change: Callback<i32>,
) -> impl IntoView {
    let local_actual = RwSignal::new(actual.get_untracked());
    Effect::new(move |_| local_actual.set(actual.get()));

    let label = move || {
        let unit = unit.get().unwrap_or_else(|| "szt.".to_string());
        let target = target.get().map(|t| format!(" / {t}")).unwrap_or_default();
        format!("{}{target} {unit}", local_actual.get())
    };

    view! {
        <div class="flex flex-col items-center gap-0.5">
            <div class="flex items-center gap-1">
                <button
                    type="button"
                    class="btn btn-xs btn-circle btn-ghost"
                    on:click=move |_| {
                        let new_val = (local_actual.get() - 1).max(0);
                        local_actual.set(new_val);
                        on_change.run(new_val);
                    }
                >
                    "\u{2212}"
                </button>
                <span class="text-sm font-mono">{label}</span>
                <button
                    type="button"
                    class="btn btn-xs btn-circle btn-ghost"
                    on:click=move |_| {
                        let new_val = local_actual.get() + 1;
                        local_actual.set(new_val);
                        on_change.run(new_val);
                    }
                >
                    "+"
                </button>
            </div>
            {move || target.get().map(|t| view! {
                <progress
                    class="progress progress-primary w-20 h-1"
                    value=move || local_actual.get().to_string()
                    max=t.to_string()
                />
            })}
        </div>
    }
}
