use leptos::prelude::*;

use crate::api;

#[derive(Clone)]
enum CountState {
    Loading,
    Loaded(usize),
    Error,
}

#[component]
pub fn ConfirmDeleteModal(
    list_name: String,
    list_id: String,
    on_confirm: Callback<()>,
    on_cancel: Callback<()>,
    #[prop(optional)] item_count: Option<usize>,
) -> impl IntoView {
    let count_state = RwSignal::new(match item_count {
        Some(n) => CountState::Loaded(n),
        None => CountState::Loading,
    });
    let deleting = RwSignal::new(false);

    // Fetch item count on mount if not provided
    if item_count.is_none() {
        let lid = list_id.clone();
        leptos::task::spawn_local(async move {
            match api::fetch_items(&lid).await {
                Ok(items) => count_state.set(CountState::Loaded(items.len())),
                Err(_) => count_state.set(CountState::Error),
            }
        });
    }

    view! {
        <dialog class="modal" open=true>
            <div class="modal-box">
                <h3 class="font-bold text-lg">"Usuń listę"</h3>

                {move || match count_state.get() {
                    CountState::Loading => view! {
                        <p class="py-4">"Wczytywanie szczegółów…"</p>
                    }.into_any(),
                    CountState::Error => view! {
                        <p class="py-4">
                            "Czy na pewno chcesz usunąć listę "
                            <strong>{list_name.clone()}</strong>
                            "? Operacja jest nieodwracalna."
                        </p>
                    }.into_any(),
                    CountState::Loaded(n) => view! {
                        <p class="py-4">
                            "Czy na pewno chcesz usunąć listę "
                            <strong>{list_name.clone()}</strong>
                            "? Zawiera "
                            <strong>{n}</strong>
                            " elementów. Operacja jest nieodwracalna."
                        </p>
                    }.into_any(),
                }}

                <div class="modal-action">
                    <button
                        type="button"
                        class="btn btn-ghost"
                        on:click=move |_| on_cancel.run(())
                    >
                        "Anuluj"
                    </button>
                    <button
                        type="button"
                        class="btn btn-error"
                        disabled=move || deleting.get()
                        on:click=move |_| {
                            deleting.set(true);
                            on_confirm.run(());
                        }
                    >
                        "Usuń listę"
                    </button>
                </div>
            </div>
            <div
                class="modal-backdrop"
                on:click=move |_| on_cancel.run(())
            />
        </dialog>
    }
}
