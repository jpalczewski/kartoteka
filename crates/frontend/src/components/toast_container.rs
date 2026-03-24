use leptos::prelude::*;

use crate::app::{ToastContext, ToastKind};

#[component]
pub fn ToastContainer() -> impl IntoView {
    let ctx = use_context::<ToastContext>().expect("ToastContext missing");

    view! {
        <div class="toast toast-end z-50">
            {move || ctx.toasts.get().into_iter().map(|toast| {
                let id = toast.id;
                let alert_class = match toast.kind {
                    ToastKind::Success => "alert alert-success",
                    ToastKind::Error => "alert alert-error",
                };
                view! {
                    <div class=alert_class>
                        <span>{toast.message}</span>
                        <button
                            type="button"
                            class="btn btn-ghost btn-xs"
                            on:click=move |_| ctx.dismiss(id)
                        >
                            "✕"
                        </button>
                    </div>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}
