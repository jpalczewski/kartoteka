use leptos::prelude::*;

#[derive(Clone, Copy, PartialEq)]
pub enum ConfirmVariant {
    Danger,
    Warning,
}

#[component]
pub fn ConfirmModal(
    open: RwSignal<bool>,
    title: String,
    message: String,
    confirm_label: String,
    variant: ConfirmVariant,
    on_confirm: Callback<()>,
) -> impl IntoView {
    let btn_class = match variant {
        ConfirmVariant::Danger => "btn btn-error",
        ConfirmVariant::Warning => "btn btn-warning",
    };

    let close = move || open.set(false);

    view! {
        <Show when=move || open.get()>
            <dialog class="modal" open=true>
                <div class="modal-box">
                    <h3 class="font-bold text-lg">{title.clone()}</h3>
                    <p class="py-4">{message.clone()}</p>
                    <div class="modal-action">
                        <button
                            type="button"
                            class="btn btn-ghost"
                            on:click=move |_| close()
                        >
                            "Anuluj"
                        </button>
                        <button
                            type="button"
                            class=btn_class
                            on:click=move |_| {
                                close();
                                on_confirm.run(());
                            }
                        >
                            {confirm_label.clone()}
                        </button>
                    </div>
                </div>
                <form method="dialog" class="modal-backdrop">
                    <button type="button" on:click=move |_| close()>"close"</button>
                </form>
            </dialog>
        </Show>
    }
}
