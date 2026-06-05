pub mod add_comment;
pub mod comment_list;

use kartoteka_shared::types::Comment;
use leptos::prelude::*;

use crate::app::{ToastContext, ToastKind};
use crate::components::comments::add_comment::AddComment;
use crate::components::comments::comment_list::CommentList;
use crate::server_fns::comments::{get_comments, get_current_user_id, remove_comment};

/// Self-contained comments section: loads, displays, and adds comments for any entity.
/// Uses spawn_local + signals (not Resource) so SSR renders an empty marker that the
/// client hydrates cleanly, then fetches data after mount.
#[component]
pub fn CommentSection(entity_type: &'static str, entity_id: Signal<String>) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let (refresh, set_refresh) = signal(0u32);

    // None = not yet loaded; Some((comments, uid)) = loaded.
    // SSR never sets this (Effect is client-only), so SSR and initial client both start at None
    // → consistent hydration, no marker/element mismatch.
    let (loaded, set_loaded) = signal(None::<(Vec<Comment>, String)>);
    let (fetch_error, set_fetch_error) = signal(None::<String>);

    Effect::new(move |_| {
        let eid = entity_id.get();
        let _ = refresh.get();
        set_fetch_error.set(None);
        leptos::task::spawn_local(async move {
            let uid = match get_current_user_id().await {
                Ok(u) => u,
                Err(e) => {
                    set_fetch_error.set(Some(e.to_string()));
                    return;
                }
            };
            match get_comments(entity_type.to_string(), eid).await {
                Ok(cs) => set_loaded.set(Some((cs, uid))),
                Err(e) => set_fetch_error.set(Some(e.to_string())),
            }
        });
    });

    let on_added = Callback::new(move |_: ()| set_refresh.update(|n| *n += 1));

    let on_delete = Callback::new(move |comment_id: String| {
        leptos::task::spawn_local(async move {
            match remove_comment(comment_id).await {
                Ok(_) => set_refresh.update(|n| *n += 1),
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    view! {
        <div class="mt-6">
            <h3 class="text-sm font-semibold text-base-content/60 uppercase tracking-wide mb-3">
                "Komentarze"
            </h3>

            {move || match (fetch_error.get(), loaded.get()) {
                (Some(e), _) => view! {
                    <p class="text-error text-sm">"Błąd: " {e}</p>
                }.into_any(),
                (_, Some((cs, uid))) => view! {
                    <CommentList comments=cs current_user_id=uid on_delete=on_delete />
                }.into_any(),
                _ => view! {}.into_any(),
            }}

            <AddComment
                entity_type=Signal::derive(move || entity_type.to_string())
                entity_id=entity_id
                on_added=on_added
            />
        </div>
    }
}
