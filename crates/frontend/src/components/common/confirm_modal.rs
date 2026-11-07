use leptos::prelude::*;
use leptos_fluent::move_tr;

#[derive(Clone, Copy, PartialEq)]
pub enum ConfirmVariant {
    Danger,
    Warning,
}

#[component]
pub fn ConfirmModal(
    open: Signal<bool>,
    title: String,
    message: String,
    confirm_label: String,
    variant: ConfirmVariant,
    on_confirm: Callback<()>,
    on_close: Callback<()>,
    /// Hide the confirm button (e.g. for informational / blocking dialogs).
    #[prop(default = false)]
    hide_confirm: bool,
) -> impl IntoView {
    let btn_class = match variant {
        ConfirmVariant::Danger => "btn btn-error",
        ConfirmVariant::Warning => "btn btn-warning",
    };

    view! {
        <Show when=move || open.get()>
            <dialog class="modal" open=true data-testid="confirm-modal">
                <div class="modal-box">
                    <h3 class="font-bold text-lg" data-testid="confirm-modal-title">{title.clone()}</h3>
                    <p class="py-4" data-testid="confirm-modal-message">{message.clone()}</p>
                    <div class="modal-action">
                        <button
                            type="button"
                            class="btn btn-ghost"
                            data-testid="confirm-modal-cancel"
                            on:click=move |_| on_close.run(())
                        >
                            {move_tr!("common-cancel")}
                        </button>
                        {(!hide_confirm).then(|| view! {
                            <button
                                type="button"
                                class=btn_class
                                data-testid="confirm-modal-confirm"
                                on:click=move |_| on_confirm.run(())
                            >
                                {confirm_label.clone()}
                            </button>
                        })}
                    </div>
                </div>
                <form method="dialog" class="modal-backdrop">
                    <button type="button" on:click=move |_| on_close.run(())>"close"</button>
                </form>
            </dialog>
        </Show>
    }
}
