use crate::app::{ToastContext, ToastKind};
use crate::components::lists::add_input::AddInput;
use crate::components::lists::list_card::list_type_icon;
use crate::server_fns::items::{create_item, toggle_item};
use crate::server_fns::lists::get_list_preview_items;
use kartoteka_shared::PreviewItem;
use kartoteka_shared::types::List;
use leptos::prelude::*;

#[component]
pub fn ListPreview(
    list: List,
    #[prop(optional)] on_delete: Option<Callback<String>>,
    #[prop(optional)] on_archive: Option<Callback<String>>,
) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let list_id = list.id.clone();
    let list_name = list.name.clone();
    let icon = list_type_icon(&list.list_type);
    let href = format!("/lists/{}", list.id);

    let (refresh, set_refresh) = signal(0u32);

    // Resource fires only when expanded (refresh > 0 means user has expanded at least once).
    // Re-expanding after collapse re-fetches to show fresh data.
    let items_resource = Resource::new(
        {
            let lid = list_id.clone();
            move || (lid.clone(), refresh.get())
        },
        |(id, rev)| async move {
            if rev == 0 {
                // Not yet expanded — return empty without hitting the server.
                Ok(Vec::<PreviewItem>::new())
            } else {
                get_list_preview_items(id).await
            }
        },
    );

    let on_toggle = Callback::new(move |item_id: String| {
        leptos::task::spawn_local(async move {
            match toggle_item(item_id).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let lid_add = list_id.clone();
    let on_add = Callback::new(move |title: String| {
        let lid = lid_add.clone();
        leptos::task::spawn_local(async move {
            match create_item(lid, title, None, None, None, None, None, None).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let list_id_del = list_id.clone();
    let list_id_archive = list_id.clone();

    view! {
        <div class="collapse collapse-arrow bg-base-200 border border-base-300">
            // Checking the hidden checkbox triggers expand; we use the CSS collapse pattern.
            // on:change fires when the checkbox toggles — we use it to trigger the first fetch.
            <input
                type="checkbox"
                on:change=move |ev| {
                    use web_sys::wasm_bindgen::JsCast;
                    let checked = ev.target()
                        .and_then(|t| t.dyn_into::<web_sys::HtmlInputElement>().ok())
                        .map(|el| el.checked())
                        .unwrap_or(false);
                    if checked {
                        set_refresh.update(|n| *n += 1);
                    }
                }
            />
            <div class="collapse-title font-semibold flex items-center gap-2 pr-10">
                <span>{icon}</span>
                <span data-testid="list-preview-title">{list_name}</span>
                <a
                    href=href
                    class="btn btn-ghost btn-xs ml-1"
                    title="Otwórz pełny widok"
                    on:click=|ev: leptos::ev::MouseEvent| ev.stop_propagation()
                >
                    "↗"
                </a>
                <div class="ml-auto flex gap-1 mr-2" on:click=|ev: leptos::ev::MouseEvent| ev.stop_propagation()>
                    {on_archive.map(|cb| {
                        let lid = list_id_archive.clone();
                        view! {
                            <button
                                type="button"
                                class="btn btn-ghost btn-xs btn-circle"
                                title="Archiwizuj"
                                on:click=move |_| cb.run(lid.clone())
                            >{"🗄"}</button>
                        }
                    })}
                    {on_delete.map(|cb| {
                        let lid = list_id_del.clone();
                        view! {
                            <button
                                type="button"
                                class="btn btn-ghost btn-xs btn-circle text-error"
                                on:click=move |_| cb.run(lid.clone())
                            >{"✕"}</button>
                        }
                    })}
                </div>
            </div>
            <div class="collapse-content">
                <Suspense fallback=|| view! { <p class="text-sm text-base-content/50 py-2">"Ładowanie..."</p> }>
                    {move || items_resource.get().map(|result| match result {
                        Err(e) => view! {
                            <p class="text-error text-sm">{e.to_string()}</p>
                        }.into_any(),
                        Ok(items) => {
                            if items.is_empty() && refresh.get() == 0 {
                                // Not yet expanded — render nothing (collapse is closed).
                                return view! {}.into_any();
                            }
                            view! {
                                <div class="flex flex-col gap-1 pt-1">
                                    {items.into_iter().map(|item| {
                                        let iid = item.id.clone();
                                        view! {
                                            <label class="flex items-center gap-2 cursor-pointer py-1">
                                                <input
                                                    type="checkbox"
                                                    class="checkbox checkbox-sm"
                                                    prop:checked=item.completed
                                                    on:change=move |_| on_toggle.run(iid.clone())
                                                />
                                                <span class:line-through=item.completed class:opacity-50=item.completed>
                                                    {item.title.clone()}
                                                </span>
                                                {item.quantity.map(|q| {
                                                    let display = item.unit.as_deref()
                                                        .map(|u| format!("{q} {u}"))
                                                        .unwrap_or_else(|| format!("{q}×"));
                                                    view! {
                                                        <span class="text-xs text-base-content/50 ml-1">{display}</span>
                                                    }
                                                })}
                                            </label>
                                        }
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
