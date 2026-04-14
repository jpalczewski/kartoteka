use kartoteka_shared::types::Item;
use leptos::prelude::*;

#[component]
pub fn ItemRow(item: Item, on_toggle: Callback<String>, on_delete: Callback<String>) -> impl IntoView {
    let item_id_toggle = item.id.clone();
    let item_id_delete = item.id.clone();
    let completed = item.completed;

    view! {
        <div class="flex items-center gap-3 p-3 bg-base-200 rounded-lg">
            <input
                type="checkbox"
                class="checkbox checkbox-primary"
                checked=completed
                on:change=move |_| on_toggle.run(item_id_toggle.clone())
            />
            <span
                class=move || if completed {
                    "flex-1 text-base-content/50 line-through"
                } else {
                    "flex-1 text-base-content"
                }
            >
                {item.title.clone()}
            </span>
            <button
                type="button"
                class="btn btn-ghost btn-xs btn-circle text-error"
                on:click=move |_| on_delete.run(item_id_delete.clone())
            >
                {"✕"}
            </button>
        </div>
    }
}
