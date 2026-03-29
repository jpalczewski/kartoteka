use leptos::prelude::*;
use leptos_fluent::{move_tr, tr};

use crate::api;
use crate::api::client::GlooClient;

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
        let client = use_context::<GlooClient>().expect("GlooClient not provided");
        leptos::task::spawn_local(async move {
            match api::fetch_items(&client, &lid).await {
                Ok(items) => count_state.set(CountState::Loaded(items.len())),
                Err(_) => count_state.set(CountState::Error),
            }
        });
    }

    view! {
        <dialog class="modal" open=true>
            <div class="modal-box">
                <h3 class="font-bold text-lg">{move_tr!("lists-confirm-delete-title")}</h3>

                {move || match count_state.get() {
                    CountState::Loading => view! {
                        <p class="py-4">{move_tr!("lists-confirm-delete-loading")}</p>
                    }.into_any(),
                    CountState::Error => view! {
                        <p class="py-4">
                            {tr!("lists-confirm-delete-message-unknown", { "name" => list_name.clone() })}
                        </p>
                    }.into_any(),
                    CountState::Loaded(n) => view! {
                        <p class="py-4">
                            {tr!("lists-confirm-delete-message", { "name" => list_name.clone(), "count" => n })}
                        </p>
                    }.into_any(),
                }}

                <div class="modal-action">
                    <button
                        type="button"
                        class="btn btn-ghost"
                        on:click=move |_| on_cancel.run(())
                    >
                        {move_tr!("common-cancel")}
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
                        {move_tr!("lists-confirm-delete-button")}
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
