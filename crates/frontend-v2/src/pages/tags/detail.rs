use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::app::{ToastContext, ToastKind};
use crate::components::common::loading::LoadingSpinner;
use crate::components::lists::list_card::ListCard;
use crate::components::tags::color_picker::TagColorPicker;
use crate::server_fns::tags::{get_tag_detail_data, update_tag_color};

#[component]
pub fn TagDetailPage() -> impl IntoView {
    let params = use_params_map();
    let tag_id = move || params.read().get("id").unwrap_or_default();
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let (refresh, set_refresh) = signal(0u32);

    let data_res = Resource::new(
        move || (tag_id(), refresh.get()),
        |(id, _)| get_tag_detail_data(id),
    );

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || data_res.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="text-error" data-testid="tag-error">"Błąd: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(data) => {
                        let color = data.tag.color.clone().unwrap_or_else(|| "#6366f1".to_string());
                        let tag_name = data.tag.name.clone();
                        let tag_type = data.tag.tag_type.clone();
                        let tag_icon = data.tag.icon.clone();
                        let linked_lists = data.linked_lists.clone();
                        let tid = data.tag.id.clone();

                        view! {
                            <div>
                                // Tag header
                                <div class="mb-6 flex items-center gap-3">
                                    {tag_icon.map(|i| view! { <span class="text-2xl">{i}</span> })}
                                    <div>
                                        <h2
                                            class="text-2xl font-bold"
                                            data-testid="tag-detail-name"
                                            style=format!("color: {color}")
                                        >
                                            {tag_name}
                                        </h2>
                                        <span class="text-sm text-base-content/50">{tag_type}</span>
                                    </div>
                                    <div class="ml-auto">
                                        <TagColorPicker
                                            value=Signal::derive(move || color.clone())
                                            size_class="w-8 h-8"
                                            on_change=Callback::new(move |new_color: String| {
                                                let id = tid.clone();
                                                leptos::task::spawn_local(async move {
                                                    match update_tag_color(id, new_color).await {
                                                        Ok(_) => set_refresh.update(|n| *n += 1),
                                                        Err(e) => toast.push(e.to_string(), ToastKind::Error),
                                                    }
                                                });
                                            })
                                        />
                                    </div>
                                </div>

                                // Linked lists
                                {if linked_lists.is_empty() {
                                    view! {
                                        <div class="text-center text-base-content/50 py-8" data-testid="tag-no-lists">
                                            "Żadna lista nie ma tego tagu."
                                        </div>
                                    }.into_any()
                                } else {
                                    view! {
                                        <div>
                                            <h3
                                                class="text-sm font-semibold text-base-content/60 mb-3 uppercase tracking-wide"
                                                data-testid="linked-lists-heading"
                                            >
                                                "Listy z tym tagiem (" {linked_lists.len()} ")"
                                            </h3>
                                            <div class="flex flex-col gap-2">
                                                {linked_lists.into_iter().map(|list| view! {
                                                    <ListCard list=list />
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        </div>
                                    }.into_any()
                                }}
                            </div>
                        }.into_any()
                    }
                })}
            </Suspense>
        </div>
    }
}
