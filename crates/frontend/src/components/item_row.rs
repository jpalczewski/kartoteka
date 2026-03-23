use kartoteka_shared::Item;
use leptos::prelude::*;

#[component]
pub fn ItemRow(
    item: Item,
    on_toggle: Callback<String>,
    on_delete: Callback<String>,
) -> impl IntoView {
    let id = item.id.clone();
    let id_toggle = id.clone();
    let id_delete = id.clone();
    let class = if item.completed {
        "item-row completed"
    } else {
        "item-row"
    };

    view! {
        <div class=class>
            <input
                type="checkbox"
                checked=item.completed
                on:change=move |_| on_toggle.run(id_toggle.clone())
            />
            <span class="item-title" style="flex: 1;">{item.title}</span>
            <button
                class="btn"
                style="padding: 0.25rem 0.5rem; font-size: 0.8rem;"
                on:click=move |_| on_delete.run(id_delete.clone())
            >
                "X"
            </button>
        </div>
    }
}
