use crate::app::{ToastContext, ToastKind};
use leptos::prelude::*;

#[component]
pub fn ToastContainer() -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");

    view! {
        <div class="toast toast-end toast-bottom z-50">
            <For
                each=move || toast.toasts.get()
                key=|t| t.id
                let:t
            >
                {
                    let id = t.id;
                    let class = match t.kind {
                        ToastKind::Success => "alert alert-success",
                        ToastKind::Error => "alert alert-error",
                    };
                    view! {
                        <div class=class>
                            <span>{t.message.clone()}</span>
                            <button
                                class="btn btn-ghost btn-xs"
                                on:click=move |_| toast.dismiss(id)
                                aria-label="Dismiss"
                            >
                                "×"
                            </button>
                        </div>
                    }
                }
            </For>
        </div>
    }
}
