use leptos::prelude::*;
use leptos_fluent::move_tr;
use leptos_router::components::A;

/// DaisyUI breadcrumb trail. `crumbs` are all ancestor links; `current` is the current page (no link).
#[component]
pub fn Breadcrumbs(crumbs: Vec<(String, String)>, current: String) -> impl IntoView {
    if crumbs.is_empty() {
        return view! {}.into_any();
    }

    view! {
        <div class="text-sm breadcrumbs mb-2">
            <ul>
                <li><A href="/">{move_tr!("common-home")}</A></li>
                {crumbs.into_iter().map(|(href, label)| {
                    view! {
                        <li><A href=href>{label}</A></li>
                    }
                }).collect_view()}
                <li>{current}</li>
            </ul>
        </div>
    }
    .into_any()
}
