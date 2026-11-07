pub mod address;
pub mod city;
mod detail_header;

use leptos::prelude::*;
use leptos_fluent::{I18n, move_tr};
use leptos_router::components::A;

use crate::app::{ToastContext, ToastKind};
use crate::components::common::confirm_modal::{ConfirmModal, ConfirmVariant};
use crate::components::common::inline_edit::{InlineEdit, InlineEditSize};
use crate::components::common::loading::LoadingSpinner;
use crate::server_fns::locations::{
    delete_location_sf, get_countries, get_locations_sf, parse_location_sf, update_location_sf,
};
use kartoteka_shared::{Country, Location, UpdateLocationRequest};

// ── shared actions ────────────────────────────────────────────────────────────

/// Runs a location server-fn call and refreshes the tree on success.
///
/// Failures become an error toast instead of an unhandled server-fn error, which is the
/// pattern every write in this module needs.
fn refresh_or_toast<F, T>(fut: F, set_refresh: WriteSignal<u32>, toast: ToastContext)
where
    F: std::future::Future<Output = Result<T, ServerFnError>> + 'static,
    T: 'static,
{
    leptos::task::spawn_local(async move {
        match fut.await {
            Ok(_) => set_refresh.update(|n| *n += 1),
            Err(e) => toast.push(e.to_string(), ToastKind::Error),
        }
    });
}

// ── data grouping ─────────────────────────────────────────────────────────────

#[derive(Clone)]
struct CountryGroup {
    country: Country,
    regions: Vec<RegionGroup>,
}

#[derive(Clone)]
struct RegionGroup {
    region: Option<String>,
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
            let city_groups: Vec<CityGroup> = cities
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

            // Group cities by region: named regions first (sorted), then None at end
            let mut region_map: std::collections::BTreeMap<String, Vec<CityGroup>> =
                std::collections::BTreeMap::new();
            let mut no_region: Vec<CityGroup> = vec![];
            for cg in city_groups {
                match cg.city.region.clone() {
                    Some(r) => region_map.entry(r).or_default().push(cg),
                    None => no_region.push(cg),
                }
            }

            let mut regions: Vec<RegionGroup> = region_map
                .into_iter()
                .map(|(r, cities)| RegionGroup {
                    region: Some(r),
                    cities,
                })
                .collect();
            if !no_region.is_empty() {
                regions.push(RegionGroup {
                    region: None,
                    cities: no_region,
                });
            }

            CountryGroup {
                country: c.clone(),
                regions,
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
            <h2 class="text-2xl font-bold mb-4">{move_tr!("locations-title")}</h2>

            // Parser input
            <div class="flex gap-2 mb-6">
                <input
                    type="text"
                    class="input input-bordered flex-1"
                    placeholder=move_tr!("locations-parse-placeholder")
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
                    {move_tr!("locations-parse-add")}
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
                                        {move_tr!("locations-empty")}
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
            <div class="flex flex-col gap-2 ml-2">
                {group
                    .regions
                    .into_iter()
                    .map(|rg| {
                        view! {
                            <div>
                                {rg.region.map(|r| view! {
                                    <div class="text-xs font-semibold text-base-content/50 uppercase tracking-wide mb-1 ml-1">
                                        {r}
                                    </div>
                                })}
                                <div class="flex flex-col gap-1">
                                    {rg.cities.into_iter().map(|cg| {
                                        let city_id = cg.city.id.clone();
                                        let city_href = format!("/locations/{city_id}");
                                        let addresses = cg.addresses;
                                        view! {
                                            <LocationRow
                                                location=cg.city
                                                kind=LocationRowKind::City
                                                href=city_href
                                                set_refresh=set_refresh
                                            >
                                                {(!addresses.is_empty()).then(|| view! {
                                                    <div class="flex flex-col gap-1 mt-1 ml-4">
                                                        {addresses.into_iter().map(|addr| {
                                                            let href = format!("/locations/{city_id}/{}", addr.id);
                                                            view! {
                                                                <LocationRow
                                                                    location=addr
                                                                    kind=LocationRowKind::Address
                                                                    href=href
                                                                    set_refresh=set_refresh
                                                                />
                                                            }
                                                        }).collect_view()}
                                                    </div>
                                                })}
                                            </LocationRow>
                                        }
                                    }).collect_view()}
                                </div>
                            </div>
                        }
                    })
                    .collect_view()}
            </div>
        </div>
    }
}

// ── location row ──────────────────────────────────────────────────────────────

/// Which visual variant of [`LocationRow`] to render.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum LocationRowKind {
    /// Card wrapper, `Sm` inline editor, no inherited text size.
    City,
    /// Compact row, `Xs` inline editor, smaller text.
    Address,
}

impl LocationRowKind {
    const fn wrapper_class(self) -> &'static str {
        match self {
            Self::City => "card bg-base-200 p-2",
            Self::Address => "py-1 px-2 rounded bg-base-100",
        }
    }

    const fn name_class(self) -> &'static str {
        match self {
            Self::City => "font-medium hover:underline",
            Self::Address => "text-sm flex-1 hover:underline",
        }
    }

    const fn edit_size(self) -> InlineEditSize {
        match self {
            Self::City => InlineEditSize::Sm,
            Self::Address => InlineEditSize::Xs,
        }
    }
}

/// One row in the location tree: name link, alias badge, rename and delete actions.
///
/// `children` renders underneath the row (a city uses it for its addresses).
#[component]
fn LocationRow(
    location: Location,
    kind: LocationRowKind,
    href: String,
    set_refresh: WriteSignal<u32>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let i18n = expect_context::<I18n>();
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let id = StoredValue::new(location.id);
    let name = StoredValue::new(location.name);
    let alias = StoredValue::new(location.alias);
    let href = StoredValue::new(href);

    let (editing, set_editing) = signal(false);
    let show_delete = RwSignal::new(false);

    let on_save = Callback::new(move |new_name: String| {
        set_editing.set(false);
        refresh_or_toast(
            update_location_sf(id.get_value(), UpdateLocationRequest::rename(new_name)),
            set_refresh,
            toast.clone(),
        );
    });

    let on_delete = Callback::new(move |_: ()| {
        show_delete.set(false);
        refresh_or_toast(
            delete_location_sf(id.get_value()),
            set_refresh,
            toast.clone(),
        );
    });

    view! {
        <div class=kind.wrapper_class()>
            {move || {
                if editing.get() {
                    view! {
                        <InlineEdit
                            value=name.get_value()
                            on_save=on_save
                            on_cancel=Callback::new(move |_: ()| set_editing.set(false))
                            size=kind.edit_size()
                        />
                    }
                    .into_any()
                } else {
                    view! {
                        <div class="flex items-center gap-2 flex-wrap">
                            <A href=href.get_value() attr:class=kind.name_class()>
                                {name.get_value()}
                            </A>
                            <InlineAlias id=id.get_value() initial=alias.get_value() set_refresh />
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

            {children.map(|children| children())}

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

// ── inline alias ─────────────────────────────────────────────────────────────

#[component]
pub(super) fn InlineAlias(
    id: String,
    initial: Option<String>,
    set_refresh: WriteSignal<u32>,
) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let i18n = expect_context::<I18n>();
    let id_sv = StoredValue::new(id);
    let alias_sv = StoredValue::new(initial);
    let (editing, set_editing) = signal(false);
    let placeholder = i18n.tr("locations-alias-placeholder");

    let on_save = Callback::new(move |alias: String| {
        set_editing.set(false);
        let id = id_sv.get_value();
        let req = if alias.trim().is_empty() {
            UpdateLocationRequest::set_alias(None)
        } else {
            UpdateLocationRequest::set_alias(Some(alias))
        };
        refresh_or_toast(update_location_sf(id, req), set_refresh, toast.clone());
    });

    let on_clear = Callback::new(move |_: ()| {
        refresh_or_toast(
            update_location_sf(id_sv.get_value(), UpdateLocationRequest::set_alias(None)),
            set_refresh,
            toast.clone(),
        );
    });

    view! {
        {move || {
            if editing.get() {
                view! {
                    <InlineEdit
                        value=alias_sv.get_value().unwrap_or_default()
                        on_save=on_save
                        on_cancel=Callback::new(move |_: ()| set_editing.set(false))
                        size=InlineEditSize::Xs
                        placeholder=Some(placeholder.clone())
                        allow_empty=true
                    />
                }
                .into_any()
            } else {
                let alias = alias_sv.get_value();
                view! {
                    <AliasBadge
                        alias=alias
                        on_edit=Callback::new(move |_: ()| set_editing.set(true))
                        on_clear=on_clear
                    />
                }
                .into_any()
            }
        }}
    }
}

// ── alias badge ───────────────────────────────────────────────────────────────

#[component]
fn AliasBadge(
    alias: Option<String>,
    on_edit: Callback<()>,
    on_clear: Callback<()>,
) -> impl IntoView {
    match alias {
        Some(a) => view! {
            <span class="inline-flex items-center gap-1 badge badge-ghost badge-sm">
                <button
                    type="button"
                    class="hover:underline"
                    on:click=move |_| on_edit.run(())
                >
                    {a}
                </button>
                <button
                    type="button"
                    class="leading-none opacity-50 hover:opacity-100"
                    on:click=move |_| on_clear.run(())
                >
                    "x"
                </button>
            </span>
        }
        .into_any(),
        None => view! {
            <button
                type="button"
                class="btn btn-xs btn-ghost opacity-40 hover:opacity-100"
                on:click=move |_| on_edit.run(())
            >
                "+alias"
            </button>
        }
        .into_any(),
    }
}
