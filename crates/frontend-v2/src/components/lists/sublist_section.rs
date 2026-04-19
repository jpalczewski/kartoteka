use kartoteka_shared::types::{Item, List};
use leptos::prelude::*;

use crate::app::{ToastContext, ToastKind};
use crate::components::items::item_row::ItemRow;
use crate::components::lists::add_input::AddInput;
use crate::server_fns::items::{create_item, delete_item, get_list_data, toggle_item};

#[component]
pub fn SublistSection(sublist: List, on_any_change: Callback<()>) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let list_id = sublist.id.clone();
    let list_name = sublist.name.clone();

    let (refresh, set_refresh) = signal(0u32);

    let data_res = Resource::new(
        {
            let lid = list_id.clone();
            move || (lid.clone(), refresh.get())
        },
        |(id, _)| get_list_data(id),
    );

    let lid_add = list_id.clone();
    let on_add = Callback::new(move |title: String| {
        let lid = lid_add.clone();
        let notify = on_any_change;
        leptos::task::spawn_local(async move {
            match create_item(lid, title).await {
                Ok(_) => {
                    set_refresh.update(|n| *n += 1);
                    notify.run(());
                }
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_toggle = Callback::new(move |item_id: String| {
        let notify = on_any_change;
        leptos::task::spawn_local(async move {
            match toggle_item(item_id).await {
                Ok(_) => {
                    set_refresh.update(|n| *n += 1);
                    notify.run(());
                }
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_delete = Callback::new(move |item_id: String| {
        leptos::task::spawn_local(async move {
            match delete_item(item_id).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    view! {
        <div class="collapse collapse-arrow bg-base-200 mb-2">
            <input type="checkbox" checked=true />
            <div class="collapse-title font-semibold flex items-center gap-2">
                <span>{list_name}</span>
                <Suspense>
                    {move || data_res.get().and_then(|r| r.ok()).map(|data| {
                        let done = data.items.iter().filter(|i| i.completed).count();
                        let total = data.items.len();
                        view! {
                            <span class="text-sm text-base-content/60 ml-auto mr-4">
                                {done} "/" {total}
                            </span>
                        }
                    })}
                </Suspense>
            </div>
            <div class="collapse-content">
                <Suspense fallback=|| view! { <p class="text-sm text-base-content/50">"Ładowanie..."</p> }>
                    {move || data_res.get().map(|result| match result {
                        Err(e) => view! {
                            <p class="text-error text-sm">{e.to_string()}</p>
                        }.into_any(),
                        Ok(data) => {
                            let items: Vec<Item> = {
                                let mut v = data.items.clone();
                                v.sort_by(|a, b| a.completed.cmp(&b.completed).then(a.position.cmp(&b.position)));
                                v
                            };
                            view! {
                                <div class="flex flex-col gap-1">
                                    {items.into_iter().map(|item| view! {
                                        <ItemRow item=item on_toggle=on_toggle on_delete=on_delete />
                                    }).collect::<Vec<_>>()}
                                    <div class="mt-2">
                                        <AddInput
                                            placeholder=Signal::derive(|| "Nowy element...".to_string())
                                            button_label=Signal::derive(|| "Dodaj".to_string())
                                            on_submit=on_add
                                        />
                                    </div>
                                </div>
                            }.into_any()
                        }
                    })}
                </Suspense>
            </div>
        </div>
    }
}
