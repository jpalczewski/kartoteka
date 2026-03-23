use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

use crate::components::nav::Nav;
use crate::pages::{home::HomePage, list::ListPage, login::LoginPage, settings::SettingsPage, tags::TagsPage};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Nav/>
            <main class="container">
                <Routes fallback=|| view! { <p>"Nie znaleziono strony"</p> }>
                    <Route path=path!("/") view=HomePage/>
                    <Route path=path!("/login") view=LoginPage/>
                    <Route path=path!("/settings") view=SettingsPage/>
                    <Route path=path!("/tags") view=TagsPage/>
                    <Route path=path!("/lists/:id") view=ListPage/>
                </Routes>
            </main>
        </Router>
    }
}
