use crate::app::{ToastContext, ToastKind};
use crate::components::ui::{IconButton, IconButtonVariant};
use leptos::prelude::*;
use leptos_fluent::move_tr;

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
                            <IconButton
                                icon="x"
                                label=move_tr!("common-dismiss")
                                variant=IconButtonVariant::Ghost
                                class="text-current"
                                on_click=Callback::new(move |_| toast.dismiss(id))
                            />
                        </div>
                    }
                }
            </For>
        </div>
    }
}
