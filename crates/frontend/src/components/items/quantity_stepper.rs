use leptos::prelude::*;

/// Quantity stepper: -/+ buttons, actual/target display, progress bar.
/// Used by ItemRow and ItemDetailPage.
#[component]
pub fn QuantityStepper(
    target: i32,
    initial_actual: i32,
    unit: String,
    on_change: Callback<i32>,
) -> impl IntoView {
    let actual = RwSignal::new(initial_actual);

    view! {
        <div class="flex flex-col items-center gap-0.5">
            <div class="flex items-center gap-1">
                <button type="button" class="btn btn-xs btn-circle btn-ghost"
                    on:click=move |_| {
                        let new_val = (actual.get() - 1).max(0);
                        actual.set(new_val);
                        on_change.run(new_val);
                    }
                >"\u{2212}"</button>
                <span class="text-sm font-mono">
                    {move || actual.get()} " / " {target} " " {unit.clone()}
                </span>
                <button type="button" class="btn btn-xs btn-circle btn-ghost"
                    on:click=move |_| {
                        let new_val = actual.get() + 1;
                        actual.set(new_val);
                        on_change.run(new_val);
                    }
                >"+"</button>
            </div>
            <progress class="progress progress-primary w-20 h-1"
                value=move || actual.get().to_string()
                max=target.to_string()
            />
        </div>
    }
}
