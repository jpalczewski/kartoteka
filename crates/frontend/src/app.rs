use leptos::prelude::*;
use leptos_fluent::leptos_fluent;
use leptos_router::{
    components::{Route, Router, Routes},
    path,
};

use crate::components::nav::Nav;
use crate::components::toast_container::ToastContainer;
use crate::pages::{
    all::AllPage,
    calendar::{CalendarPage, day::CalendarDayPage},
    container::ContainerPage,
    home::HomePage,
    item_detail::ItemDetailPage,
    list::ListPage,
    login::LoginPage,
    oauth_consent::OAuthConsentPage,
    settings::SettingsPage,
    signup::SignupPage,
    tags::{TagsPage, detail::TagDetailPage},
    time::TimePage,
    today::TodayPage,
};

// ── Toast context ──────────────────────────────────────────────────────────────

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

// ── i18n ───────────────────────────────────────────────────────────────────────

#[component]
fn I18nProvider(children: Children) -> impl IntoView {
    leptos_fluent! {
        children: children(),
        locales: "../../locales",
        default_language: "en",
        set_language_to_cookie: true,
        initial_language_from_cookie: true,
        initial_language_from_navigator: true,
        initial_language_from_accept_language_header: true,
        cookie_name: "lang",
    }
}

// ── App ────────────────────────────────────────────────────────────────────────

#[component]
pub fn App() -> impl IntoView {
    provide_context(ToastContext::new());
    provide_context(crate::context::GlobalRefresh::new());

    // Signal to e2e tests that WASM hydration is complete.
    // Deferred so the Router and route components finish attaching event handlers.
    #[cfg(target_arch = "wasm32")]
    set_timeout(
        || {
            if let Some(win) = web_sys::window() {
                if let Some(doc) = win.document() {
                    if let Some(body) = doc.body() {
                        body.set_attribute("data-hydrated", "true").ok();
                    }
                }
            }
        },
        std::time::Duration::ZERO,
    );

    view! {
        <I18nProvider>
            <Router>
                <Nav/>
                <ToastContainer/>
                <main class="container mx-auto px-4">
                    <Routes fallback=|| view! { <p class="p-4">"404 — Nie znaleziono strony"</p> }>
                        <Route path=path!("/") view=HomePage/>
                        <Route path=path!("/all") view=AllPage/>
                        <Route path=path!("/today") view=TodayPage/>
                        <Route path=path!("/time") view=TimePage/>
                        <Route path=path!("/login") view=LoginPage/>
                        <Route path=path!("/signup") view=SignupPage/>
                        <Route path=path!("/settings") view=SettingsPage/>
                        <Route path=path!("/consent") view=OAuthConsentPage/>
                        <Route path=path!("/calendar") view=CalendarPage/>
                        <Route path=path!("/calendar/:date") view=CalendarDayPage/>
                        <Route path=path!("/tags") view=TagsPage/>
                        <Route path=path!("/tags/:id") view=TagDetailPage/>
                        <Route path=path!("/lists/:list_id/items/:id") view=ItemDetailPage/>
                        <Route path=path!("/lists/:id") view=ListPage/>
                        <Route path=path!("/containers/:id") view=ContainerPage/>
                    </Routes>
                </main>
            </Router>
        </I18nProvider>
    }
}
