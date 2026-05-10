use leptos::prelude::*;
use leptos_fluent::I18n;

use crate::app::{ToastContext, ToastKind};
use crate::components::common::confirm_modal::{ConfirmModal, ConfirmVariant};
use crate::components::common::loading::LoadingSpinner;
use crate::server_fns::locations::{
    delete_location_sf, get_countries, get_locations_sf, parse_location_sf, update_location_sf,
};
use kartoteka_shared::{Country, Location};

// ── data grouping ─────────────────────────────────────────────────────────────

#[derive(Clone)]
struct CountryGroup {
    country: Country,
    cities: Vec<CityGroup>,
}

#[derive(Clone)]
struct CityGroup {
    city: Location,
    addresses: Vec<Location>,
}

fn group_locations(countries: &[Country], locs: &[Location]) -> Vec<CountryGroup> {
    let cities: Vec<&Location> = locs.iter().filter(|l| l.location_type == "city").collect();
    let addresses: Vec<&Location> = locs
        .iter()
        .filter(|l| l.location_type == "address")
        .collect();

    let mut groups: Vec<CountryGroup> = countries
        .iter()
        .filter(|c| cities.iter().any(|l| l.country_id == c.id))
        .map(|c| {
            let city_groups = cities
                .iter()
                .filter(|l| l.country_id == c.id)
                .map(|city| CityGroup {
                    city: (*city).clone(),
                    addresses: addresses
                        .iter()
                        .filter(|a| a.parent_id.as_deref() == Some(city.id.as_str()))
                        .map(|a| (*a).clone())
                        .collect(),
                })
                .collect();
            CountryGroup {
                country: c.clone(),
                cities: city_groups,
            }
        })
        .collect();

    groups.sort_by(|a, b| a.country.iso_code.cmp(&b.country.iso_code));
    groups
}

// ── page ──────────────────────────────────────────────────────────────────────

#[component]
pub fn LocationsPage() -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let i18n = expect_context::<I18n>();
    let (refresh, set_refresh) = signal(0u32);

    let countries_res = Resource::new(|| (), |_| get_countries());
    let locs_res = Resource::new(move || refresh.get(), |_| get_locations_sf());

    let (parse_input, set_parse_input) = signal(String::new());
    let (parsing, set_parsing) = signal(false);

    let on_parse = Callback::new(move |_: ()| {
        let input = parse_input.get_untracked();
        if input.trim().is_empty() || parsing.get_untracked() {
            return;
        }
        set_parsing.set(true);
        leptos::task::spawn_local(async move {
            match parse_location_sf(input).await {
                Ok(_) => {
                    set_parse_input.set(String::new());
                    set_refresh.update(|n| *n += 1);
                }
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
            set_parsing.set(false);
        });
    });

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            <h2 class="text-2xl font-bold mb-4">"Lokalizacje"</h2>

            // Parser input
            <div class="flex gap-2 mb-6">
                <input
                    type="text"
                    class="input input-bordered flex-1"
                    placeholder=i18n.tr("locations-parse-placeholder")
                    prop:value=move || parse_input.get()
                    prop:disabled=move || parsing.get()
                    on:input=move |ev| set_parse_input.set(event_target_value(&ev))
                    on:keydown=move |ev| {
                        if ev.key() == "Enter" {
                            on_parse.run(());
                        }
                    }
                />
                <button
                    class="btn btn-primary"
                    prop:disabled=move || parsing.get()
                    on:click=move |_| on_parse.run(())
                >
                    {i18n.tr("locations-parse-add")}
                </button>
            </div>

            // Location tree grouped by country
            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || {
                    let countries = countries_res.get();
                    let locs = locs_res.get();
                    match (countries, locs) {
                        (Some(Ok(countries)), Some(Ok(locs))) => {
                            let groups = group_locations(&countries, &locs);
                            if groups.is_empty() {
                                return view! {
                                    <div class="text-center text-base-content/50 py-8">
                                        {i18n.tr("locations-empty")}
                                    </div>
                                }
                                .into_any();
                            }
                            view! {
                                <div class="flex flex-col gap-4">
                                    {groups
                                        .into_iter()
                                        .map(|group| {
                                            let country_name = i18n
                                                .tr(&format!("country-{}", group.country.iso_code));
                                            view! {
                                                <CountrySection
                                                    group=group
                                                    country_name=country_name
                                                    set_refresh=set_refresh
                                                />
                                            }
                                        })
                                        .collect_view()}
                                </div>
                            }
                            .into_any()
                        }
                        (Some(Err(e)), _) | (_, Some(Err(e))) => {
                            view! {
                                <p class="text-error">
                                    {format!("{}: {}", i18n.tr("locations-load-error"), e)}
                                </p>
                            }
                            .into_any()
                        }
                        _ => view! { <LoadingSpinner/> }.into_any(),
                    }
                }}
            </Suspense>
        </div>
    }
}

// ── country section ───────────────────────────────────────────────────────────

#[component]
fn CountrySection(
    group: CountryGroup,
    country_name: String,
    set_refresh: WriteSignal<u32>,
) -> impl IntoView {
    view! {
        <div>
            <h3 class="text-lg font-semibold text-base-content/70 mb-2 flex items-center gap-2">
                <span class="badge badge-outline">{group.country.iso_code.clone()}</span>
                {country_name}
            </h3>
            <div class="flex flex-col gap-1 ml-2">
                {group
                    .cities
                    .into_iter()
                    .map(|cg| {
                        view! { <CityRow city_group=cg set_refresh=set_refresh /> }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

// ── city row ──────────────────────────────────────────────────────────────────

#[component]
fn CityRow(city_group: CityGroup, set_refresh: WriteSignal<u32>) -> impl IntoView {
    let i18n = expect_context::<I18n>();
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let city = city_group.city.clone();
    let city_id = city.id.clone();

    let (editing, set_editing) = signal(false);
    let (edit_name, set_edit_name) = signal(city.name.clone());
    let show_delete = RwSignal::new(false);

    let on_save = {
        let id = city_id.clone();
        Callback::new(move |_: ()| {
            let name = edit_name.get_untracked();
            if name.trim().is_empty() {
                return;
            }
            set_editing.set(false);
            let id = id.clone();
            leptos::task::spawn_local(async move {
                match update_location_sf(id, Some(name), None, false, None, None, false).await {
                    Ok(_) => set_refresh.update(|n| *n += 1),
                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                }
            });
        })
    };

    let on_delete = {
        let id = city_id.clone();
        Callback::new(move |_: ()| {
            show_delete.set(false);
            let id = id.clone();
            leptos::task::spawn_local(async move {
                match delete_location_sf(id).await {
                    Ok(_) => set_refresh.update(|n| *n += 1),
                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                }
            });
        })
    };

    let display_name = city.name.clone();
    let region = city.region.clone();

    view! {
        <div class="card bg-base-200 p-2">
            {move || {
                if editing.get() {
                    view! {
                        <div class="flex gap-2 items-center">
                            <input
                                type="text"
                                class="input input-bordered input-sm flex-1"
                                prop:value=move || edit_name.get()
                                on:input=move |ev| set_edit_name.set(event_target_value(&ev))
                                on:keydown=move |ev| {
                                    if ev.key() == "Enter" {
                                        on_save.run(());
                                    }
                                }
                            />
                            <button
                                class="btn btn-sm btn-primary"
                                on:click=move |_| on_save.run(())
                            >
                                {i18n.tr("locations-save")}
                            </button>
                            <button
                                class="btn btn-sm btn-ghost"
                                on:click=move |_| set_editing.set(false)
                            >
                                {i18n.tr("locations-cancel")}
                            </button>
                        </div>
                    }
                    .into_any()
                } else {
                    view! {
                        <div class="flex items-center gap-2">
                            <span class="font-medium">{display_name.clone()}</span>
                            {region
                                .as_ref()
                                .map(|r| {
                                    view! {
                                        <span class="text-sm text-base-content/50">{r.clone()}</span>
                                    }
                                })}
                            <div class="ml-auto flex gap-1">
                                <button
                                    class="btn btn-xs btn-ghost"
                                    on:click=move |_| set_editing.set(true)
                                >
                                    "✎"
                                </button>
                                <button
                                    class="btn btn-xs btn-ghost text-error"
                                    on:click=move |_| show_delete.set(true)
                                >
                                    "✕"
                                </button>
                            </div>
                        </div>
                    }
                    .into_any()
                }
            }}

            // Addresses under city
            {if !city_group.addresses.is_empty() {
                view! {
                    <div class="flex flex-col gap-1 mt-1 ml-4">
                        {city_group
                            .addresses
                            .into_iter()
                            .map(|addr| {
                                view! { <AddressRow addr=addr set_refresh=set_refresh /> }
                            })
                            .collect_view()}
                    </div>
                }
                .into_any()
            } else {
                view! {}.into_any()
            }}

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
    }
}

// ── address row ───────────────────────────────────────────────────────────────

#[component]
fn AddressRow(addr: Location, set_refresh: WriteSignal<u32>) -> impl IntoView {
    let i18n = expect_context::<I18n>();
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let addr_id = addr.id.clone();

    let (editing, set_editing) = signal(false);
    let (edit_name, set_edit_name) = signal(addr.name.clone());
    let show_delete = RwSignal::new(false);

    let on_save = {
        let id = addr_id.clone();
        Callback::new(move |_: ()| {
            let name = edit_name.get_untracked();
            if name.trim().is_empty() {
                return;
            }
            set_editing.set(false);
            let id = id.clone();
            leptos::task::spawn_local(async move {
                match update_location_sf(id, Some(name), None, false, None, None, false).await {
                    Ok(_) => set_refresh.update(|n| *n += 1),
                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                }
            });
        })
    };

    let on_delete = {
        let id = addr_id.clone();
        Callback::new(move |_: ()| {
            show_delete.set(false);
            let id = id.clone();
            leptos::task::spawn_local(async move {
                match delete_location_sf(id).await {
                    Ok(_) => set_refresh.update(|n| *n += 1),
                    Err(e) => toast.push(e.to_string(), ToastKind::Error),
                }
            });
        })
    };

    let display_name = addr.name.clone();

    view! {
        <div class="flex items-center gap-2 py-1 px-2 rounded bg-base-100">
            {move || {
                if editing.get() {
                    view! {
                        <input
                            type="text"
                            class="input input-bordered input-xs flex-1"
                            prop:value=move || edit_name.get()
                            on:input=move |ev| set_edit_name.set(event_target_value(&ev))
                            on:keydown=move |ev| {
                                if ev.key() == "Enter" {
                                    on_save.run(());
                                }
                            }
                        />
                        <button
                            class="btn btn-xs btn-primary"
                            on:click=move |_| on_save.run(())
                        >
                            {i18n.tr("locations-save")}
                        </button>
                        <button
                            class="btn btn-xs btn-ghost"
                            on:click=move |_| set_editing.set(false)
                        >
                            {i18n.tr("locations-cancel")}
                        </button>
                    }
                    .into_any()
                } else {
                    view! {
                        <span class="text-sm flex-1">{display_name.clone()}</span>
                        <button
                            class="btn btn-xs btn-ghost"
                            on:click=move |_| set_editing.set(true)
                        >
                            "✎"
                        </button>
                        <button
                            class="btn btn-xs btn-ghost text-error"
                            on:click=move |_| show_delete.set(true)
                        >
                            "✕"
                        </button>
                    }
                    .into_any()
                }
            }}

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
    }
}
