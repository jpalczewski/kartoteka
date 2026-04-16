use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::app::{ToastContext, ToastKind};
use crate::components::comments::CommentSection;
use crate::components::common::loading::LoadingSpinner;
use crate::components::items::item_row::ItemRow;
use crate::components::lists::{
    add_input::AddInput,
    list_card::{ListCard, list_type_icon},
};
use crate::server_fns::items::{create_item, delete_item, get_list_data, toggle_item};

#[component]
pub fn ListPage() -> impl IntoView {
    let params = use_params_map();
    let list_id = move || params.read().get("id").unwrap_or_default();

    let toast = use_context::<ToastContext>().expect("ToastContext missing");

    let (refresh, set_refresh) = signal(0u32);

    let data_res = Resource::new(
        move || (list_id(), refresh.get()),
        |(id, _)| get_list_data(id),
    );

    // ── Mutation callbacks ─────────────────────────────────────────

    let on_add_item = Callback::new(move |title: String| {
        let lid = list_id();
        leptos::task::spawn_local(async move {
            match create_item(lid, title).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_toggle_item = Callback::new(move |item_id: String| {
        leptos::task::spawn_local(async move {
            match toggle_item(item_id).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_delete_item = Callback::new(move |item_id: String| {
        leptos::task::spawn_local(async move {
            match delete_item(item_id).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || data_res.get().map(|result| match result {
                    Err(e) => view! {
                        <p class="text-error">"Błąd: " {e.to_string()}</p>
                    }.into_any(),
                    Ok(data) => {
                        let icon = list_type_icon(&data.list.list_type);
                        let list_name = data.list.name.clone();
                        let list_description = data.list.description.clone();
                        let items = data.items.clone();
                        let sublists = data.sublists.clone();

                        view! {
                            <div>
                                // Header
                                <div class="mb-4 flex items-center gap-2">
                                    <span class="text-2xl">{icon}</span>
                                    <h2 class="text-2xl font-bold">{list_name}</h2>
                                </div>

                                {list_description.map(|desc| view! {
                                    <p class="text-base-content/60 mb-4">{desc}</p>
                                })}

                                // Add item input
                                <div class="mb-4">
                                    <AddInput
                                        placeholder=Signal::derive(|| "Nowy element...".to_string())
                                        button_label=Signal::derive(|| "Dodaj".to_string())
                                        on_submit=on_add_item
                                    />
                                </div>

                                // Sublists section
                                {if sublists.is_empty() {
                                    view! {}.into_any()
                                } else {
                                    view! {
                                        <div class="mb-4">
                                            <h3 class="text-sm font-semibold text-base-content/60 mb-2 uppercase tracking-wide">
                                                "Podlisty"
                                            </h3>
                                            <div class="flex flex-col gap-2">
                                                {sublists.into_iter().map(|sublist| view! {
                                                    <ListCard list=sublist />
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        </div>
                                    }.into_any()
                                }}

                                // Items list
                                {if items.is_empty() {
                                    view! {
                                        <div class="text-center text-base-content/50 py-8">
                                            "Brak elementów. Dodaj pierwszy powyżej."
                                        </div>
                                    }.into_any()
                                } else {
                                    let completed_count = items.iter().filter(|i| i.completed).count();
                                    let total = items.len();

                                    view! {
                                        <div>
                                            <div class="flex items-center justify-between mb-2">
                                                <span class="text-sm text-base-content/60">
                                                    {completed_count} "/" {total} " ukończone"
                                                </span>
                                            </div>
                                            <div class="flex flex-col gap-2">
                                                {items.into_iter().map(|item| view! {
                                                    <ItemRow
                                                        item=item
                                                        on_toggle=on_toggle_item
                                                        on_delete=on_delete_item
                                                    />
                                                }).collect::<Vec<_>>()}
                                            </div>
                                        </div>
                                    }.into_any()
                                }}

                                // Comments
                                <CommentSection
                                    entity_type="list"
                                    entity_id=Signal::derive(list_id)
                                />
                            </div>
                        }.into_any()
                    }
                })}
            </Suspense>
        </div>
    }
}
