use leptos::prelude::*;

/// Confirmation dialog before deleting a list.
#[component]
pub fn ConfirmDeleteModal(
    list_name: String,
    on_confirm: Callback<()>,
    on_cancel: Callback<()>,
) -> impl IntoView {
    let deleting = RwSignal::new(false);
    let name = list_name.clone();

    view! {
        <dialog class="modal" open=true>
            <div class="modal-box">
                <h3 class="font-bold text-lg">"Usuń listę"</h3>
                <p class="py-4">
                    "Czy na pewno chcesz usunąć listę \""
                    {name.clone()}
                    "\"?"
                </p>
                <div class="modal-action ">
                    <button
                        type="button"
                        class="btn btn-ghost "
                        on:click=move |_| on_cancel.run(())
                        disabled=move || deleting.get()
                    >
                        "Anuluj"
                    </button>
                    <button
                        type="button"
                        class="btn btn-error "
                        on:click=move |_| {
                            deleting.set(true);
                            on_confirm.run(());
                        }
                        disabled=move || deleting.get()
                    >
                        "Usuń"
                    </button>
                </div>
            </div>
            <form method="dialog" class="modal-backdrop ">
                <button type="button" on:click=move |_| on_cancel.run(())>
                    "close"
                </button>
            </form>
        </dialog>
    }
}
