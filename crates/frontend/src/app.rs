use leptos::prelude::*;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

use crate::components::nav::Nav;
use crate::pages::{home::HomePage, list::ListPage};

#[component]
pub fn App() -> impl IntoView {
    view! {
        <Router>
            <Nav/>
            <main class="container">
                <Routes fallback=|| view! { <p>"Nie znaleziono strony"</p> }>
                    <Route path=path!("/") view=HomePage/>
                    <Route path=path!("/lists/:id") view=ListPage/>
                </Routes>
            </main>
        </Router>
    }
}
