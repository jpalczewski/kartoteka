use leptos::prelude::*;
use leptos_fluent::leptos_fluent;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

use crate::components::nav::Nav;
use crate::components::sync_locale::SyncLocale;
use crate::components::toast_container::ToastContainer;
use crate::pages::{
    calendar::CalendarPage, calendar::day::CalendarDayPage, container::ContainerPage,
    home::HomePage, list::ListPage, login::LoginPage, oauth_consent::OAuthConsentPage,
    settings::McpRedirect, settings::SettingsPage, signup::SignupPage, tags::TagsPage,
    tags::detail::TagDetailPage, today::TodayPage,
};

#[derive(Clone, Debug, PartialEq)]
pub enum ToastKind {
    Success,
    Error,
}

#[derive(Clone, Debug)]
pub struct Toast {
    pub id: u32,
    pub message: String,
    pub kind: ToastKind,
}

#[derive(Clone, Copy)]
pub struct ToastContext {
    pub toasts: RwSignal<Vec<Toast>>,
    next_id: RwSignal<u32>,
}

impl ToastContext {
    pub fn new() -> Self {
        Self {
            toasts: RwSignal::new(Vec::new()),
            next_id: RwSignal::new(0),
        }
    }

    pub fn push(&self, message: String, kind: ToastKind) {
        let id = self.next_id.get();
        self.next_id.update(|n| *n += 1);
        self.toasts
            .update(|ts| ts.push(Toast { id, message, kind }));

        let toasts = self.toasts;
        set_timeout(
            move || toasts.update(|ts| ts.retain(|t| t.id != id)),
            std::time::Duration::from_millis(3000),
        );
    }

    pub fn dismiss(&self, id: u32) {
        self.toasts.update(|ts| ts.retain(|t| t.id != id));
    }
}

impl Default for ToastContext {
    fn default() -> Self {
        Self::new()
    }
}

#[component]
fn I18nProvider(children: Children) -> impl IntoView {
    leptos_fluent! {
        children: children(),
        locales: "../../locales",
        default_language: "en",
        initial_language_from_local_storage: true,
        initial_language_from_navigator: true,
        initial_language_from_navigator_to_local_storage: true,
        set_language_to_local_storage: true,
        local_storage_key: "lang",
    }
}

#[component]
pub fn App() -> impl IntoView {
    let toast_ctx = ToastContext::new();
    provide_context(toast_ctx);

    view! {
        <I18nProvider>
            <SyncLocale/>
            <Router>
                <Nav/>
                <ToastContainer/>
                <main class="container">
                    <Routes fallback=|| view! { <p>"Nie znaleziono strony"</p> }>
                        <Route path=path!("/") view=HomePage/>
                        <Route path=path!("/today") view=TodayPage/>
                        <Route path=path!("/login") view=LoginPage/>
                        <Route path=path!("/signup") view=SignupPage/>
                        <Route path=path!("/settings") view=SettingsPage/>
                        <Route path=path!("/mcp") view=McpRedirect/>
                        <Route path=path!("/oauth/consent") view=OAuthConsentPage/>
                        <Route path=path!("/calendar") view=CalendarPage/>
                        <Route path=path!("/calendar/:date") view=CalendarDayPage/>
                        <Route path=path!("/tags") view=TagsPage/>
                        <Route path=path!("/tags/:id") view=TagDetailPage/>
                        <Route path=path!("/lists/:id") view=ListPage/>
                        <Route path=path!("/containers/:id") view=ContainerPage/>
                    </Routes>
                </main>
            </Router>
        </I18nProvider>
    }
}
