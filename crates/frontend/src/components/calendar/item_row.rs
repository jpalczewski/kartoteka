use leptos::prelude::*;
use leptos_router::components::A;

#[component]
pub fn CalendarItemRow(
    item_id: String,
    list_id: String,
    title: String,
    completed: bool,
    on_toggle: Callback<()>,
    on_delete: Callback<()>,
    /// compact=true: xs sizing, no background padding (week view)
    /// compact=false: sm sizing, padded card (day view)
    #[prop(default = false)]
    compact: bool,
) -> impl IntoView {
    let href = format!("/lists/{list_id}/items/{item_id}");

    let (row_class, checkbox_class, title_class, title_class_done) = if compact {
        (
            "flex items-start gap-1 group",
            "checkbox checkbox-xs checkbox-primary mt-0.5 shrink-0",
            "flex-1 text-xs leading-tight hover:text-primary",
            "flex-1 text-xs text-base-content/50 line-through leading-tight",
        )
    } else {
        (
            "flex items-center gap-3 p-2 bg-base-200 rounded-lg group",
            "checkbox checkbox-sm checkbox-primary",
            "flex-1 text-sm text-base-content hover:text-primary",
            "flex-1 text-sm text-base-content/50 line-through",
        )
    };

    view! {
        <div class=row_class>
            <input
                type="checkbox"
                class=checkbox_class
                checked=completed
                on:change=move |_| on_toggle.run(())
            />
            <A
                href=href
                attr:class=move || if completed { title_class_done } else { title_class }
            >
                {title}
            </A>
            <button
                class="btn btn-ghost btn-xs opacity-0 group-hover:opacity-100 text-error"
                on:click=move |_| on_delete.run(())
            >
                "×"
            </button>
        </div>
    }
}
