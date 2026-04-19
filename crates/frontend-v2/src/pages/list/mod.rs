use leptos::prelude::*;
use leptos_router::hooks::use_params_map;

use crate::app::{ToastContext, ToastKind};
use crate::components::comments::CommentSection;
use crate::components::common::loading::LoadingSpinner;
use crate::components::items::item_row::ItemRow;
use crate::components::lists::{
    add_input::AddInput, list_card::list_type_icon, sublist_section::SublistSection,
};
use crate::server_fns::items::{create_item, delete_item, get_list_data, toggle_item};
use crate::server_fns::lists::create_list;

#[component]
pub fn ListPage() -> impl IntoView {
    let params = use_params_map();
    let list_id = move || params.read().get("id").unwrap_or_default();

    let toast = use_context::<ToastContext>().expect("ToastContext missing");

    let (refresh, set_refresh) = signal(0u32);
    let (show_completed, set_show_completed) = signal(true);

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

    let on_create_sublist = Callback::new(move |name: String| {
        use kartoteka_shared::types::CreateListRequest;
        let parent_id = list_id();
        leptos::task::spawn_local(async move {
            let req = CreateListRequest {
                name,
                list_type: None,
                icon: None,
                description: None,
                container_id: None,
                parent_list_id: Some(parent_id),
                features: vec![],
            };
            match create_list(req).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let (adding_sublist, set_adding_sublist) = signal(false);

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
                        let created_at_local = data.created_at_local.clone();
                        let all_items = data.items.clone();
                        let completed_count = all_items.iter().filter(|i| i.completed).count();
                        let total = all_items.len();
                        let sublists = data.sublists.clone();

                        view! {
                            <div>
                                // Header
                                <div class="mb-1 flex items-center gap-2">
                                    <span class="text-2xl">{icon}</span>
                                    <h2 class="text-2xl font-bold">{list_name}</h2>
                                </div>
                                <p class="text-xs text-base-content/40 mb-4" data-testid="list-created-at">
                                    "Utworzono: " {created_at_local}
                                </p>

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
                                {
                                    let notify = Callback::new(move |_| set_refresh.update(|n| *n += 1));
                                    view! {
                                        <div class="mb-4">
                                            {if !sublists.is_empty() {
                                                view! {
                                                    <h3 class="text-sm font-semibold text-base-content/60 mb-2 uppercase tracking-wide">
                                                        "Podlisty"
                                                    </h3>
                                                    <div class="flex flex-col gap-2 mb-2">
                                                        {sublists.into_iter().map(|sublist| view! {
                                                            <SublistSection sublist=sublist on_any_change=notify />
                                                        }).collect::<Vec<_>>()}
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! {}.into_any()
                                            }}
                                            // Add sublist button
                                            {move || if adding_sublist.get() {
                                                view! {
                                                    <AddInput
                                                        placeholder=Signal::derive(|| "Nazwa podlisty...".to_string())
                                                        button_label=Signal::derive(|| "Utwórz".to_string())
                                                        on_submit=Callback::new(move |name: String| {
                                                            set_adding_sublist.set(false);
                                                            on_create_sublist.run(name);
                                                        })
                                                    />
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <button
                                                        type="button"
                                                        class="btn btn-ghost btn-sm"
                                                        on:click=move |_| set_adding_sublist.set(true)
                                                    >
                                                        "+ Dodaj podlistę"
                                                    </button>
                                                }.into_any()
                                            }}
                                        </div>
                                    }.into_any()
                                }

                                // Items list
                                {move || {
                                    let visible: Vec<_> = all_items
                                        .iter()
                                        .filter(|i| show_completed.get() || !i.completed)
                                        .cloned()
                                        .collect();
                                    if visible.is_empty() {
                                        view! {
                                            <div class="text-center text-base-content/50 py-8">
                                                {if !show_completed.get() && completed_count > 0 {
                                                    "Wszystkie elementy ukończone — odznacz filtr aby je zobaczyć."
                                                } else {
                                                    "Brak elementów. Dodaj pierwszy powyżej."
                                                }}
                                            </div>
                                        }.into_any()
                                    } else {
                                        view! {
                                            <div>
                                                <div class="flex items-center justify-between mb-2">
                                                    <span class="text-sm text-base-content/60" data-testid="completion-count">
                                                        {completed_count} "/" {total} " ukończone"
                                                    </span>
                                                    <label class="flex items-center gap-2 cursor-pointer select-none">
                                                        <span class="text-xs text-base-content/50">"Ukryj ukończone"</span>
                                                        <input
                                                            type="checkbox"
                                                            class="toggle toggle-xs"
                                                            prop:checked=move || !show_completed.get()
                                                            on:change=move |ev| set_show_completed.set(!event_target_checked(&ev))
                                                        />
                                                    </label>
                                                </div>
                                                <div class="flex flex-col gap-2">
                                                    {visible.into_iter().map(|item| view! {
                                                        <ItemRow
                                                            item=item
                                                            on_toggle=on_toggle_item
                                                            on_delete=on_delete_item
                                                        />
                                                    }).collect::<Vec<_>>()}
                                                </div>
                                            </div>
                                        }.into_any()
                                    }
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
