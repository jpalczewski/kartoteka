use leptos::prelude::*;
use leptos_fluent::{I18n, move_tr};
use leptos_router::components::A;
use leptos_router::hooks::use_params_map;

use crate::app::ToastContext;
use crate::components::common::inline_edit::{InlineEdit, InlineEditSize};
use crate::components::common::loading::LoadingSpinner;
use crate::server_fns::locations::{
    get_city_addresses_sf, get_countries, get_country_regions_sf, get_location_detail_sf,
    update_location_sf,
};
use kartoteka_shared::UpdateLocationRequest;

use super::detail_header::LocationDetailHeader;
use super::refresh_or_toast;

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

    let regions_res = Resource::new(
        move || {
            let city = city_res.get()?.ok()??;
            let countries = countries_res.get()?.ok()?;
            countries
                .into_iter()
                .find(|c| c.id == city.country_id)
                .map(|c| c.iso_code)
        },
        |iso| async move {
            match iso {
                Some(code) => get_country_regions_sf(code).await,
                None => Ok(vec![]),
            }
        },
    );

    let (editing_region, set_editing_region) = signal(false);
    let (edit_region, set_edit_region) = signal(String::new());

    let on_save_region = Callback::new(move |region: String| {
        set_editing_region.set(false);
        if region.trim().is_empty() {
            // Clearing a region is not supported by `update_location_sf`: `region: None`
            // means "keep" and there is no clear flag (unlike alias/address), so an
            // empty save is a no-op rather than storing an empty-string region.
            return;
        }
        refresh_or_toast(
            update_location_sf(
                city_id.get_untracked(),
                UpdateLocationRequest::set_region(region),
            ),
            set_refresh,
            toast.clone(),
        );
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <div class="mb-4">
                <A href="/locations" attr:class="text-sm text-base-content/50 hover:text-base-content">
                    {move_tr!("locations-title")}
                </A>
            </div>

            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || city_res.get().map(|result| {
                    match result {
                        Err(e) => view! { <p class="text-error">{e.to_string()}</p> }.into_any(),
                        Ok(None) => view! { <p class="text-base-content/50">{move_tr!("locations-city-not-found")}</p> }.into_any(),
                        Ok(Some(city)) => {
                            let country_name = countries_res.get()
                                .and_then(|r| r.ok())
                                .and_then(|cs| cs.into_iter().find(|c| c.id == city.country_id))
                                .map(|c| i18n.tr(&format!("country-{}", c.iso_code)))
                                .unwrap_or_default();

                            let city_id_str = city.id.clone();
                            let city_region_sv = StoredValue::new(city.region.clone());
                            let country_name_sv = StoredValue::new(country_name);

                            view! {
                                <div>
                                    <LocationDetailHeader
                                        id=city.id
                                        name=city.name
                                        alias=city.alias
                                        after_delete_href="/locations".to_string()
                                        set_refresh=set_refresh
                                    >
                                        // Region / country line
                                        <div class="flex items-center gap-2 mt-1 text-sm text-base-content/60">
                                            {move || if editing_region.get() {
                                                let regions = regions_res.get()
                                                    .and_then(|r| r.ok())
                                                    .unwrap_or_default();
                                                let uses_select = !regions.is_empty();
                                                view! {
                                                    <div class="flex gap-2 items-center">
                                                        {if uses_select {
                                                            let current = edit_region.get_untracked();
                                                            view! {
                                                                <select
                                                                    class="select select-bordered select-xs"
                                                                    on:change=move |ev| set_edit_region.set(event_target_value(&ev))
                                                                >
                                                                    <option value="" disabled selected=current.is_empty()>"—"</option>
                                                                    {regions.into_iter().map(|name| {
                                                                        let selected = name == current;
                                                                        let name2 = name.clone();
                                                                        view! {
                                                                            <option value=name selected=selected>{name2}</option>
                                                                        }
                                                                    }).collect_view()}
                                                                </select>
                                                            }.into_any()
                                                        } else {
                                                            view! {
                                                                <InlineEdit
                                                                    value=city_region_sv.get_value().unwrap_or_default()
                                                                    on_save=on_save_region
                                                                    on_cancel=Callback::new(move |_: ()| set_editing_region.set(false))
                                                                    size=InlineEditSize::Xs
                                                                />
                                                            }.into_any()
                                                        }}
                                                        {uses_select.then(|| view! {
                                                            <button
                                                                class="btn btn-xs btn-primary"
                                                                on:click=move |_| on_save_region.run(edit_region.get_untracked())
                                                            >
                                                                {move_tr!("common-save")}
                                                            </button>
                                                            <button class="btn btn-xs btn-ghost" on:click=move |_| set_editing_region.set(false)>
                                                                {move_tr!("common-cancel")}
                                                            </button>
                                                        })}
                                                    </div>
                                                }.into_any()
                                            } else {
                                                view! {
                                                    <div class="flex items-center gap-2">
                                                        {city_region_sv.get_value().map(|r| view! {
                                                            <span>{r.clone()}</span>
                                                        })}
                                                        <button
                                                            class="btn btn-xs btn-ghost"
                                                            on:click=move |_| {
                                                                set_edit_region.set(city_region_sv.get_value().unwrap_or_default());
                                                                set_editing_region.set(true);
                                                            }
                                                        >
                                                            {if city_region_sv.get_value().is_some() { "✎" } else { "+region" }}
                                                        </button>
                                                        <span>{country_name_sv.get_value()}</span>
                                                    </div>
                                                }.into_any()
                                            }}
                                        </div>
                                    </LocationDetailHeader>

                                    // Addresses list
                                    <h3 class="font-semibold mb-3">{move_tr!("locations-addresses-title")}</h3>
                                    <Suspense fallback=|| view! { <LoadingSpinner/> }>
                                        {move || addresses_res.get().map(|result| match result {
                                            Err(e) => view! { <p class="text-error">{e.to_string()}</p> }.into_any(),
                                            Ok(addrs) if addrs.is_empty() => view! {
                                                <p class="text-base-content/50 text-sm">{move_tr!("locations-addresses-empty")}</p>
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
                                </div>
                            }.into_any()
                        }
                    }
                })}
            </Suspense>
        </div>
    }
}
