use leptos::prelude::*;
use leptos_fluent::{I18n, move_tr};
use leptos_router::hooks::use_params_map;

use crate::components::common::breadcrumbs::Breadcrumbs;
use crate::components::common::loading::LoadingSpinner;
use crate::server_fns::locations::get_location_detail_sf;
use crate::state::invalidation::use_invalidation;
use kartoteka_shared::ChangeTopic;

use super::detail_header::LocationDetailHeader;

#[component]
pub fn AddressDetailPage() -> impl IntoView {
    let params = use_params_map();
    let city_id = Memo::new(move |_| params.read().get("city_id").unwrap_or_default());
    let addr_id = Memo::new(move |_| params.read().get("id").unwrap_or_default());
    let i18n = expect_context::<I18n>();
    let invalidation = use_invalidation();

    let addr_res = Resource::new(
        move || (addr_id.get(), invalidation.track(&[ChangeTopic::Locations])),
        |(id, _)| get_location_detail_sf(id),
    );
    let city_res = Resource::new(move || city_id.get(), get_location_detail_sf);

    view! {
        <div class="container mx-auto max-w-2xl p-4">
            // Breadcrumbs need both the address and its city, so they render inside their
            // own boundary rather than showing an incomplete trail while loading.
            <Suspense fallback=|| view! {}>
                {move || {
                    let addr = addr_res.get()?.ok().flatten()?;
                    let mut crumbs =
                        vec![("/locations".to_string(), i18n.tr("locations-title"))];
                    if let Some(city) = city_res.get().and_then(|r| r.ok()).flatten() {
                        crumbs.push((
                            format!("/locations/{}", city.id),
                            city.alias.clone().unwrap_or(city.name.clone()),
                        ));
                    }
                    Some(view! { <Breadcrumbs crumbs=crumbs current=addr.name.clone()/> })
                }}
            </Suspense>

            <Suspense fallback=|| view! { <LoadingSpinner/> }>
                {move || addr_res.get().map(|result| {
                    match result {
                        Err(e) => view! { <p class="text-error">{e.to_string()}</p> }.into_any(),
                        Ok(None) => view! { <p class="text-base-content/50">{move_tr!("locations-address-not-found")}</p> }.into_any(),
                        Ok(Some(addr)) => {
                            let after_delete_href = format!("/locations/{}", city_id.get());

                            view! {
                                <LocationDetailHeader
                                    id=addr.id
                                    name=addr.name
                                    alias=addr.alias
                                    after_delete_href=after_delete_href
                                />
                            }.into_any()
                        }
                    }
                })}
            </Suspense>
        </div>
    }
}
