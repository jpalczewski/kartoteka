pub mod add_comment;
pub mod comment_list;

use leptos::prelude::*;
use leptos_fluent::move_tr;

use crate::app::{ToastContext, ToastKind};
use crate::components::comments::add_comment::AddComment;
use crate::components::comments::comment_list::CommentList;
use crate::server_fns::comments::{get_comments, remove_comment};

#[component]
pub fn CommentSection(entity_type: &'static str, entity_id: Signal<String>) -> impl IntoView {
    let toast = use_context::<ToastContext>().expect("ToastContext missing");
    let (refresh, set_refresh) = signal(0u32);

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
                {move_tr!("comments-section-title")}
            </h3>

            <Suspense fallback=|| view! { <span class="loading loading-dots loading-xs"></span> }>
                {move || match comments_res.get() {
                    Some(Ok(payload)) => view! {
                        <CommentList
                            comments=payload.comments
                            current_user_id=payload.current_user_id
                            on_delete=on_delete
                        />
                    }.into_any(),
                    Some(Err(e)) => view! {
                        <p class="text-error text-sm">{move_tr!("error-prefix")} " " {e.to_string()}</p>
                    }.into_any(),
                    None => view! {}.into_any(),
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
