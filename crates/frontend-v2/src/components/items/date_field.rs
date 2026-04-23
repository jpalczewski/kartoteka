use leptos::prelude::*;

#[component]
pub fn DateFieldInput(
    label: &'static str,
    value: RwSignal<String>,
    #[prop(optional)] data_testid: Option<&'static str>,
    #[prop(default = false)] show_clear: bool,
    #[prop(default = false)] large: bool,
) -> impl IntoView {
    let (gap, span_w, span_size, input_size) = if large {
        ("flex items-center gap-3", "w-32", "text-sm", "input-sm")
    } else {
        ("flex items-center gap-2", "w-28", "text-xs", "input-xs")
    };

    view! {
        <label class=gap>
            <span class=format!("text-base-content/50 {span_w} {span_size}")>{label}</span>
            <input
                type="date"
                class=format!("input input-bordered {input_size} flex-1")
                data-testid=data_testid.unwrap_or("")
                prop:value=move || value.get()
                on:input=move |ev| value.set(event_target_value(&ev))
            />
            {show_clear.then(move || view! {
                <button
                    type="button"
                    class="btn btn-ghost btn-xs text-base-content/30"
                    title="Wyczyść"
                    on:click=move |_| value.set(String::new())
                >"✕"</button>
            })}
        </label>
    }
}
