use leptos::prelude::*;

#[component]
pub fn Breadcrumbs(crumbs: Vec<(String, String)>) -> impl IntoView {
    view! {
        <div class="breadcrumbs text-sm mb-4">
            <ul>
                <li><a href="/">"Home"</a></li>
                {crumbs.into_iter().map(|(label, href)| {
                    view! {
                        <li><a href=href>{label}</a></li>
                    }
                }).collect::<Vec<_>>()}
            </ul>
        </div>
    }
}
