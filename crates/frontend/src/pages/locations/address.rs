use leptos::prelude::*;
use leptos_fluent::{I18n, move_tr};
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::app::{ToastContext, ToastKind};
use crate::components::common::confirm_modal::{ConfirmModal, ConfirmVariant};
use crate::components::common::loading::LoadingSpinner;
use crate::server_fns::locations::{
    delete_location_sf, get_location_detail_sf, update_location_sf,
};

use super::InlineAlias;

#[component]
pub fn AddressDetailPage() -> impl IntoView {
    let params = use_params_map();
    let city_id = Memo::new(move |_| params.read().get("city_id").unwrap_or_default());
    let addr_id = Memo::new(move |_| params.read().get("id").unwrap_or_default());
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let i18n = expect_context::<I18n>();
    let (refresh, set_refresh) = signal(0u32);

    let addr_res = Resource::new(
        move || (addr_id.get(), refresh.get()),
        |(id, _)| get_location_detail_sf(id),
    );
    let city_res = Resource::new(move || city_id.get(), get_location_detail_sf);

    let (editing_name, set_editing_name) = signal(false);
    let (edit_name, set_edit_name) = signal(String::new());
    let show_delete = RwSignal::new(false);

    let on_save_name = Callback::new(move |_: ()| {
        let name = edit_name.get_untracked();
        if name.trim().is_empty() {
            return;
        }
        let id = addr_id.get_untracked();
        set_editing_name.set(false);
        leptos::task::spawn_local(async move {
            match update_location_sf(id, Some(name), None, false, None, None, false).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let navigate = leptos_router::hooks::use_navigate();
    let on_delete = Callback::new(move |_: ()| {
        show_delete.set(false);
        let id = addr_id.get_untracked();
        let cid = city_id.get_untracked();
        let toast2 = toast.clone();
        let nav = navigate.clone();
        leptos::task::spawn_local(async move {
            match delete_location_sf(id).await {
                Ok(_) => nav(&format!("/locations/{}", cid), Default::default()),
                Err(e) => toast2.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            // Breadcrumb
            <div class="flex items-center gap-2 mb-4 text-sm text-base-content/50">
                <A href="/locations" attr:class="hover:text-base-content">
                    {move_tr!("locations-title")}
                </A>
                <span>"/"</span>
                <Suspense>
                    {move || city_res.get().map(|r| {
                        let cid = city_id.get();
                        match r.ok().flatten() {
                            Some(city) => view! {
                                <A href=format!("/locations/{}", cid) attr:class="hover:text-base-content">
                                    {city.alias.unwrap_or(city.name)}
                                </A>
                            }.into_any(),
                            None => view! { <span>{cid}</span> }.into_any(),
                        }
                    })}
                </Suspense>
            </div>

            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || addr_res.get().map(|result| {
                    match result {
                        Err(e) => view! { <p class="text-error">{e.to_string()}</p> }.into_any(),
                        Ok(None) => view! { <p class="text-base-content/50">"Nie znaleziono adresu."</p> }.into_any(),
                        Ok(Some(addr)) => {
                            let addr_name = StoredValue::new(addr.name.clone());
                            let addr_id_str = addr.id.clone();
                            let addr_alias = addr.alias.clone();

                            view! {
                                <div>
                                    <div class="flex items-start justify-between mb-6">
                                        <div>
                                            {move || if editing_name.get() {
                                                view! {
                                                    <div class="flex gap-2 items-center">
                                                        <input
                                                            type="text"
                                                            class="input input-bordered input-sm"
                                                            prop:value=move || edit_name.get()
                                                            on:input=move |ev| set_edit_name.set(event_target_value(&ev))
                                                            on:keydown=move |ev| {
                                                                match ev.key().as_str() {
                                                                    "Enter" => on_save_name.run(()),
                                                                    "Escape" => set_editing_name.set(false),
                                                                    _ => {}
                                                                }
                                                            }
                                                        />
                                                        <button class="btn btn-sm btn-primary" on:click=move |_| on_save_name.run(())>
                                                            {move_tr!("locations-save")}
                                                        </button>
                                                        <button class="btn btn-sm btn-ghost" on:click=move |_| set_editing_name.set(false)>
                                                            {move_tr!("locations-cancel")}
                                                        </button>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <div class="flex items-center gap-2">
                                                        <h2 class="text-2xl font-bold">{addr_name.get_value()}</h2>
                                                        <button
                                                            class="btn btn-xs btn-ghost"
                                                            on:click=move |_| {
                                                                set_edit_name.set(addr_name.get_value());
                                                                set_editing_name.set(true);
                                                            }
                                                        >
                                                            "✎"
                                                        </button>
                                                    </div>
                                                }.into_any()
                                            }}
                                            <div class="mt-2">
                                                <InlineAlias id=addr_id_str.clone() initial=addr_alias.clone() set_refresh />
                                            </div>
                                        </div>
                                        <button
                                            class="btn btn-sm btn-ghost text-error"
                                            on:click=move |_| show_delete.set(true)
                                        >
                                            {i18n.tr("locations-delete-title")}
                                        </button>
                                    </div>

                                    <ConfirmModal
                                        open=Signal::from(show_delete)
                                        title=i18n.tr("locations-delete-title")
                                        message=i18n.tr("locations-delete-confirm")
                                        confirm_label=i18n.tr("common-delete")
                                        variant=ConfirmVariant::Danger
                                        on_confirm=on_delete
                                        on_close=Callback::new(move |_: ()| show_delete.set(false))
                                    />
                                </div>
                            }.into_any()
                        }
                    }
                })}
            </Suspense>
        </div>
    }
}
