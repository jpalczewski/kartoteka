use leptos::prelude::*;

/// Inline stepper: [−] actual / target unit [+] with optional progress bar.
/// Fires `on_change` with the new actual value on each click.
#[component]
pub fn QuantityStepper(
    actual: i32,
    target: Option<i32>,
    unit: Option<String>,
    on_change: Callback<i32>,
) -> impl IntoView {
    let local_actual = RwSignal::new(actual);
    let unit_str = unit.unwrap_or_else(|| "szt.".to_string());
    let target_str = target.map(|t| format!(" / {t}"));

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
                <span class="text-sm font-mono">
                    {move || local_actual.get()}
                    {target_str.clone()}
                    {format!(" {unit_str}")}
                </span>
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
            {target.map(|t| view! {
                <progress
                    class="progress progress-primary w-20 h-1"
                    value=move || local_actual.get().to_string()
                    max=t.to_string()
                />
            })}
        </div>
    }
}
