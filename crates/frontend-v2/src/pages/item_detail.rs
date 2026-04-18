use leptos::prelude::*;
use leptos_router::{components::A, hooks::use_params_map};

use crate::app::{ToastContext, ToastKind};
use crate::components::comments::CommentSection;
use crate::components::common::loading::LoadingSpinner;
use crate::components::relations::RelatedEntities;
use crate::components::time_entries::ItemTimerWidget;
use crate::server_fns::items::{get_item, toggle_item, update_item};

#[component]
pub fn ItemDetailPage() -> impl IntoView {
    let params = use_params_map();
    let list_id = move || params.read().get("list_id").unwrap_or_default();
    let item_id = move || params.read().get("id").unwrap_or_default();

    let toast = use_context::<ToastContext>().expect("ToastContext missing");

    let (refresh, set_refresh) = signal(0u32);

    let item_res = Resource::new(move || (item_id(), refresh.get()), |(id, _)| get_item(id));

    // ── Local edit signals (populated once item loads) ────────────
    let title_input: RwSignal<String> = RwSignal::new(String::new());
    let description_input: RwSignal<String> = RwSignal::new(String::new());

    // ── Save callback ─────────────────────────────────────────────
    let on_save = move |_: leptos::ev::MouseEvent| {
        let id = item_id();
        let title = title_input.get();
        let description = description_input.get();
        leptos::task::spawn_local(async move {
            match update_item(id, title, description).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    // ── Toggle callback ────────────────────────────────────────────
    let on_toggle = move |_: leptos::ev::Event| {
        let id = item_id();
        leptos::task::spawn_local(async move {
            match toggle_item(id).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    };

    let back_href = move || format!("/lists/{}", list_id());

    Effect::new(move |_| {
        if let Some(Ok(item)) = item_res.get() {
            title_input.set(item.title.clone());
            description_input.set(item.description.clone().unwrap_or_default());
        }
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            // Back link
            <div class="mb-4">
                <A href=back_href attr:class="btn btn-ghost btn-sm gap-1">
                    {"← Back to list"}
                </A>
            </div>

            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || item_res.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="text-error">"Error: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(item) => {
                        let completed = item.completed;
                        let created_at = item.created_at.clone();
                        let updated_at = item.updated_at.clone();

                        view! {
                            <div class="flex flex-col gap-4">
                                // Completed toggle
                                <label class="flex items-center gap-3 cursor-pointer">
                                    <input
                                        type="checkbox"
                                        class="checkbox checkbox-primary"
                                        checked=completed
                                        on:change=on_toggle
                                    />
                                    <span class="text-base-content/70">
                                        {if completed { "Completed" } else { "Mark as done" }}
                                    </span>
                                </label>

                                // Title field
                                <div class="form-control">
                                    <label class="label">
                                        <span class="label-text font-semibold">"Title"</span>
                                    </label>
                                    <input
                                        type="text"
                                        class="input input-bordered w-full"
                                        prop:value=move || title_input.get()
                                        on:input=move |ev| title_input.set(event_target_value(&ev))
                                    />
                                </div>

                                // Description field
                                <div class="form-control">
                                    <label class="label">
                                        <span class="label-text font-semibold">"Description"</span>
                                    </label>
                                    <textarea
                                        class="textarea textarea-bordered w-full h-32"
                                        prop:value=move || description_input.get()
                                        on:input=move |ev| description_input.set(event_target_value(&ev))
                                    />
                                </div>

                                // Save button
                                <button
                                    type="button"
                                    class="btn btn-primary w-full"
                                    on:click=on_save
                                >
                                    "Save"
                                </button>

                                // Timestamps
                                <div class="text-xs text-base-content/40 mt-2 flex flex-col gap-1">
                                    <span>"Created: " {created_at}</span>
                                    <span>"Updated: " {updated_at}</span>
                                </div>

                                // Comments
                                <CommentSection
                                    entity_type="item"
                                    entity_id=Signal::derive(item_id)
                                />

                                // Relations
                                <RelatedEntities
                                    entity_id=Signal::derive(item_id)
                                />

                                // Time tracking
                                <ItemTimerWidget
                                    item_id=Signal::derive(item_id)
                                />
                            </div>
                        }.into_any()
                    }
                })}
            </Suspense>
        </div>
    }
}
