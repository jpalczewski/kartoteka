pub mod detail;

use leptos::prelude::*;

use crate::app::{ToastContext, ToastKind};
use crate::components::common::loading::LoadingSpinner;
use crate::server_fns::tags::{create_tag, delete_tag, get_all_tags};

#[component]
pub fn TagsPage() -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let (refresh, set_refresh) = signal(0u32);

    let tags_res = Resource::new(move || refresh.get(), |_| get_all_tags());

    let (new_name, set_new_name) = signal(String::new());

    let on_create = Callback::new(move |_: ()| {
        let name = new_name.get();
        if name.trim().is_empty() {
            return;
        }
        leptos::task::spawn_local(async move {
            match create_tag(name, None, None, None).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
        set_new_name.set(String::new());
    });

    let on_delete = Callback::new(move |id: String| {
        leptos::task::spawn_local(async move {
            match delete_tag(id).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-4">"Tagi"</h2>

            // Create tag form
            <div class="flex gap-2 mb-6">
                <input
                    type="text"
                    class="input input-bordered flex-1"
                    placeholder="Nowy tag..."
                    data-testid="new-tag-input"
                    prop:value=move || new_name.get()
                    on:input=move |ev| set_new_name.set(event_target_value(&ev))
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            on_create.run(());
                        }
                    }
                />
                <button
                    type="button"
                    class="btn btn-primary"
                    data-testid="create-tag-btn"
                    on:click=move |_| on_create.run(())
                >
                    "Dodaj"
                </button>
            </div>

            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || tags_res.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="text-error">"Błąd: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(tags) => {
                        if tags.is_empty() {
                            return view! {
                                <div class="text-center text-base-content/50 py-8">
                                    "Brak tagów. Dodaj pierwszy powyżej."
                                </div>
                            }.into_any();
                        }
                        view! {
                            <div class="flex flex-col gap-2">
                                {tags.into_iter().map(|tag| {
                                    let tid = tag.id.clone();
                                    let color = tag.color.clone().unwrap_or_else(|| "#6366f1".to_string());
                                    let href = format!("/tags/{}", &tag.id);
                                    view! {
                                        <div class="flex items-center justify-between p-3 bg-base-200 rounded-lg" data-testid="tag-item">
                                            <a href=href class="flex items-center gap-2 flex-1" data-testid="tag-link">
                                                {tag.icon.as_deref().map(|i| view! { <span>{i.to_string()}</span> })}
                                                <span
                                                    class="badge badge-outline font-medium"
                                                    style=format!("border-color: {color}; color: {color}")
                                                >
                                                    {tag.name.clone()}
                                                </span>
                                                <span class="text-xs text-base-content/40">{tag.tag_type.clone()}</span>
                                            </a>
                                            <button
                                                type="button"
                                                class="btn btn-ghost btn-xs btn-circle text-error"
                                                data-testid="delete-tag-btn"
                                                on:click=move |_| on_delete.run(tid.clone())
                                            >
                                                {"✕"}
                                            </button>
                                        </div>
                                    }
                                }).collect::<Vec<_>>()}
                            </div>
                        }.into_any()
                    }
                })}
            </Suspense>
        </div>
    }
}
