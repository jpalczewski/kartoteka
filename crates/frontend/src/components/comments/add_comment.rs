use leptos::prelude::*;

use crate::app::{ToastContext, ToastKind};
use crate::server_fns::comments::add_comment;

#[component]
pub fn AddComment(
    entity_type: Signal<String>,
    entity_id: Signal<String>,
    on_added: Callback<()>,
) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let content = RwSignal::new(String::new());

    let on_submit = move |ev: leptos::ev::MouseEvent| {
        ev.prevent_default();
        let text = content.get();
        if text.trim().is_empty() {
            return;
        }
        let et = entity_type.get();
        let eid = entity_id.get();
        leptos::task::spawn_local(async move {
            match add_comment(et, eid, text).await {
                Ok(_) => {
                    content.set(String::new());
                    on_added.run(());
                }
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    view! {
        <div class="flex flex-col gap-2 mt-3">
            <textarea
                class="textarea textarea-bordered w-full h-20 text-sm"
                placeholder="Dodaj komentarz..."
                prop:value=content
                on:input=move |ev| content.set(event_target_value(&ev))
            />
            <button
                type="button"
                class="btn btn-primary btn-sm self-end"
                disabled=move || content.get().trim().is_empty()
                on:click=on_submit
            >
                "Wyślij"
            </button>
        </div>
    }
}
