pub mod add_comment;
pub mod comment_list;

use leptos::prelude::*;

use crate::app::{ToastContext, ToastKind};
use crate::components::comments::add_comment::AddComment;
use crate::components::comments::comment_list::CommentList;
use crate::server_fns::comments::{get_comments, get_current_user_id, remove_comment};

/// Self-contained comments section: loads, displays, and adds comments for any entity.
#[component]
pub fn CommentSection(entity_type: &'static str, entity_id: Signal<String>) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let (refresh, set_refresh) = signal(0u32);

    let user_id_res = Resource::new(|| (), |_| get_current_user_id());

    let comments_res = Resource::new(
        move || (entity_id.get(), refresh.get()),
        move |(eid, _)| get_comments(entity_type.to_string(), eid),
    );

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

            <Suspense fallback=|| view! { <span class="loading loading-dots loading-xs"></span> }>
                {move || {
                    let comments = comments_res.get();
                    let user_id = user_id_res.get();
                    match (comments, user_id) {
                        (Some(Ok(comments)), Some(Ok(uid))) => view! {
                            <CommentList
                                comments=comments
                                current_user_id=uid
                                on_delete=on_delete
                            />
                        }.into_any(),
                        (Some(Err(e)), _) => view! {
                            <p class="text-error text-sm">"Błąd: " {e.to_string()}</p>
                        }.into_any(),
                        _ => view! {}.into_any(),
                    }
                }}
            </Suspense>

            <AddComment
                entity_type=Signal::derive(move || entity_type.to_string())
                entity_id=entity_id
                on_added=on_added
            />
        </div>
    }
}
