use leptos::prelude::*;
use leptos_fluent::leptos_fluent;
use leptos_router::components::{Route, Router, Routes};
use leptos_router::path;

#[cfg(target_arch = "wasm32")]
use crate::api::client::GlooClient;

use crate::api::SessionInfo;
use crate::components::nav::Nav;
use crate::components::sync_locale::SyncLocale;
use crate::components::toast_container::ToastContainer;
use crate::pages::{
    admin::AdminPage, calendar::CalendarPage, calendar::day::CalendarDayPage,
    container::ContainerPage, home::HomePage, item_detail::ItemDetailPage, list::ListPage,
    login::LoginPage, oauth_consent::OAuthConsentPage, settings::McpRedirect,
    settings::SettingsPage, signup::SignupPage, tags::TagsPage, tags::detail::TagDetailPage,
    today::TodayPage,
};
use crate::state::AdminContext;

pub type SessionResource = LocalResource<Option<SessionInfo>>;

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

    let admin_ctx = AdminContext::new();
    provide_context(admin_ctx);

    #[cfg(target_arch = "wasm32")]
    provide_context(GlooClient);

    let session_res: SessionResource = LocalResource::new(crate::api::get_session);
    provide_context(session_res);

    // Fetch /api/me after session resolves to populate is_admin signal
    #[cfg(target_arch = "wasm32")]
    {
        let client = GlooClient;
        Effect::new(move |_| {
            if let Some(Some(_)) = session_res.get() {
                let client = client.clone();
                leptos::task::spawn_local(async move {
                    if let Ok(me) = crate::api::admin::get_me(&client, None).await {
                        admin_ctx.is_admin.set(me.is_admin);
                    }
                });
            }
        });
    }

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
                        <Route path=path!("/lists/:list_id/items/:id") view=ItemDetailPage/>
                        <Route path=path!("/lists/:id") view=ListPage/>
                        <Route path=path!("/containers/:id") view=ContainerPage/>
                        <Route path=path!("/admin") view=AdminPage/>
                    </Routes>
                </main>
            </Router>
        </I18nProvider>
    }
}
