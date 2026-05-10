use leptos::prelude::*;
use leptos_fluent::I18n;
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::app::{ToastContext, ToastKind};
use crate::components::common::confirm_modal::{ConfirmModal, ConfirmVariant};
use crate::components::common::loading::LoadingSpinner;
use crate::server_fns::locations::{
    delete_location_sf, get_city_addresses_sf, get_countries, get_location_detail_sf,
    update_location_sf,
};

use super::InlineAlias;

#[component]
pub fn CityDetailPage() -> impl IntoView {
    let params = use_params_map();
    let city_id = Memo::new(move |_| params.read().get("id").unwrap_or_default());
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let i18n = expect_context::<I18n>();
    let (refresh, set_refresh) = signal(0u32);

    let city_res = Resource::new(
        move || (city_id.get(), refresh.get()),
        |(id, _)| get_location_detail_sf(id),
    );
    let addresses_res = Resource::new(
        move || (city_id.get(), refresh.get()),
        |(id, _)| get_city_addresses_sf(id),
    );
    let countries_res = Resource::new(|| (), |_| get_countries());

    let (editing_name, set_editing_name) = signal(false);
    let (edit_name, set_edit_name) = signal(String::new());
    let show_delete = RwSignal::new(false);

    let on_save_name = Callback::new(move |_: ()| {
        let name = edit_name.get_untracked();
        if name.trim().is_empty() {
            return;
        }
        let id = city_id.get_untracked();
        set_editing_name.set(false);
        leptos::task::spawn_local(async move {
            match update_location_sf(id, Some(name), None, false, None, None, false).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    let on_delete = Callback::new(move |_: ()| {
        show_delete.set(false);
        let id = city_id.get_untracked();
        let toast2 = toast.clone();
        leptos::task::spawn_local(async move {
            match delete_location_sf(id).await {
                Ok(_) => {
                    let navigate = leptos_router::hooks::use_navigate();
                    navigate("/locations", Default::default());
                }
                Err(e) => toast2.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <div class="mb-4">
                <A href="/locations" attr:class="text-sm text-base-content/50 hover:text-base-content">
                    {i18n.tr("locations-title")}
                </A>
            </div>

            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || city_res.get().map(|result| {
                    match result {
                        Err(e) => view! { <p class="text-error">{e.to_string()}</p> }.into_any(),
                        Ok(None) => view! { <p class="text-base-content/50">"Nie znaleziono miasta."</p> }.into_any(),
                        Ok(Some(city)) => {
                            let country_name = countries_res.get()
                                .and_then(|r| r.ok())
                                .and_then(|cs| cs.into_iter().find(|c| c.id == city.country_id))
                                .map(|c| i18n.tr(&format!("country-{}", c.iso_code)))
                                .unwrap_or_default();

                            let city_name = StoredValue::new(city.name.clone());
                            let city_id_str = city.id.clone();
                            let city_alias = city.alias.clone();
                            let city_region = city.region.clone();

                            view! {
                                <div>
                                    // Header
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
                                                            {i18n.tr("locations-save")}
                                                        </button>
                                                        <button class="btn btn-sm btn-ghost" on:click=move |_| set_editing_name.set(false)>
                                                            {i18n.tr("locations-cancel")}
                                                        </button>
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <div class="flex items-center gap-2">
                                                        <h2 class="text-2xl font-bold">{city_name.get_value()}</h2>
                                                        <button
                                                            class="btn btn-xs btn-ghost"
                                                            on:click=move |_| {
                                                                set_edit_name.set(city_name.get_value());
                                                                set_editing_name.set(true);
                                                            }
                                                        >
                                                            "✎"
                                                        </button>
                                                    </div>
                                                }.into_any()
                                            }}
                                            <div class="flex items-center gap-2 mt-1 text-sm text-base-content/60">
                                                {city_region.as_ref().map(|r| view! { <span>{r.clone()}</span> })}
                                                <span>{country_name}</span>
                                            </div>
                                            <div class="mt-2">
                                                <InlineAlias id=city_id_str.clone() initial=city_alias.clone() set_refresh />
                                            </div>
                                        </div>
                                        <button
                                            class="btn btn-sm btn-ghost text-error"
                                            on:click=move |_| show_delete.set(true)
                                        >
                                            {i18n.tr("locations-delete-title")}
                                        </button>
                                    </div>

                                    // Addresses list
                                    <h3 class="font-semibold mb-3">"Adresy"</h3>
                                    <Suspense fallback=|| view! { <LoadingSpinner/> }>
                                        {move || addresses_res.get().map(|result| match result {
                                            Err(e) => view! { <p class="text-error">{e.to_string()}</p> }.into_any(),
                                            Ok(addrs) if addrs.is_empty() => view! {
                                                <p class="text-base-content/50 text-sm">"Brak adresow."</p>
                                            }.into_any(),
                                            Ok(addrs) => view! {
                                                <div class="flex flex-col gap-2">
                                                    {addrs.into_iter().map(|addr| {
                                                        let city_id_link = city_id_str.clone();
                                                        let addr_id = addr.id.clone();
                                                        let display = addr.alias.clone().unwrap_or_else(|| addr.name.clone());
                                                        let sub = if addr.alias.is_some() { Some(addr.name.clone()) } else { None };
                                                        view! {
                                                            <A
                                                                href=format!("/locations/{}/{}", city_id_link, addr_id)
                                                                attr:class="flex items-center gap-2 p-2 rounded bg-base-200 hover:bg-base-300"
                                                            >
                                                                <span class="text-sm font-medium">{display}</span>
                                                                {sub.map(|s| view! {
                                                                    <span class="text-xs text-base-content/50">{s}</span>
                                                                })}
                                                            </A>
                                                        }
                                                    }).collect_view()}
                                                </div>
                                            }.into_any(),
                                        })}
                                    </Suspense>

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
