pub mod add_comment;
pub mod comment_list;

use leptos::prelude::*;
use leptos_fluent::move_tr;

use crate::app::{ToastContext, ToastKind};
use crate::components::comments::add_comment::AddComment;
use crate::components::comments::comment_list::CommentList;
use crate::server_fns::comments::{get_comments, remove_comment};
use kartoteka_shared::types::CommentsPayload;

#[component]
pub fn CommentSection(
    entity_type: &'static str,
    entity_id: Signal<String>,
    #[prop(optional)] initial_payload: Option<CommentsPayload>,
) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");

    // Initialized from pre-fetched data — avoids a nested Resource that breaks hydration.
    let payload = RwSignal::new(initial_payload);

    let on_added = Callback::new(move |_: ()| {
        let eid = entity_id.get_untracked();
        leptos::task::spawn_local(async move {
            match get_comments(entity_type.to_string(), eid).await {
                Ok(p) => payload.set(Some(p)),
                Err(e) => leptos::logging::warn!("CommentSection refresh failed: {e}"),
            }
        });
    });

    let on_delete = Callback::new(move |comment_id: String| {
        let eid = entity_id.get_untracked();
        leptos::task::spawn_local(async move {
            match remove_comment(comment_id).await {
                Ok(_) => match get_comments(entity_type.to_string(), eid).await {
                    Ok(p) => payload.set(Some(p)),
                    Err(e) => leptos::logging::warn!("CommentSection refresh failed: {e}"),
                },
                Err(e) => toast.push(e.to_string(), ToastKind::Error),
            }
        });
    });

    view! {
        <div class="mt-6">
            <h3 class="text-sm font-semibold text-base-content/60 uppercase tracking-wide mb-3">
                {move_tr!("comments-section-title")}
            </h3>

            {move || match payload.get() {
                Some(p) => view! {
                    <CommentList
                        comments=p.comments
                        current_user_id=p.current_user_id
                        on_delete=on_delete
                    />
                }.into_any(),
                None => view! {
                    <span class="loading loading-dots loading-xs"></span>
                }.into_any(),
            }}

            <AddComment
                entity_type=Signal::derive(move || entity_type.to_string())
                entity_id=entity_id
                on_added=on_added
            />
        </div>
    }
}
